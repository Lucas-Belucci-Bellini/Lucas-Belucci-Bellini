//! Atividade do ecossistema: o que o monitor de commits observa.
//!
//! Uma varredura é uma linha em `ecosystem.sync_runs` (`kind = 'commits'`).
//! Um repositório só ganha linha em `ecosystem.commit_observations` quando o
//! estado dele **muda** — SHA, erro ou vazio (D-020): 67 repositórios
//! varridos por hora sem mudança não geram 67 linhas. Os contadores do monitor
//! entram em `ecosystem.metric_samples` só quando o snapshot é publicado,
//! que é quando eles mudam.

use sqlx::Connection;
use sqlx::postgres::PgConnection;

use crate::{Database, RunContext, StoreError, close_run, open_run, record_failed_run};

/// Estado do branch padrão de um repositório.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadState {
    /// Último commit.
    Commit {
        /// SHA de 40 dígitos hexadecimais.
        sha: String,
        /// Data do commit (ISO 8601), quando legível.
        committed_at: Option<String>,
        /// Primeira linha da mensagem, até 140 caracteres.
        message: String,
        /// Página do commit.
        url: Option<String>,
    },
    /// Repositório sem commits.
    Empty,
    /// A consulta falhou.
    Error {
        /// Etapa (`latest_commit`).
        stage: String,
        /// Texto do erro, como o monitor Python o grava.
        message: String,
    },
}

/// Uma transição observada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadObservation {
    /// Nome curto do repositório (o monitor indexa por nome).
    pub name: String,
    /// Branch consultado.
    pub branch: String,
    /// Estado novo.
    pub state: HeadState,
    /// Commits desde a observação anterior; `None` = não determinado.
    pub commits_since_previous: Option<i64>,
}

/// Contadores do monitor depois de um snapshot publicado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorCounters {
    /// `ecosystem.commits.project_total`.
    pub project: i64,
    /// `ecosystem.commits.monitor_total`.
    pub monitor: i64,
    /// `ecosystem.commits.tracked_total`.
    pub tracked: i64,
}

/// O que uma varredura grava.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitScanRecord {
    /// Dono dos repositórios (o `full_name` é `dono/nome`).
    pub owner: String,
    /// Repositórios varridos.
    pub scanned: usize,
    /// Só os que mudaram de estado.
    pub observations: Vec<HeadObservation>,
    /// Presentes quando o snapshot foi publicado.
    pub counters: Option<MonitorCounters>,
}

/// Resultado da gravação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitScanReport {
    /// Execução criada.
    pub sync_run_id: i64,
    /// Observações gravadas.
    pub recorded: usize,
    /// Repositórios que não estão em `ecosystem.repositories` (rode `sync github`).
    pub unregistered: Vec<String>,
}

impl Database {
    /// Grava uma varredura do monitor numa transação só. Se algo falha, nada
    /// entra e a execução fica registrada como `failed`, com o motivo.
    pub async fn record_commit_scan(
        &self,
        run: &RunContext,
        scan: &CommitScanRecord,
    ) -> Result<CommitScanReport, StoreError> {
        let mut conn = self.connect(false).await?;
        match record_scan_on(&mut conn, run, scan).await {
            Ok(report) => Ok(report),
            Err(error) => {
                record_failed_run(&mut conn, "commits", run, scan.scanned, &error).await;
                Err(error)
            }
        }
    }
}

async fn record_scan_on(
    conn: &mut PgConnection,
    run: &RunContext,
    scan: &CommitScanRecord,
) -> Result<CommitScanReport, StoreError> {
    let mut tx = conn.begin().await?;
    let sync_run_id = open_run(&mut tx, "commits", run).await?;

    let mut recorded = 0;
    let mut unregistered = Vec::new();
    for observation in &scan.observations {
        let full_name = format!("{}/{}", scan.owner, observation.name);
        let repository_id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM ecosystem.repositories WHERE full_name_key = lower($1) AND gone_at IS NULL",
        )
        .bind(&full_name)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(repository_id) = repository_id else {
            unregistered.push(full_name);
            continue;
        };
        let (sha, committed_at, message, url, empty, stage, error) = match &observation.state {
            HeadState::Commit { sha, committed_at, message, url } => {
                (Some(sha), committed_at.as_ref(), Some(message), url.as_ref(), false, None, None)
            }
            HeadState::Empty => (None, None, None, None, true, None, None),
            HeadState::Error { stage, message } => (None, None, None, None, false, Some(stage), Some(message)),
        };
        sqlx::query(
            "INSERT INTO ecosystem.commit_observations \
               (repository_id, sync_run_id, branch, head_sha, head_committed_at, head_message, head_url, \
                commits_since_previous, is_empty, error_stage, error_message) \
             VALUES ($1, $2, $3, $4, $5::timestamptz, $6, $7, $8, $9, $10, $11)",
        )
        .bind(repository_id)
        .bind(sync_run_id)
        .bind(&observation.branch)
        .bind(sha)
        .bind(committed_at)
        .bind(message)
        .bind(url)
        // Negativo não existe no GitHub; se vier, é "não determinado".
        .bind(observation.commits_since_previous.filter(|n| *n >= 0).and_then(|n| i32::try_from(n).ok()))
        .bind(empty)
        .bind(stage)
        .bind(error)
        .execute(&mut *tx)
        .await?;
        recorded += 1;
    }

    if let Some(counters) = scan.counters {
        for (key, value) in [
            ("ecosystem.commits.project_total", counters.project),
            ("ecosystem.commits.monitor_total", counters.monitor),
            ("ecosystem.commits.tracked_total", counters.tracked),
        ] {
            sqlx::query(
                "INSERT INTO ecosystem.metric_samples (metric_key, value, sync_run_id, provenance) \
                 VALUES ($1, $2::numeric, $3, 'collected')",
            )
            .bind(key)
            .bind(value)
            .bind(sync_run_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    close_run(&mut tx, sync_run_id, scan.scanned, recorded).await?;
    tx.commit().await?;
    Ok(CommitScanReport { sync_run_id, recorded, unregistered })
}
