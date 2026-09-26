//! PostgreSQL do ecossistema.
//!
//! Aplica, reverte e relata as migrations de `db/migrations`, **embutidas no
//! binário** em tempo de compilação, e grava o que os coletores observam:
//! checagens de site (`record_website_checks`), varreduras do monitor de
//! commits ([`activity`]) e o inventário do GitHub.
//!
//! # Garantias
//!
//! * **Mesma tabela de controle do `sqlx-cli`** (`_sqlx_migrations`, mesmos
//!   checksums): um banco migrado por um é reconhecido pelo outro.
//! * **Tudo ou nada.** `migrate` e `revert` rodam dentro de uma transação
//!   externa (o sqlx usa um *savepoint* por migration): se a terceira de cinco
//!   falha, o banco fica exatamente como estava antes do comando. A exceção
//!   seria uma migration marcada `-- no-transaction` — não há nenhuma; se
//!   aparecer, o comando cai para uma transação por migration.
//! * **Perda de dado só com opt-in.** Toda `down` que apaga tabela chama
//!   `ecosystem.assert_data_loss_allowed()` (migration 0001). `revert` só liga
//!   `ecosystem.allow_data_loss=on` — e só na conexão dedicada daquele
//!   comando — quando o chamador pede explicitamente.
//! * **`status` não escreve nada**: nem a tabela de controle ele cria.
//! * Migration aplicada e depois **alterada**, migration no banco que **este
//!   binário não conhece** e migration **pela metade** são recusadas antes de
//!   qualquer mudança, com o número da versão.

use std::collections::BTreeMap;
use std::str::FromStr;

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::postgres::{PgConnectOptions, PgConnection};
use sqlx::{Connection, Postgres, Transaction};

pub mod activity;
pub mod inventory;

/// As migrations de `db/migrations`, embutidas em tempo de compilação.
pub static MIGRATOR: Migrator = sqlx::migrate!("../../db/migrations");

/// Versão mínima do servidor (`server_version_num`): `UNIQUE NULLS NOT
/// DISTINCT`, usado na migration 0002, é do PostgreSQL 15.
pub const MIN_SERVER_VERSION: i32 = 150_000;

/// Início da mensagem de `ecosystem.assert_data_loss_allowed()` (0001).
const DATA_LOSS_MESSAGE: &str = "down migration would destroy data";

/// Falhas do acesso ao banco.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// A URL não é uma URL `postgres://` válida.
    #[error("DATABASE_URL inválida: {0}")]
    InvalidUrl(String),
    /// Não foi possível abrir a conexão.
    #[error("não foi possível conectar ao PostgreSQL: {0}")]
    Connect(#[source] sqlx::Error),
    /// Servidor mais antigo que o mínimo.
    #[error("PostgreSQL {found} não é suportado: o schema exige 15 ou mais novo")]
    UnsupportedServer {
        /// `server_version_num` do servidor.
        found: i32,
    },
    /// O banco tem uma migration que este binário não conhece.
    #[error(
        "o banco tem a migration {0}, que este binário não conhece: o banco está à frente do código; \
         atualize o profile-core antes de mexer no schema"
    )]
    UnknownApplied(i64),
    /// Uma migration aplicada foi editada depois.
    #[error(
        "a migration {0} foi alterada depois de aplicada (checksum diferente): migration aplicada não se edita; \
         a correção é uma migration nova"
    )]
    Modified(i64),
    /// Uma migration ficou registrada como falha.
    #[error(
        "a migration {0} ficou pela metade (success = false em _sqlx_migrations); corrija à mão antes de continuar"
    )]
    Dirty(i64),
    /// A `down` recusou apagar dados.
    #[error(
        "reverter a migration {version} apagaria dados ({detail}); nada foi alterado. \
         Exporte os dados e repita com a permissão explícita de perda de dados"
    )]
    DataLossRefused {
        /// Migration cuja `down` recusou.
        version: i64,
        /// Mensagem do banco.
        detail: String,
    },
    /// Versão-alvo que não existe.
    #[error("não existe migration {0} para servir de alvo (use 0 para reverter todas)")]
    UnknownTarget(i64),
    /// Outra falha do mecanismo de migrations.
    #[error(transparent)]
    Migrate(MigrateError),
    /// Outra falha de banco.
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Situação de uma migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationState {
    /// Aplicada, com o mesmo conteúdo embutido neste binário.
    Applied,
    /// Ainda não aplicada.
    Pending,
    /// Aplicada com outro conteúdo (checksum diferente).
    Modified,
    /// Registrada como falha.
    Dirty,
    /// Aplicada no banco, desconhecida deste binário.
    Unknown,
}

