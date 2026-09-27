//! Séries de métricas do perfil (`ecosystem.metric_samples`).
//!
//! Uma amostra só entra quando o valor daquela série (métrica × janela)
//! muda em relação à mais recente: a coleta diária de contribuições consulta
//! 13 janelas × 7 métricas, e as janelas fechadas quase nunca mudam — gravar
//! todas as vezes seria duplicar o GitHub sem informação nova.

use sqlx::Connection;
use sqlx::postgres::PgConnection;

use crate::{Database, RunContext, StoreError, close_run, open_run, record_failed_run};

/// Uma amostra de métrica sem sujeito (escopo `profile` ou `ecosystem`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    /// Chave em `metric_definitions`.
    pub metric_key: String,
    /// Valor, como texto numérico.
    pub value: String,
    /// Início da janela (ISO 8601).
    pub window_start: Option<String>,
    /// Fim da janela.
    pub window_end: Option<String>,
}

/// Resultado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplesReport {
    /// Execução criada.
    pub sync_run_id: i64,
    /// Amostras gravadas (as que mudaram).
    pub recorded: usize,
}

impl Database {
    /// Grava, numa execução do tipo `kind`, só as amostras cujo valor mudou.
    pub async fn record_samples(
        &self,
        kind: &str,
        run: &RunContext,
        samples: &[Sample],
    ) -> Result<SamplesReport, StoreError> {
        let mut conn = self.connect(false).await?;
        match record_on(&mut conn, kind, run, samples).await {
            Ok(report) => Ok(report),
            Err(error) => {
                record_failed_run(&mut conn, kind, run, samples.len(), &error).await;
                Err(error)
            }
        }
    }
}

async fn record_on(
    conn: &mut PgConnection,
    kind: &str,
    run: &RunContext,
    samples: &[Sample],
) -> Result<SamplesReport, StoreError> {
    let mut tx = conn.begin().await?;
    let sync_run_id = open_run(&mut tx, kind, run).await?;
    let mut recorded = 0;
    for sample in samples {
        let unchanged: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM ecosystem.metric_latest \
               WHERE metric_key = $1 AND repository_id IS NULL AND project_id IS NULL AND dimension = '' \
                 AND window_start IS NOT DISTINCT FROM $2::timestamptz \
                 AND window_end IS NOT DISTINCT FROM $3::timestamptz \
                 AND value = $4::numeric)",
        )
        .bind(&sample.metric_key)
        .bind(&sample.window_start)
        .bind(&sample.window_end)
        .bind(&sample.value)
        .fetch_one(&mut *tx)
        .await?;
        if unchanged {
            continue;
        }
        sqlx::query(
            "INSERT INTO ecosystem.metric_samples (metric_key, value, window_start, window_end, sync_run_id, provenance) \
             VALUES ($1, $2::numeric, $3::timestamptz, $4::timestamptz, $5, 'collected')",
        )
        .bind(&sample.metric_key)
        .bind(&sample.value)
        .bind(&sample.window_start)
        .bind(&sample.window_end)
        .bind(sync_run_id)
        .execute(&mut *tx)
        .await?;
        recorded += 1;
    }
    close_run(&mut tx, sync_run_id, samples.len(), recorded).await?;
    tx.commit().await?;
    Ok(SamplesReport { sync_run_id, recorded })
}
