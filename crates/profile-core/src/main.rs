//! `profile-core` — o binário do núcleo do ecossistema.
//!
//! Fase 1 (docs/migration/PYTHON-TO-RUST.md): só o schema do PostgreSQL.
//!
//! ```text
//! profile-core db status   [--json]
//! profile-core db migrate
//! profile-core db revert   [--to VERSÃO | --all] [--allow-data-loss]
//! ```
//!
//! A conexão vem de `DATABASE_URL` (ou `--database-url`, que deixa a senha
//! visível na lista de processos). Nenhuma mensagem repete a URL.
//!
//! Saída (docs/migration/PYTHON-TO-RUST.md §7): `0` sucesso · `1` uma
//! verificação reprovou (banco inconsistente, trava de perda de dados) · `2`
//! não foi possível executar (uso incorreto, URL, conexão, erro de SQL).

use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use serde_json::json;
use store::{Database, MigrationState, MigrationStatus, RevertTarget, StoreError};

#[derive(Parser)]
#[command(name = "profile-core", version, about = "Núcleo do ecossistema do perfil.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Schema do PostgreSQL: as migrations de db/migrations, embutidas no binário.
    #[command(subcommand)]
    Db(DbCommand),
}

#[derive(Args)]
struct Connection {
    /// URL do PostgreSQL (postgres://…). Prefira a variável de ambiente: na linha
    /// de comando, a senha fica visível na lista de processos.
    #[arg(long, env = "DATABASE_URL", hide_env_values = true, value_name = "URL")]
    database_url: String,
}

#[derive(Subcommand)]
enum DbCommand {
    /// Mostra a situação de cada migration. Não escreve nada no banco.
    Status {
        #[command(flatten)]
        connection: Connection,
        /// Saída em JSON.
        #[arg(long)]
        json: bool,
    },
    /// Aplica as migrations pendentes: todas ou nenhuma.
    Migrate {
        #[command(flatten)]
        connection: Connection,
    },
    /// Reverte migrations: todas as pedidas ou nenhuma. Sem opção, só a última.
    Revert {
        #[command(flatten)]
        connection: Connection,
        /// Reverte tudo o que está acima desta versão.
        #[arg(long, value_name = "VERSÃO", conflicts_with = "all")]
        to: Option<i64>,
        /// Reverte todas as migrations.
        #[arg(long)]
        all: bool,
        /// Permite que uma migration de reversão apague linhas
        /// (ecosystem.allow_data_loss=on). Exporte os dados antes.
        #[arg(long)]
        allow_data_loss: bool,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("profile-core: erro: {error}");
            if matches!(error, StoreError::DataLossRefused { .. }) {
                eprintln!("profile-core: dica: exporte os dados e repita com --allow-data-loss");
            }
            exit_code(&error)
        }
    }
}

/// `1` quando o comando verificou algo e reprovou; `2` quando não conseguiu
/// executar. O erro de uso do clap também sai com `2`.
fn exit_code(error: &StoreError) -> ExitCode {
    match error {
        StoreError::DataLossRefused { .. }
        | StoreError::Modified(_)
        | StoreError::Dirty(_)
        | StoreError::UnknownApplied(_) => ExitCode::from(1),
        StoreError::InvalidUrl(_)
        | StoreError::Connect(_)
        | StoreError::UnsupportedServer { .. }
        | StoreError::UnknownTarget(_)
        | StoreError::Migrate(_)
        | StoreError::Database(_) => ExitCode::from(2),
    }
}

async fn run(cli: Cli) -> Result<ExitCode, StoreError> {
    let Command::Db(command) = cli.command;
    match command {
        DbCommand::Status { connection, json } => {
            let status = Database::from_url(&connection.database_url)?.status().await?;
            if json {
                println!("{}", status_json(&status));
            } else {
                print!("{}", status_table(&status));
            }
            let inconsistent: Vec<_> = status
                .iter()
                .filter(|m| {
                    matches!(m.state, MigrationState::Modified | MigrationState::Dirty | MigrationState::Unknown)
                })
                .collect();
            if inconsistent.is_empty() {
                return Ok(ExitCode::SUCCESS);
            }
            for migration in inconsistent {
                eprintln!("profile-core: erro: migration {} está {}", migration.version, migration.state.as_str());
            }
            Ok(ExitCode::FAILURE)
        }
        DbCommand::Migrate { connection } => {
            let database = Database::from_url(&connection.database_url)?;
            let applied = database.migrate().await?;
            if applied.is_empty() {
                println!("nada a aplicar: o schema já está na versão mais nova que este binário conhece");
            }
            for migration in &applied {
                println!("aplicada   {:04}  {}", migration.version, migration.description);
            }
            Ok(ExitCode::SUCCESS)
        }
        DbCommand::Revert { connection, to, all, allow_data_loss } => {
            let target = match (to, all) {
                (Some(version), _) => RevertTarget::Version(version),
                (None, true) => RevertTarget::Version(0),
                (None, false) => RevertTarget::Last,
            };
            let database = Database::from_url(&connection.database_url)?;
            let reverted = database.revert(target, allow_data_loss).await?;
            if reverted.is_empty() {
                println!("nada a reverter");
            }
            for migration in &reverted {
                println!("revertida  {:04}  {}", migration.version, migration.description);
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn status_table(status: &[MigrationStatus]) -> String {
    let mut out = format!("{:<6}  {:<8}  {:<20}  {}\n", "VERSÃO", "ESTADO", "APLICADA EM (UTC)", "DESCRIÇÃO");
    for m in status {
        out.push_str(&format!(
            "{:<6}  {:<8}  {:<20}  {}\n",
            format!("{:04}", m.version),
            m.state.as_str(),
            m.installed_on.as_deref().unwrap_or("-"),
            m.description
        ));
    }
    let count = |state| status.iter().filter(|m| m.state == state).count();
    out.push_str(&format!(
        "\n{} migrations: {} aplicadas, {} pendentes\n",
        status.len(),
        count(MigrationState::Applied),
        count(MigrationState::Pending)
    ));
    out
}

fn status_json(status: &[MigrationStatus]) -> String {
    let migrations: Vec<_> = status
        .iter()
        .map(|m| {
            json!({
                "version": m.version,
                "description": m.description,
                "state": m.state.as_str(),
                "installed_on": m.installed_on,
            })
        })
        .collect();
    json!({ "migrations": migrations }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn definicao_da_cli_e_valida() {
        Cli::command().debug_assert();
    }

    #[test]
    fn revert_nao_aceita_to_e_all_juntos() {
        let parsed = Cli::try_parse_from([
            "profile-core",
            "db",
            "revert",
            "--database-url",
            "postgres://x/y",
            "--to",
            "3",
            "--all",
        ]);
        assert!(parsed.is_err());
    }
}