impl MigrationState {
    /// Rótulo curto para relatórios.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Pending => "pending",
            Self::Modified => "modified",
            Self::Dirty => "dirty",
            Self::Unknown => "unknown",
        }
    }
}

/// Uma linha do relatório de migrations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationStatus {
    /// Versão (`0003` → 3).
    pub version: i64,
    /// Descrição tirada do nome do arquivo.
    pub description: String,
    /// Situação no banco.
    pub state: MigrationState,
    /// Quando foi aplicada (UTC, ISO 8601), se foi.
    pub installed_on: Option<String>,
}

/// Até onde reverter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevertTarget {
    /// Só a última aplicada (o padrão do `sqlx migrate revert`).
    Last,
    /// Tudo acima desta versão; `0` reverte todas.
    Version(i64),
}

/// Um banco do ecossistema, ainda sem conexão aberta.
#[derive(Debug, Clone)]
pub struct Database {
    options: PgConnectOptions,
}

impl Database {
    /// A partir de uma URL `postgres://…` (ou `postgresql://…`). A mensagem de
    /// erro nunca repete a URL, que pode conter senha.
    pub fn from_url(url: &str) -> Result<Self, StoreError> {
        if !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
            return Err(StoreError::InvalidUrl("o esquema precisa ser postgres:// ou postgresql://".into()));
        }
        PgConnectOptions::from_str(url)
            .map(Self::from_options)
            .map_err(|error| StoreError::InvalidUrl(error.to_string().replace(url, "<DATABASE_URL>")))
    }

    /// A partir de opções já montadas.
    pub fn from_options(options: PgConnectOptions) -> Self {
        Self { options }
    }

    /// Situação de cada migration embutida e de cada migration do banco.
    /// Não escreve nada.
    pub async fn status(&self) -> Result<Vec<MigrationStatus>, StoreError> {
        let mut conn = self.connect(false).await?;
        status_on(&mut conn).await
    }

    /// Aplica as migrations pendentes, todas ou nenhuma. Devolve as que
    /// foram aplicadas agora.
    pub async fn migrate(&self) -> Result<Vec<MigrationStatus>, StoreError> {
        let mut conn = self.connect(false).await?;
        let before = status_on(&mut conn).await?;
        ensure_consistent(&before)?;

        if let Some(mut tx) = begin_if_atomic(&mut conn).await? {
            MIGRATOR.run(&mut *tx).await.map_err(from_migrate_error)?;
            tx.commit().await?;
        } else {
            MIGRATOR.run(&mut conn).await.map_err(from_migrate_error)?;
        }

        let after = status_on(&mut conn).await?;
        Ok(after
            .into_iter()
            .filter(|m| m.state == MigrationState::Applied)
            .filter(|m| before.iter().any(|b| b.version == m.version && b.state == MigrationState::Pending))
            .collect())
    }

    /// Reverte até `target`, todas ou nenhuma. `allow_data_loss` liga
    /// `ecosystem.allow_data_loss=on` nesta conexão: sem ele, uma `down` que
    /// apagaria linhas recusa e nada muda. Devolve as revertidas, da mais nova
    /// para a mais antiga.
    pub async fn revert(
        &self,
        target: RevertTarget,
        allow_data_loss: bool,
    ) -> Result<Vec<MigrationStatus>, StoreError> {
        let mut conn = self.connect(allow_data_loss).await?;
        let status = status_on(&mut conn).await?;
        ensure_consistent(&status)?;

        let applied: Vec<i64> =
            status.iter().filter(|m| m.state == MigrationState::Applied).map(|m| m.version).collect();
        let target = match target {
            RevertTarget::Last => match applied.as_slice() {
                [] => return Ok(Vec::new()),
                [.., previous, _last] => *previous,
                [_only] => 0,
            },
            RevertTarget::Version(version) if version == 0 || MIGRATOR.version_exists(version) => version,
            RevertTarget::Version(version) => return Err(StoreError::UnknownTarget(version)),
        };
        let reverting: Vec<MigrationStatus> =
            status.into_iter().rev().filter(|m| m.state == MigrationState::Applied && m.version > target).collect();

        if let Some(mut tx) = begin_if_atomic(&mut conn).await? {
            MIGRATOR.undo(&mut *tx, target).await.map_err(from_migrate_error)?;
            tx.commit().await?;
        } else {
            MIGRATOR.undo(&mut conn, target).await.map_err(from_migrate_error)?;
        }
        Ok(reverting)
    }

    async fn connect(&self, allow_data_loss: bool) -> Result<PgConnection, StoreError> {
        let options = if allow_data_loss {
            self.options.clone().options([("ecosystem.allow_data_loss", "on")])
        } else {
            self.options.clone()
        };
        let mut conn = PgConnection::connect_with(&options).await.map_err(StoreError::Connect)?;
        let found: i32 =
            sqlx::query_scalar("SELECT current_setting('server_version_num')::int").fetch_one(&mut conn).await?;
        if found < MIN_SERVER_VERSION {
            return Err(StoreError::UnsupportedServer { found });
        }
        Ok(conn)
    }
}

