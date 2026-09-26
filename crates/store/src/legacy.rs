//! Carga inicial do estado que hoje vive em JSON versionado
//! (docs/database/MIGRATIONS.md §6), num `sync_run` do tipo `legacy_import`.
//!
//! Pré-requisito: `sync github` (repositórios e projetos) e `import manifests`
//! (passos 1–3 do plano). Aqui entram os passos que só existem no JSON:
//!
//! | passo | origem | destino |
//! |:--|:---|:---|
//! | 3b | `FEATURED_SUMMARIES` e os nomes fixos de `status_for()` | `projects.summary`, `projects.lifecycle_override` |
//! | 4 | `docs/project-catalog.json` | `websites` + uma checagem inicial |
//! | 5 | `ECOSYSTEM-COMMIT-STATE.json` → `repositories` | `commit_observations` |
//! | 6 | `ECOSYSTEM-COMMIT-STATE.json` → `metrics` | `metric_samples` (`legacy_import`) |
//! | 7 | `contributions-timeline-data.json` | `metric_samples` (`legacy_import`) |
//!
//! **Idempotente:** cada passo só grava o que ainda não existe (resumo e
//! sobreposição vazios, site sem checagem, repositório sem observação,
//! métrica sem amostra importada). Rodar de novo não duplica nada, e nada que
//! a coleta já gravou é sobrescrito.

use sqlx::Connection;
use sqlx::postgres::PgConnection;

use crate::activity::HeadObservation;
use crate::editorial::project_of;
use crate::metrics::Sample;
use crate::{Database, RunContext, StoreError, close_run, open_run, record_failed_run, refresh_primary};

/// Um site declarado no catálogo legado, com o último resultado conhecido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogSite {
    /// `owner/nome`.
    pub repository: String,
    /// `website_declared`.
    pub url: String,
    /// `github_homepage` | `manifest`.
    pub source: String,
    /// `verified` | `unreachable` | `invalid`.
    pub outcome: String,
    /// `website_http_status` (0 vira ausente).
    pub http_status: Option<i16>,
    /// Destino (`website_final_url` ou a própria URL).
    pub final_url: Option<String>,
}

/// O que a carga legada grava.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyRecord {
    /// Resumos editoriais por nome de repositório.
    pub summaries: Vec<(String, String)>,
    /// Nomes com `lifecycle_override = 'in_development'` fixo no código.
    pub in_development: Vec<String>,
    /// Prefixo com a mesma sobreposição (`baluarte-`), e a exceção dele.
    pub in_development_prefix: Option<(String, String)>,
    /// Sites do catálogo.
    pub sites: Vec<CatalogSite>,
    /// Dono dos repositórios do monitor.
    pub owner: String,
    /// Estado de cada repositório no monitor.
    pub heads: Vec<HeadObservation>,
    /// Contadores do monitor e contribuições, como amostras importadas.
    pub samples: Vec<Sample>,
}

/// Resultado, passo a passo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyReport {
    /// Execução criada.
    pub sync_run_id: i64,
    /// Resumos gravados.
    pub summaries: usize,
    /// Sobreposições gravadas.
    pub overrides: usize,
    /// Sites criados.
    pub sites_created: usize,
    /// Checagens iniciais gravadas.
    pub checks: usize,
    /// Observações de commit gravadas.
    pub heads: usize,
    /// Amostras gravadas.
    pub samples: usize,
    /// O que não casou com o banco.
    pub skipped: Vec<String>,
}

impl Database {
    /// Faz a carga legada numa transação só.
    pub async fn import_legacy(&self, run: &RunContext, record: &LegacyRecord) -> Result<LegacyReport, StoreError> {
        let mut conn = self.connect(false).await?;
        match import_on(&mut conn, run, record).await {
            Ok(report) => Ok(report),
            Err(error) => {
                record_failed_run(&mut conn, "legacy_import", run, 0, &error).await;
                Err(error)
            }
        }
    }
}

