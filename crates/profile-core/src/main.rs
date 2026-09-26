//! `profile-core` — o binário do núcleo do ecossistema.
//!
//! ```text
//! profile-core db status     [--json]
//! profile-core db migrate
//! profile-core db revert     [--to VERSÃO | --all] [--allow-data-loss]
//! profile-core check sites   [--json] [--fail-on-down] [--timeout S] [--retries N]
//!                            [--max-workers N] [--root DIR] [--no-db] [--trigger T]
//! profile-core catalog build [--root DIR] [--input-repos FILE] [--now ISO] [--write]
//!                            [--site-checks-fixture FILE | --skip-site-check]
//!                            [--site-timeout S] [--site-workers N] [--site-checks-out FILE]
//! ```
//!
//! `catalog build` gera o `docs/project-catalog.json` com os mesmos bytes do
//! `update_profile.py`. Sem `--write`, imprime o catálogo e não grava nada;
//! com `--write`, recusa sem `PROFILE_GITHUB_TOKEN` (a menos que o inventário
//! venha de arquivo), como o Python: um inventário só de públicos apagaria os
//! privados do catálogo.
//!
//! `check sites` imprime o relatório do `scripts/check_websites.py` byte a
//! byte (exceto o `checked_at`) e grava cada checagem em
//! `ecosystem.website_checks`. Sem banco configurado ele **recusa** em vez de
//! pular o histórico calado (lição do A1): use `--no-db` para só verificar.
//!
//! A conexão vem de `DATABASE_URL` (ou `--database-url`, que deixa a senha
//! visível na lista de processos). Nenhuma mensagem repete a URL.
//!
//! Saída (docs/migration/PYTHON-TO-RUST.md §7): `0` sucesso · `1` uma
//! verificação reprovou (banco inconsistente, trava de perda de dados) · `2`
//! não foi possível executar (uso incorreto, URL, conexão, erro de SQL).

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};
use profile_core::catalog_build::{self, BuildOptions, ChecksSource};
use profile_core::sites;
use serde_json::json;
use site_monitor::check::BuildError;
use site_monitor::{Checker, Options, Status, WebsiteCheck};
use store::{Database, MigrationState, MigrationStatus, RevertTarget, RunContext, StoreError, WebsiteCheckRecord};

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
    /// Verificações.
    #[command(subcommand)]
    Check(CheckCommand),
    /// Catálogo do ecossistema (docs/project-catalog.json).
    #[command(subcommand)]
    Catalog(CatalogCommand),
}

#[derive(Subcommand)]
enum CatalogCommand {
    /// Gera o catálogo a partir do inventário do GitHub (o update_profile.py, sem o README).
    Build(CatalogArgs),
}

#[derive(Args)]
struct CatalogArgs {
    /// Raiz com docs/ (manifestos e catálogo).
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Inventário de um arquivo (array JSON da API) em vez do GitHub.
    #[arg(long, value_name = "FILE")]
    input_repos: Option<PathBuf>,
    /// Resultados de verificação de um arquivo em vez de HTTP.
    #[arg(long, value_name = "FILE", conflicts_with = "skip_site_check")]
    site_checks_fixture: Option<PathBuf>,
    /// Não verifica os sites: todos saem como não verificados.
    #[arg(long)]
    skip_site_check: bool,
    /// Timeout de cada verificação, em segundos.
    #[arg(long, default_value_t = 15.0, value_parser = positive_seconds)]
    site_timeout: f64,
    /// Verificações simultâneas.
    #[arg(long, default_value_t = 6)]
    site_workers: usize,
    /// Relógio fixo, ISO 8601 com fuso (2026-09-25T12:00:00Z).
    #[arg(long, value_name = "ISO")]
    now: Option<String>,
    /// Grava docs/project-catalog.json (só se mudou).
    #[arg(long)]
    write: bool,
    /// Grava também os resultados de verificação usados, no formato de --site-checks-fixture.
    #[arg(long, value_name = "FILE")]
    site_checks_out: Option<PathBuf>,
}

#[derive(Subcommand)]
enum CheckCommand {
    /// Verifica os sites do catálogo e do manifesto (o check_websites.py) e
    /// grava o histórico.
    Sites(SitesArgs),
}

#[derive(Args)]
struct SitesArgs {
    /// Saída em JSON (o formato do check_websites.py --json).
    #[arg(long)]
    json: bool,
    /// Sai com 1 se algum site conhecido estiver fora.
    #[arg(long)]
    fail_on_down: bool,
    /// Timeout de cada conexão e de cada leitura, em segundos.
    #[arg(long, default_value_t = 15.0, value_parser = positive_seconds)]
    timeout: f64,
    /// Tentativas extras por URL.
    #[arg(long, default_value_t = 1)]
    retries: u32,
    /// Verificações simultâneas.
    #[arg(long, default_value_t = 6)]
    max_workers: usize,
    /// Raiz com docs/project-catalog.json e docs/README_SITES.json.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// URL do PostgreSQL onde o histórico é gravado. Prefira a variável de ambiente.
    #[arg(long, env = "DATABASE_URL", hide_env_values = true, value_name = "URL")]
    database_url: Option<String>,
    /// Só verifica, sem gravar histórico (modo sombra, comparação com o Python).
    #[arg(long)]
    no_db: bool,
    /// Gatilho registrado em ecosystem.sync_runs.
    #[arg(long, default_value = "manual", value_parser = ["schedule", "manual", "push", "api", "test"])]
    trigger: String,
}