/// Proveniência de uma execução — vira uma linha de `ecosystem.sync_runs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunContext {
    /// `schedule` | `manual` | `push` | `api` | `test`.
    pub trigger: String,
    /// Quem executou (`cli`, `github-actions:<workflow>`).
    pub source: String,
    /// SHA do código.
    pub code_version: Option<String>,
    /// Referência externa (id do run no GitHub Actions).
    pub external_ref: Option<String>,
}

/// Uma checagem de site, no formato de `ecosystem.website_checks`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebsiteCheckRecord {
    /// URL verificada; casa com `websites.url` ativos.
    pub url: String,
    /// Início da checagem (ISO 8601 com fuso).
    pub checked_at: String,
    /// `verified` | `unreachable` | `invalid`.
    pub outcome: String,
    /// Código HTTP (100–599); `None` sem resposta.
    pub http_status: Option<i16>,
    /// Destino após redirects.
    pub final_url: Option<String>,
    /// Redirects seguidos.
    pub redirect_count: i16,
    /// Tempo até a resposta final.
    pub response_time_ms: Option<i32>,
    /// Tentativas (≥ 1).
    pub attempts: i16,
    /// `timeout` | `dns` | `connect` | `tls` | `http_status` | `invalid_url` | `other`.
    pub error_kind: Option<String>,
    /// Mensagem do erro.
    pub error_message: Option<String>,
}

/// O que foi gravado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordReport {
    /// A execução (`sync_runs.id`).
    pub sync_run_id: i64,
    /// Linhas acrescentadas a `website_checks`.
    pub recorded: usize,
    /// URLs sem site ativo em `ecosystem.websites` (quem registra sites é a
    /// coleta e a importação de manifestos, não o monitor).
    pub unregistered: Vec<String>,
}

impl Database {
    /// Acrescenta as checagens ao histórico, numa execução `websites`.
    ///
    /// Cada checagem vira uma linha para **cada** site ativo com aquela URL.
    /// Tudo numa transação: ou a execução entra inteira como `succeeded`, ou
    /// nada entra — e então fica registrada uma execução `failed` com o motivo
    /// ("falha é dado", D-007). O monitor não cria projetos nem sites: URL sem
    /// site registrado é só relatada.
    pub async fn record_website_checks(
        &self,
        run: &RunContext,
        checks: &[WebsiteCheckRecord],
    ) -> Result<RecordReport, StoreError> {
        let mut conn = self.connect(false).await?;
        match record_on(&mut conn, run, checks).await {
            Ok(report) => Ok(report),
            Err(error) => {
                record_failed_run(&mut conn, "websites", run, checks.len(), &error).await;
                Err(error)
            }
        }
    }
}