async fn import_on(
    conn: &mut PgConnection,
    run: &RunContext,
    record: &LegacyRecord,
) -> Result<LegacyReport, StoreError> {
    let mut tx = conn.begin().await?;
    let sync_run_id = open_run(&mut tx, "legacy_import", run).await?;
    let mut report = LegacyReport { sync_run_id, ..LegacyReport::default() };

    // 3b. Constantes do código: só onde o banco ainda não tem valor.
    for (name, summary) in &record.summaries {
        report.summaries += sqlx::query(
            "UPDATE ecosystem.projects p SET summary = $2 FROM ecosystem.project_repositories pr \
             JOIN ecosystem.repositories r ON r.id = pr.repository_id \
             WHERE pr.project_id = p.id AND pr.role = 'primary' AND r.name = $1 AND r.gone_at IS NULL \
               AND p.summary IS NULL",
        )
        .bind(name)
        .bind(summary)
        .execute(&mut *tx)
        .await?
        .rows_affected() as usize;
    }
    let (prefix, exception) = record.in_development_prefix.clone().unwrap_or_default();
    report.overrides = sqlx::query(
        "UPDATE ecosystem.projects p SET lifecycle_override = 'in_development' FROM ecosystem.project_repositories pr \
         JOIN ecosystem.repositories r ON r.id = pr.repository_id \
         WHERE pr.project_id = p.id AND pr.role = 'primary' AND r.gone_at IS NULL AND p.lifecycle_override IS NULL \
           AND (r.name = ANY($1) OR ($2 <> '' AND starts_with(r.name, $2) AND r.name <> $3))",
    )
    .bind(&record.in_development)
    .bind(&prefix)
    .bind(&exception)
    .execute(&mut *tx)
    .await?
    .rows_affected() as usize;

    // 4. Sites do catálogo e a última checagem conhecida.
    for site in &record.sites {
        let Some((project_id, private)) = project_of(&mut tx, &site.repository).await? else {
            report.skipped.push(format!("site de {} (fora do banco)", site.repository));
            continue;
        };
        if private {
            report.skipped.push(format!("site de {} (privado)", site.repository));
            continue;
        }
        let existing: Option<i64> =
            sqlx::query_scalar("SELECT id FROM ecosystem.websites WHERE project_id = $1 AND url = $2")
                .bind(project_id)
                .bind(&site.url)
                .fetch_optional(&mut *tx)
                .await?;
        let website_id = match existing {
            Some(id) => id,
            None => {
                report.sites_created += 1;
                sqlx::query_scalar(
                    "INSERT INTO ecosystem.websites (project_id, url, source, is_primary) VALUES ($1, $2, $3, false) \
                     RETURNING id",
                )
                .bind(project_id)
                .bind(&site.url)
                .bind(&site.source)
                .fetch_one(&mut *tx)
                .await?
            }
        };
        refresh_primary(&mut tx, project_id).await?;
        let has_checks: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ecosystem.website_checks WHERE website_id = $1)")
                .bind(website_id)
                .fetch_one(&mut *tx)
                .await?;
        if has_checks {
            continue;
        }
        let error_kind = match site.outcome.as_str() {
            "verified" => None,
            "invalid" => Some("invalid_url"),
            _ if site.http_status.is_some() => Some("http_status"),
            _ => Some("other"),
        };
        sqlx::query(
            "INSERT INTO ecosystem.website_checks \
               (website_id, sync_run_id, outcome, http_status, final_url, error_kind, error_message) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(website_id)
        .bind(sync_run_id)
        .bind(&site.outcome)
        .bind(site.http_status)
        .bind(&site.final_url)
        .bind(error_kind)
        .bind(error_kind.map(|_| "importado de docs/project-catalog.json"))
        .execute(&mut *tx)
        .await?;
        report.checks += 1;
    }

    // 5. Estado do monitor: uma observação por repositório que ainda não tem nenhuma.
    for head in &record.heads {
        let full_name = format!("{}/{}", record.owner, head.name);
        let repository_id: Option<(i64, bool)> = sqlx::query_as(
            "SELECT r.id, EXISTS (SELECT 1 FROM ecosystem.commit_observations o WHERE o.repository_id = r.id) \
             FROM ecosystem.repositories r WHERE r.full_name_key = lower($1) AND r.gone_at IS NULL",
        )
        .bind(&full_name)
        .fetch_optional(&mut *tx)
        .await?;
        match repository_id {
            None => report.skipped.push(format!("estado do monitor de {full_name} (fora do banco)")),
            Some((_, true)) => {}
            Some((repository_id, false)) => {
                crate::activity::insert_observation(&mut tx, repository_id, sync_run_id, head).await?;
                report.heads += 1;
            }
        }
    }

    // 6 e 7. Amostras importadas: só as que ainda não foram importadas.
    for sample in &record.samples {
        let inserted = sqlx::query(
            "INSERT INTO ecosystem.metric_samples \
               (metric_key, value, window_start, window_end, sync_run_id, provenance) \
             SELECT $1, $2::numeric, $3::timestamptz, $4::timestamptz, $5, 'legacy_import' \
             WHERE NOT EXISTS (SELECT 1 FROM ecosystem.metric_samples s WHERE s.metric_key = $1 \
               AND s.provenance = 'legacy_import' AND s.dimension = '' \
               AND s.window_start IS NOT DISTINCT FROM $3::timestamptz \
               AND s.window_end IS NOT DISTINCT FROM $4::timestamptz)",
        )
        .bind(&sample.metric_key)
        .bind(&sample.value)
        .bind(&sample.window_start)
        .bind(&sample.window_end)
        .bind(sync_run_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        report.samples += inserted as usize;
    }

    let changed =
        report.summaries + report.overrides + report.sites_created + report.checks + report.heads + report.samples;
    let seen = record.summaries.len() + record.sites.len() + record.heads.len() + record.samples.len();
    close_run(&mut tx, sync_run_id, seen, changed).await?;
    tx.commit().await?;
    Ok(report)
}