fn positive_seconds(value: &str) -> Result<f64, String> {
    match value.parse::<f64>() {
        Ok(seconds) if seconds.is_finite() && seconds > 0.0 => Ok(seconds),
        _ => Err("precisa ser um número de segundos maior que zero".into()),
    }
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Catalog(#[from] catalog_build::BuildError),
    #[error(transparent)]
    Collect(#[from] sites::CollectError),
    #[error(transparent)]
    Http(#[from] BuildError),
    #[error("check sites grava o histórico no banco: defina DATABASE_URL ou passe --no-db")]
    NoDatabase,
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
            if matches!(error, CliError::Store(StoreError::DataLossRefused { .. })) {
                eprintln!("profile-core: dica: exporte os dados e repita com --allow-data-loss");
            }
            exit_code(&error)
        }
    }
}

/// `1` quando o comando verificou algo e reprovou; `2` quando não conseguiu
/// executar. O erro de uso do clap também sai com `2`.
fn exit_code(error: &CliError) -> ExitCode {
    let CliError::Store(error) = error else {
        return ExitCode::from(2);
    };
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

async fn run(cli: Cli) -> Result<ExitCode, CliError> {
    let command = match cli.command {
        Command::Db(command) => command,
        Command::Check(CheckCommand::Sites(args)) => return check_sites(args).await,
        Command::Catalog(CatalogCommand::Build(args)) => return build_catalog(args).await,
    };
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

async fn check_sites(args: SitesArgs) -> Result<ExitCode, CliError> {
    let database = if args.no_db {
        None
    } else {
        Some(Database::from_url(args.database_url.as_deref().ok_or(CliError::NoDatabase)?)?)
    };
    let urls = sites::collect_urls(&args.root)?;
    if urls.is_empty() {
        print!("{}", sites::NO_URLS);
        return Ok(ExitCode::SUCCESS);
    }

    let options = Options {
        timeout: Duration::from_secs_f64(args.timeout),
        retries: args.retries,
        max_workers: args.max_workers,
    };
    let checks = Checker::new(options)?.check_websites(urls.values().cloned()).await;
    let rows = sites::build_rows(&urls, &checks);
    print!("{}", if args.json { sites::render_json(&rows) } else { sites::render_text(&rows) });

    let mut ordered: Vec<&WebsiteCheck> = checks.values().collect();
    ordered.sort_by(|a, b| a.url.cmp(&b.url));
    for check in ordered.iter().filter(|c| c.python_crash.is_some()) {
        eprintln!(
            "profile-core: aviso: {:?} derrubaria o check_websites.py ({}); aqui conta como fora do ar",
            check.url,
            check.python_crash.unwrap_or_default()
        );
    }
    if let Some(database) = database {
        let records: Vec<WebsiteCheckRecord> =
            ordered.iter().filter(|c| c.status != Status::Invalid).map(|c| to_record(c)).collect();
        let report = database.record_website_checks(&run_context(&args.trigger), &records).await?;
        eprintln!(
            "profile-core: histórico: {} checagens gravadas (sync_run {}); {} URLs sem site registrado",
            report.recorded,
            report.sync_run_id,
            report.unregistered.len()
        );
    }
    Ok(ExitCode::from(sites::exit_code(&rows, args.fail_on_down)))
}

async fn build_catalog(args: CatalogArgs) -> Result<ExitCode, CliError> {
    let checks = match (args.site_checks_fixture, args.skip_site_check) {
        (Some(path), _) => ChecksSource::Fixture(path),
        (None, true) => ChecksSource::Skip,
        (None, false) => {
            ChecksSource::Live { timeout: Duration::from_secs_f64(args.site_timeout), workers: args.site_workers }
        }
    };
    let options = BuildOptions {
        root: args.root,
        input_repos: args.input_repos,
        checks,
        now: args.now,
        write: args.write,
        checks_out: args.site_checks_out,
    };
    let token = std::env::var("PROFILE_GITHUB_TOKEN").ok();
    let built = catalog_build::run(&options, token).await?;
    if built.written.is_none() {
        print!("{}", built.text);
    }
    eprintln!("profile-core: {}", catalog_build::summary(&built));
    Ok(ExitCode::SUCCESS)
}

fn to_record(check: &WebsiteCheck) -> WebsiteCheckRecord {
    let http_status = (100..=599).contains(&check.http_status).then_some(check.http_status as i16);
    let error_message = if check.http_status != 0 && http_status.is_none() {
        Some(format!("HTTP {} (fora de 100–599)", check.http_status))
    } else {
        check.error_message.clone()
    };
    WebsiteCheckRecord {
        url: check.url.clone(),
        checked_at: check.checked_at.clone(),
        outcome: check.status.as_str().into(),
        http_status,
        final_url: (!check.final_url.is_empty()).then(|| check.final_url.clone()),
        redirect_count: i16::try_from(check.redirect_count).unwrap_or(i16::MAX),
        response_time_ms: check.response_time_ms.map(|ms| i32::try_from(ms).unwrap_or(i32::MAX)),
        attempts: i16::try_from(check.attempts.max(1)).unwrap_or(i16::MAX),
        error_kind: check.error_kind.map(|kind| kind.as_str().into()),
        error_message,
    }
}

/// Proveniência: no GitHub Actions, o workflow, o SHA e o run.
fn run_context(trigger: &str) -> RunContext {
    let env = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
    RunContext {
        trigger: trigger.into(),
        source: env("GITHUB_WORKFLOW").map_or_else(|| "cli".into(), |workflow| format!("github-actions:{workflow}")),
        code_version: env("GITHUB_SHA"),
        external_ref: env("GITHUB_RUN_ID"),
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