/// A transação falhou e foi desfeita: registra a execução como `failed`, com
/// o motivo, fora dela. Falhar aqui também não esconde o erro original.
pub(crate) async fn record_failed_run(
    conn: &mut PgConnection,
    kind: &str,
    run: &RunContext,
    items_seen: usize,
    error: &StoreError,
) {
    let _ = sqlx::query(
        "INSERT INTO ecosystem.sync_runs \
           (kind, trigger, source, code_version, external_ref, status, finished_at, items_seen, error_message) \
         VALUES ($1, $2, $3, $4, $5, 'failed', now(), $6, $7)",
    )
    .bind(kind)
    .bind(&run.trigger)
    .bind(&run.source)
    .bind(&run.code_version)
    .bind(&run.external_ref)
    .bind(i32::try_from(items_seen).unwrap_or(i32::MAX))
    .bind(error.to_string())
    .execute(conn)
    .await;
}

/// Abre uma execução (`running`) dentro da transação e devolve o id.
pub(crate) async fn open_run(
    tx: &mut Transaction<'_, Postgres>,
    kind: &str,
    run: &RunContext,
) -> Result<i64, StoreError> {
    Ok(sqlx::query_scalar(
        "INSERT INTO ecosystem.sync_runs (kind, trigger, source, code_version, external_ref) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(kind)
    .bind(&run.trigger)
    .bind(&run.source)
    .bind(&run.code_version)
    .bind(&run.external_ref)
    .fetch_one(&mut **tx)
    .await?)
}

/// Fecha uma execução como `succeeded`.
pub(crate) async fn close_run(
    tx: &mut Transaction<'_, Postgres>,
    sync_run_id: i64,
    items_seen: usize,
    items_changed: usize,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE ecosystem.sync_runs \
         SET status = 'succeeded', finished_at = clock_timestamp(), items_seen = $2, items_changed = $3 \
         WHERE id = $1",
    )
    .bind(sync_run_id)
    .bind(i32::try_from(items_seen).unwrap_or(i32::MAX))
    .bind(i32::try_from(items_changed).unwrap_or(i32::MAX))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn record_on(
    conn: &mut PgConnection,
    run: &RunContext,
    checks: &[WebsiteCheckRecord],
) -> Result<RecordReport, StoreError> {
    let mut tx = conn.begin().await?;
    let sync_run_id = open_run(&mut tx, "websites", run).await?;

    let mut recorded = 0_usize;
    let mut unregistered = Vec::new();
    for check in checks {
        let website_ids: Vec<i64> =
            sqlx::query_scalar("SELECT id FROM ecosystem.websites WHERE url = $1 AND retired_at IS NULL ORDER BY id")
                .bind(&check.url)
                .fetch_all(&mut *tx)
                .await?;
        if website_ids.is_empty() {
            unregistered.push(check.url.clone());
            continue;
        }
        for website_id in website_ids {
            sqlx::query(
                "INSERT INTO ecosystem.website_checks \
                   (website_id, sync_run_id, checked_at, outcome, http_status, final_url, redirect_count, \
                    response_time_ms, attempts, error_kind, error_message) \
                 VALUES ($1, $2, $3::timestamptz, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(website_id)
            .bind(sync_run_id)
            .bind(&check.checked_at)
            .bind(&check.outcome)
            .bind(check.http_status)
            .bind(&check.final_url)
            .bind(check.redirect_count)
            .bind(check.response_time_ms)
            .bind(check.attempts)
            .bind(&check.error_kind)
            .bind(&check.error_message)
            .execute(&mut *tx)
            .await?;
            recorded += 1;
        }
    }

    close_run(&mut tx, sync_run_id, checks.len(), recorded).await?;
    tx.commit().await?;
    Ok(RecordReport { sync_run_id, recorded, unregistered })
}

/// Transação externa, a menos que alguma migration não possa rodar em uma.
async fn begin_if_atomic(conn: &mut PgConnection) -> Result<Option<Transaction<'_, Postgres>>, StoreError> {
    if MIGRATOR.iter().any(|m| m.no_tx) {
        return Ok(None);
    }
    Ok(Some(conn.begin().await?))
}

type AppliedRow = (i64, bool, Vec<u8>, String, String);

async fn status_on(conn: &mut PgConnection) -> Result<Vec<MigrationStatus>, StoreError> {
    let has_table: bool =
        sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations') IS NOT NULL").fetch_one(&mut *conn).await?;
    let mut applied: BTreeMap<i64, AppliedRow> = BTreeMap::new();
    if has_table {
        let rows: Vec<AppliedRow> = sqlx::query_as(
            "SELECT version, success, checksum, description, \
                    to_char(installed_on AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') \
             FROM _sqlx_migrations ORDER BY version",
        )
        .fetch_all(&mut *conn)
        .await?;
        applied.extend(rows.into_iter().map(|row| (row.0, row)));
    }

    let mut report: Vec<MigrationStatus> = MIGRATOR
        .iter()
        .filter(|m| !m.migration_type.is_down_migration())
        .map(|migration| {
            let row = applied.remove(&migration.version);
            let state = match &row {
                None => MigrationState::Pending,
                Some((_, false, ..)) => MigrationState::Dirty,
                Some((_, true, checksum, ..)) if checksum.as_slice() != &*migration.checksum => {
                    MigrationState::Modified
                }
                Some(_) => MigrationState::Applied,
            };
            MigrationStatus {
                version: migration.version,
                description: migration.description.to_string(),
                state,
                installed_on: row.map(|(.., installed_on)| installed_on),
            }
        })
        .collect();
    report.extend(applied.into_values().map(|(version, _, _, description, installed_on)| MigrationStatus {
        version,
        description,
        state: MigrationState::Unknown,
        installed_on: Some(installed_on),
    }));
    report.sort_by_key(|m| m.version);
    Ok(report)
}

fn ensure_consistent(status: &[MigrationStatus]) -> Result<(), StoreError> {
    for migration in status {
        match migration.state {
            MigrationState::Unknown => return Err(StoreError::UnknownApplied(migration.version)),
            MigrationState::Modified => return Err(StoreError::Modified(migration.version)),
            MigrationState::Dirty => return Err(StoreError::Dirty(migration.version)),
            MigrationState::Applied | MigrationState::Pending => {}
        }
    }
    Ok(())
}

fn from_migrate_error(error: MigrateError) -> StoreError {
    match error {
        MigrateError::ExecuteMigration(sqlx::Error::Database(db), version)
            if db.message().starts_with(DATA_LOSS_MESSAGE) =>
        {
            StoreError::DataLossRefused { version, detail: db.message().to_string() }
        }
        MigrateError::VersionMismatch(version) => StoreError::Modified(version),
        MigrateError::VersionMissing(version) => StoreError::UnknownApplied(version),
        MigrateError::Dirty(version) => StoreError::Dirty(version),
        other => StoreError::Migrate(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_embutidas_sao_as_de_db_migrations() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../db/migrations");
        let mut on_disk: Vec<i64> = std::fs::read_dir(&dir)
            .expect("db/migrations")
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .filter_map(|name| name.strip_suffix(".up.sql")?.split('_').next()?.parse().ok())
            .collect();
        on_disk.sort_unstable();
        let ups: Vec<i64> =
            MIGRATOR.iter().filter(|m| !m.migration_type.is_down_migration()).map(|m| m.version).collect();
        let downs = MIGRATOR.iter().filter(|m| m.migration_type.is_down_migration()).count();
        assert_eq!(on_disk, ups, "o binário embute exatamente as migrations do repositório");
        assert_eq!((1..=ups.len() as i64).collect::<Vec<_>>(), ups, "versões contíguas a partir de 1");
        assert_eq!(ups.len(), downs, "toda up tem down");
    }

    #[test]
    fn nenhuma_migration_fora_de_transacao() {
        // Uma migration `-- no-transaction` tira a garantia de tudo ou nada
        // (ver a documentação do crate). Se ela for inevitável, este teste
        // muda junto com a documentação — de propósito, não por acidente.
        assert!(MIGRATOR.iter().all(|m| !m.no_tx));
    }

    #[test]
    fn url_invalida_e_recusada_sem_rede() {
        assert!(matches!(Database::from_url("mysql://x"), Err(StoreError::InvalidUrl(_))));
        assert!(matches!(Database::from_url("postgres://u:segredo@host:notaport/db"), Err(StoreError::InvalidUrl(_))));
        let error = Database::from_url("postgres://u:segredo@host:notaport/db").unwrap_err().to_string();
        assert!(!error.contains("segredo"), "{error}");
        assert!(Database::from_url("postgresql://u@localhost/db").is_ok());
    }
}
