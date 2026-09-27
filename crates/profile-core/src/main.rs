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
//! profile-core render readme [--root DIR] [--input-repos FILE [--languages-dir DIR]] [--now ISO]
//!                            [--write] [--out-dir DIR] [--catalog-out FILE]
//!                            [--site-checks-fixture FILE | --skip-site-check]
//!                            [--site-timeout S] [--site-workers N] [--site-checks-out FILE]
//! profile-core render lang-stats | cards | assets [--root DIR] [--now ISO]
//!                            [--include-forks] [--skip-files]
//! profile-core render readme … --check
//! profile-core render all    (--write | --out-dir DIR | --check) [as opções do render readme]
//!                            [--include-forks] [--skip-files]
//! profile-core sync all      [--root DIR] [--now ISO] [--write] [--no-db] [--trigger T]
//! profile-core sync commits  [--root DIR] [--now ISO] [--write] [--no-db] [--trigger T]
//! profile-core sync github   [--root DIR] [--input-repos FILE [--languages-dir DIR]] [--no-db] [--trigger T]
//! profile-core sync contributions [--root DIR] [--now ISO] [--write] [--no-db] [--trigger T]
//! profile-core import manifests   [--root DIR] [--trigger T]
//! profile-core import legacy      [--root DIR] [--trigger T]
//! profile-core validate readme    --before FILE --after FILE
//! profile-core validate exclusions | links | visual | catalog [--root DIR]
//! profile-core validate badges    [--root DIR] [--offline]
//! profile-core validate all       --before FILE [--root DIR] [--online]
//! ```
//!
//! `validate` são os `scripts/validate_*.py`: mesma saída, mesmo stderr e
//! mesmo código de saída (onde o script sairia com traceback, `1` e a
//! exceção). `validate all` roda o que o `update-profile.yml` confere depois
//! de gerar, sem parar no primeiro que reprovar.
//!
//! `import manifests` deixa o banco igual aos manifestos editoriais
//! (docs/README_*.json); `import legacy` é a carga única do estado que hoje
//! vive em JSON (docs/database/MIGRATIONS.md §6) — idempotente.
//!
//! `sync github` grava donos, repositórios, linguagens e projetos. Gravar
//! exige o inventário completo (`PROFILE_GITHUB_TOKEN` ou `--input-repos`):
//! sem os privados, eles pareceriam sumidos.
//!
//! `sync commits` é o `ecosystem_watch.py`: mesmo estado, mesmo relatório e
//! mesmos contadores, lendo o `docs/ECOSYSTEM-COMMIT-STATE.json` como estado
//! anterior. Com banco, grava as transições e os contadores; sem banco
//! configurado recusa, a menos que `--no-db`.
//!
//! `catalog build` gera o `docs/project-catalog.json` com os mesmos bytes do
//! `update_profile.py`. Sem `--write`, imprime o catálogo e não grava nada;
//! com `--write`, recusa sem `PROFILE_GITHUB_TOKEN` (a menos que o inventário
//! venha de arquivo), como o Python: um inventário só de públicos apagaria os
//! privados do catálogo.
//!
//! `render readme` é o `update_profile.py` inteiro: README, snapshot e
//! catálogo com os mesmos bytes, e a mesma saída padrão. Com `--write`, as
//! mesmas regras do `catalog build`; `--out-dir` grava tudo num diretório à
//! parte, sem tocar no que é publicado (o modo sombra).
//!
//! `render lang-stats` e `render cards` são o `lang_stats.py` e o
//! `profile_cards.py` (os SVGs de `assets/`); `render assets` roda os dois,
//! como o `lang-stats.yml`. Mesmas variáveis de ambiente dos scripts.
//!
//! `render all` é o pipeline único do D-018: README, lang-stats e cards num
//! processo. `--check` (também no `render readme`) não grava nada e sai com
//! `1` se algum arquivo publicado mudaria. `sync all` roda a coleta inteira:
//! github → manifestos → commits → contribuições → sites.
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
use ecosystem_domain::classify::{Classifier, PY_V1, V2};
use profile_core::assets::{self, AssetsError};
use profile_core::catalog_build::{self, BuildOptions, ChecksSource, DbSource, Tokens};
use profile_core::commits;
use profile_core::contributions;
use profile_core::imports;
use profile_core::inventory_sync::{self, SyncOptions};
use profile_core::pipeline::{self, Mode, PipelineError};
use profile_core::render::{self, RenderOptions};
use profile_core::sites;
use profile_core::validate::{self, Crash, Outcome};
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
    /// Coleta do GitHub.
    #[command(subcommand)]
    Sync(SyncCommand),
    /// Importações para o banco.
    #[command(subcommand)]
    Import(ImportCommand),
    /// Saídas publicadas no perfil.
    #[command(subcommand)]
    Render(RenderCommand),
    /// Validadores do README e do catálogo (os scripts/validate_*.py).
    #[command(subcommand)]
    Validate(ValidateCommand),
}

#[derive(Subcommand)]
enum ValidateCommand {
    /// Fora dos 13 blocos gerados, nada mudou (validate_dynamic_sections.py).
    Readme {
        /// README antes da geração.
        #[arg(long, value_name = "FILE")]
        before: PathBuf,
        /// README depois da geração.
        #[arg(long, value_name = "FILE")]
        after: PathBuf,
    },
    /// Nenhum repositório excluído aparece no README (validate_exclusions.py).
    Exclusions(ValidateRoot),
    /// Com site verificado, o site vem antes do código (validate_project_links.py).
    Links(ValidateRoot),
    /// A identidade visual do README continua lá (validate_restored_style.py).
    Visual(ValidateRoot),
    /// Badges de linguagem e categorias do Arsenal (validate_language_badges.py).
    Badges {
        #[command(flatten)]
        root: ValidateRoot,
        /// Não confere as URLs dos badges (o resto, sim).
        #[arg(long)]
        offline: bool,
    },
    /// Catálogo coerente com o README e os manifestos (validate_profile.py, sem a rede).
    Catalog(ValidateRoot),
    /// Tudo o que o update-profile.yml confere, sem parar no primeiro que reprovar.
    All {
        #[command(flatten)]
        root: ValidateRoot,
        /// README antes da geração (para `readme`).
        #[arg(long, value_name = "FILE")]
        before: PathBuf,
        /// Confere também as URLs dos badges.
        #[arg(long)]
        online: bool,
    },
}

#[derive(Args)]
struct ValidateRoot {
    /// Raiz com README.md, docs/ e assets/.
    #[arg(long, default_value = ".")]
    root: PathBuf,
}

#[derive(Subcommand)]
enum RenderCommand {
    /// README, profile-snapshot.svg e catálogo (o update_profile.py inteiro).
    Readme(ReadmeArgs),
    /// assets/lang-stats.svg e assets/profile-top-langs.svg (o lang_stats.py).
    LangStats(AssetsArgs),
    /// Os quatro cards do GraphQL (o profile_cards.py).
    Cards(AssetsArgs),
    /// lang-stats e cards, nessa ordem, parando no primeiro que falhar.
    Assets(AssetsArgs),
    /// README, lang-stats e cards num processo (o pipeline único do D-018).
    All(AllArgs),
}

#[derive(Args)]
struct AllArgs {
    #[command(flatten)]
    catalog: CatalogArgs,
    /// Linguagens de arquivo, um owner__nome.json por repositório (com --input-repos).
    #[arg(long, value_name = "DIR", requires = "input_repos")]
    languages_dir: Option<PathBuf>,
    /// Grava tudo neste diretório, com o layout da raiz, sem tocar no que é publicado.
    #[arg(long, value_name = "DIR", conflicts_with_all = ["write", "check"])]
    out_dir: Option<PathBuf>,
    /// Não grava nada: sai com 1 se algum arquivo publicado mudaria.
    #[arg(long, conflicts_with = "write")]
    check: bool,
    /// Inclui os forks nos SVGs de linguagem (ou INCLUDE_FORKS=1).
    #[arg(long)]
    include_forks: bool,
    /// Não lê as árvores git: sem tipos de arquivo (ou SEM_ARQUIVOS=1).
    #[arg(long)]
    skip_files: bool,
}

#[derive(Args)]
struct AssetsArgs {
    /// Raiz com docs/README_EXCLUDED.json e assets/.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Relógio fixo, ISO 8601 com fuso.
    #[arg(long, value_name = "ISO")]
    now: Option<String>,
    /// Inclui os forks (ou INCLUDE_FORKS=1, como no script).
    #[arg(long)]
    include_forks: bool,
    /// Não lê as árvores git: sem tipos de arquivo (ou SEM_ARQUIVOS=1).
    #[arg(long)]
    skip_files: bool,
}

#[derive(Args)]
struct ReadmeArgs {
    #[command(flatten)]
    catalog: CatalogArgs,
    /// Linguagens de arquivo, um owner__nome.json por repositório (com --input-repos).
    #[arg(long, value_name = "DIR", requires = "input_repos")]
    languages_dir: Option<PathBuf>,
    /// Grava também o catálogo neste arquivo.
    #[arg(long, value_name = "FILE")]
    catalog_out: Option<PathBuf>,
    /// Grava README.md, profile-snapshot.svg e project-catalog.json neste
    /// diretório, sem tocar no que é publicado (vale sem token).
    #[arg(long, value_name = "DIR")]
    out_dir: Option<PathBuf>,
    /// Não grava nada: sai com 1 se o README, o snapshot ou o catálogo mudariam.
    #[arg(long, conflicts_with_all = ["write", "out_dir", "catalog_out"])]
    check: bool,
}

#[derive(Subcommand)]
enum ImportCommand {
    /// Manifestos editoriais (exclusões, curadoria, arsenal, sites manuais).
    Manifests(ImportArgs),
    /// Carga única do estado legado (catálogo, monitor, timeline, constantes do código).
    Legacy(ImportArgs),
}

#[derive(Args)]
struct ImportArgs {
    /// Raiz com docs/.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    #[command(flatten)]
    connection: Connection,
    /// Gatilho registrado em ecosystem.sync_runs.
    #[arg(long, default_value = "manual", value_parser = ["schedule", "manual", "push", "api", "test"])]
    trigger: String,
}

#[derive(Subcommand)]
enum SyncCommand {
    /// Monitor de commits do ecossistema (o ecosystem_watch.py).
    Commits(CommitsArgs),
    /// Inventário do GitHub: donos, repositórios, linguagens e projetos.
    Github(GithubArgs),
    /// Contribuições mensais do GraphQL (o update_contribution_timeline.py).
    Contributions(ContributionsArgs),
    /// A coleta inteira: github → manifestos → commits → contribuições → sites.
    All(SyncAllArgs),
}

#[derive(Args)]
struct SyncAllArgs {
    /// Raiz com docs/.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Relógio fixo, ISO 8601 com fuso.
    #[arg(long, value_name = "ISO")]
    now: Option<String>,
    /// Grava os arquivos do monitor e da timeline em docs/.
    #[arg(long)]
    write: bool,
    /// URL do PostgreSQL. Prefira a variável de ambiente.
    #[arg(long, env = "DATABASE_URL", hide_env_values = true, value_name = "URL")]
    database_url: Option<String>,
    /// Sem banco: só coleta e relata (os manifestos não são importados).
    #[arg(long)]
    no_db: bool,
    /// Gatilho registrado em ecosystem.sync_runs.
    #[arg(long, default_value = "manual", value_parser = ["schedule", "manual", "push", "api", "test"])]
    trigger: String,
}

#[derive(Args)]
struct ContributionsArgs {
    /// Raiz onde docs/assets/ é gravado.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Relógio fixo, ISO 8601 com fuso.
    #[arg(long, value_name = "ISO")]
    now: Option<String>,
    /// Grava o JSON e a página em docs/assets/; sem ela, o JSON vai para a saída padrão.
    #[arg(long)]
    write: bool,
    /// URL do PostgreSQL. Prefira a variável de ambiente.
    #[arg(long, env = "DATABASE_URL", hide_env_values = true, value_name = "URL")]
    database_url: Option<String>,
    /// Só coleta, sem gravar no banco (modo sombra).
    #[arg(long)]
    no_db: bool,
    /// Gatilho registrado em ecosystem.sync_runs.
    #[arg(long, default_value = "manual", value_parser = ["schedule", "manual", "push", "api", "test"])]
    trigger: String,
}

#[derive(Args)]
struct GithubArgs {
    /// Raiz com docs/README_EXCLUDED.json.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Inventário de um arquivo (array JSON da API) em vez do GitHub.
    #[arg(long, value_name = "FILE")]
    input_repos: Option<PathBuf>,
    /// Linguagens de arquivo, um owner__nome.json por repositório (com --input-repos).
    #[arg(long, value_name = "DIR", requires = "input_repos")]
    languages_dir: Option<PathBuf>,
    /// URL do PostgreSQL. Prefira a variável de ambiente.
    #[arg(long, env = "DATABASE_URL", hide_env_values = true, value_name = "URL")]
    database_url: Option<String>,
    /// Só lê e relata, sem gravar (modo sombra).
    #[arg(long)]
    no_db: bool,
    /// Gatilho registrado em ecosystem.sync_runs.
    #[arg(long, default_value = "manual", value_parser = ["schedule", "manual", "push", "api", "test"])]
    trigger: String,
    /// Versão da heurística que rotula os projetos novos e os de rótulo heurístico.
    #[arg(long, default_value = PY_V1, value_parser = [PY_V1, V2])]
    classifier: String,
}

#[derive(Args)]
struct CommitsArgs {
    /// Raiz com docs/ECOSYSTEM-COMMIT-STATE.json (o estado anterior).
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Relógio fixo, ISO 8601 com fuso.
    #[arg(long, value_name = "ISO")]
    now: Option<String>,
    /// Grava o estado e o relatório em docs/ quando houver mudança; sem ela,
    /// o estado novo vai para a saída padrão.
    #[arg(long)]
    write: bool,
    /// URL do PostgreSQL onde as transições são gravadas. Prefira a variável de ambiente.
    #[arg(long, env = "DATABASE_URL", hide_env_values = true, value_name = "URL")]
    database_url: Option<String>,
    /// Só varre, sem gravar no banco (modo sombra).
    #[arg(long)]
    no_db: bool,
    /// Gatilho registrado em ecosystem.sync_runs.
    #[arg(long, default_value = "manual", value_parser = ["schedule", "manual", "push", "api", "test"])]
    trigger: String,
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
    /// Inventário, linguagens e checagens de site do banco (DATABASE_URL), não do
    /// GitHub; o editorial continua vindo dos manifestos.
    #[arg(long, conflicts_with = "input_repos")]
    from_db: bool,
    /// URL do PostgreSQL para --from-db. Prefira a variável de ambiente.
    #[arg(long, env = "DATABASE_URL", hide_env_values = true, value_name = "URL")]
    database_url: Option<String>,
    /// Versão da heurística de rótulo: a do Python (padrão) ou a que casa
    /// palavras inteiras (corrige A6; muda a saída pública — D-039).
    #[arg(long, default_value = PY_V1, value_parser = [PY_V1, V2])]
    classifier: String,
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
    #[error("{0} grava o histórico no banco: defina DATABASE_URL ou passe --no-db")]
    NoDatabase(&'static str),
    #[error(transparent)]
    CatalogInput(#[from] catalog::CatalogError),
    #[error(transparent)]
    GitHubClient(#[from] github_client::BuildError),
    #[error(transparent)]
    Crash(#[from] commits::Crash),
    #[error(transparent)]
    Assets(#[from] AssetsError),
    #[error(transparent)]
    Pipeline(#[from] PipelineError),
    #[error("{0}: {1}")]
    Io(String, std::io::Error),
    #[error(transparent)]
    Inventory(#[from] inventory_sync::SyncError),
    #[error("PROFILE_README_TOKEN ou GITHUB_TOKEN ausente")]
    NoGraphQlToken,
    #[error("timeline generation failed: {0}")]
    Timeline(String),
    #[error(
        "sync github grava o inventário completo: defina PROFILE_GITHUB_TOKEN ou use --input-repos \
         (só com os públicos, os privados pareceriam sumidos), ou passe --no-db"
    )]
    IncompleteInventory,
    #[error("--from-db lê o inventário do banco: defina DATABASE_URL")]
    FromDbWithoutUrl,
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
    if matches!(error, CliError::Crash(_) | CliError::Assets(AssetsError::Crash(_)))
        || matches!(error, CliError::Catalog(error) if error.is_python_crash())
        || matches!(error, CliError::Pipeline(error) if error.is_python_crash())
    {
        // Onde o Python sairia com traceback (código 1).
        return ExitCode::from(1);
    }
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
        | StoreError::Manifest(_)
        | StoreError::Database(_) => ExitCode::from(2),
    }
}

async fn run(cli: Cli) -> Result<ExitCode, CliError> {
    let command = match cli.command {
        Command::Db(command) => command,
        Command::Check(CheckCommand::Sites(args)) => return check_sites(args).await,
        Command::Catalog(CatalogCommand::Build(args)) => return build_catalog(args).await,
        Command::Sync(SyncCommand::Commits(args)) => return sync_commits(args).await,
        Command::Sync(SyncCommand::Github(args)) => return sync_github(args).await,
        Command::Sync(SyncCommand::Contributions(args)) => return sync_contributions(args).await,
        Command::Sync(SyncCommand::All(args)) => return sync_all(args).await,
        Command::Import(ImportCommand::Manifests(args)) => return import_manifests(args).await,
        Command::Import(ImportCommand::Legacy(args)) => return import_legacy(args).await,
        Command::Render(RenderCommand::Readme(args)) => return render_readme(args).await,
        Command::Render(RenderCommand::LangStats(args)) => return render_assets(args, true, false).await,
        Command::Render(RenderCommand::Cards(args)) => return render_assets(args, false, true).await,
        Command::Render(RenderCommand::Assets(args)) => return render_assets(args, true, true).await,
        Command::Render(RenderCommand::All(args)) => return render_all(args).await,
        Command::Validate(command) => return Ok(run_validate(command).await),
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
        Some(Database::from_url(args.database_url.as_deref().ok_or(CliError::NoDatabase("check sites"))?)?)
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

/// A versão escolhida (o clap já recusou nome desconhecido).
fn classifier(name: &str) -> Classifier {
    Classifier::parse(name).unwrap_or_default()
}

fn build_options(args: CatalogArgs, languages_dir: Option<PathBuf>) -> Result<BuildOptions, CliError> {
    let from_db = match (args.from_db, args.database_url) {
        (false, _) => None,
        (true, Some(url)) => Some(DbSource(url)),
        (true, None) => return Err(CliError::FromDbWithoutUrl),
    };
    let checks = match (args.site_checks_fixture, args.skip_site_check) {
        (Some(path), _) => ChecksSource::Fixture(path),
        (None, true) => ChecksSource::Skip,
        (None, false) => {
            ChecksSource::Live { timeout: Duration::from_secs_f64(args.site_timeout), workers: args.site_workers }
        }
    };
    Ok(BuildOptions {
        root: args.root,
        input_repos: args.input_repos,
        languages_dir,
        checks,
        now: args.now,
        write: args.write,
        checks_out: args.site_checks_out,
        from_db,
        classifier: classifier(&args.classifier),
    })
}

async fn render_readme(args: ReadmeArgs) -> Result<ExitCode, CliError> {
    let site_check_skipped = args.catalog.skip_site_check;
    let options = RenderOptions {
        build: build_options(args.catalog, args.languages_dir)?,
        catalog_out: args.catalog_out,
        out_dir: args.out_dir,
        site_check_skipped,
    };
    if args.check {
        return Ok(print_report(pipeline::check_readme(&options, &Tokens::from_env()).await?));
    }
    let rendered = render::run(&options, &Tokens::from_env()).await?;
    print!("{}", rendered.stdout);
    eprintln!("profile-core: {}", rendered.note);
    Ok(ExitCode::SUCCESS)
}

/// As opções dos SVGs, com as variáveis de ambiente dos scripts.
fn assets_options(root: PathBuf, now: Option<String>, include_forks: bool, skip_files: bool) -> assets::Options {
    let env = |name: &str| std::env::var(name).ok();
    assets::Options {
        root,
        now,
        token: env("GITHUB_TOKEN").filter(|token| !token.is_empty()),
        user: env("GH_USER").unwrap_or_else(|| catalog::OWNER.into()),
        include_forks: include_forks || env("INCLUDE_FORKS").as_deref() == Some("1"),
        skip_files: skip_files || env("SEM_ARQUIVOS").as_deref() == Some("1"),
        out: None,
    }
}

fn print_report(report: pipeline::Report) -> ExitCode {
    print!("{}", report.stdout);
    for note in &report.notes {
        eprintln!("profile-core: {note}");
    }
    ExitCode::from(report.code)
}

async fn render_all(args: AllArgs) -> Result<ExitCode, CliError> {
    let mode = match (args.catalog.write, args.out_dir, args.check) {
        (true, _, _) => Mode::Write,
        (false, Some(dir), _) => Mode::OutDir(dir),
        (false, None, true) => Mode::Check,
        (false, None, false) => {
            eprintln!("profile-core: erro: render all precisa de --write, --out-dir DIR ou --check");
            return Ok(ExitCode::from(2));
        }
    };
    let assets =
        assets_options(args.catalog.root.clone(), args.catalog.now.clone(), args.include_forks, args.skip_files);
    let site_check_skipped = args.catalog.skip_site_check;
    let options = RenderOptions {
        build: build_options(args.catalog, args.languages_dir)?,
        catalog_out: None,
        out_dir: None,
        site_check_skipped,
    };
    Ok(print_report(pipeline::render_all(&options, &assets, &Tokens::from_env(), &mode).await?))
}

async fn render_assets(args: AssetsArgs, lang_stats: bool, cards: bool) -> Result<ExitCode, CliError> {
    let options = assets_options(args.root, args.now, args.include_forks, args.skip_files);
    if lang_stats {
        let outcome = assets::run_lang_stats(&options).await?;
        print!("{}", outcome.stdout);
        if outcome.code != 0 {
            return Ok(ExitCode::from(outcome.code));
        }
    }
    if cards {
        print!("{}", assets::run_cards(&options).await?.stdout);
    }
    Ok(ExitCode::SUCCESS)
}

async fn build_catalog(args: CatalogArgs) -> Result<ExitCode, CliError> {
    let options = build_options(args, None)?;
    let built = catalog_build::run(&options, &Tokens::from_env()).await?;
    if built.written.is_none() {
        print!("{}", built.text);
    }
    eprintln!("profile-core: {}", catalog_build::summary(&built));
    Ok(ExitCode::SUCCESS)
}

async fn sync_commits(args: CommitsArgs) -> Result<ExitCode, CliError> {
    let database = if args.no_db {
        None
    } else {
        Some(Database::from_url(args.database_url.as_deref().ok_or(CliError::NoDatabase("sync commits"))?)?)
    };
    let now = match &args.now {
        Some(value) => catalog::parse_now(value)?,
        None => chrono::Utc::now(),
    };
    let identity = commits::Identity::from_env();
    let previous = commits::Previous::read(&args.root)?;
    let mut settings = github_client::Settings::new(commits::USER_AGENT);
    settings.api_version_header = true;
    let client = github_client::Client::new(settings)?.with_token(std::env::var("GITHUB_TOKEN").ok());
    let scan = commits::scan(&commits::GitHub(client), &identity, &previous).await?;

    if scan.changed {
        let published = commits::publish(&scan, now);
        if args.write {
            commits::write(&args.root, &published)
                .map_err(|error| CliError::Io(commits::state_path(&args.root).display().to_string(), error))?;
        } else {
            print!("{}", published.state);
        }
        published.report?;
    } else {
        print!("{}", ecosystem_domain::monitor::UNCHANGED);
    }
    eprintln!(
        "profile-core: monitor: {} repositórios, {} com mudança, {} erros; contadores {} = {} dos projetos + {} do monitor",
        scan.current.len(),
        scan.changes.len(),
        scan.errors.len(),
        scan.tracked_commits(),
        scan.project_commits,
        scan.monitor_commits
    );

    if let Some(database) = database {
        let (observations, skipped) = commits::transitions(&scan, &previous);
        for name in &skipped {
            eprintln!("profile-core: aviso: {name}: SHA fora do formato; transição não gravada no banco");
        }
        let record = store::activity::CommitScanRecord {
            owner: identity.user.clone(),
            scanned: scan.current.len(),
            observations,
            counters: scan.changed.then_some(store::activity::MonitorCounters {
                project: scan.project_commits,
                monitor: scan.monitor_commits,
                tracked: scan.tracked_commits(),
            }),
        };
        let report = database.record_commit_scan(&run_context(&args.trigger), &record).await?;
        eprintln!(
            "profile-core: histórico: {} transições gravadas (sync_run {}); {} repositórios sem registro",
            report.recorded,
            report.sync_run_id,
            report.unregistered.len()
        );
    }
    Ok(ExitCode::SUCCESS)
}

async fn sync_github(args: GithubArgs) -> Result<ExitCode, CliError> {
    let env = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
    let inventory_token = env("PROFILE_GITHUB_TOKEN");
    let database = if args.no_db {
        None
    } else {
        let url = args.database_url.as_deref().ok_or(CliError::NoDatabase("sync github"))?;
        if args.input_repos.is_none() && inventory_token.is_none() {
            return Err(CliError::IncompleteInventory);
        }
        Some(Database::from_url(url)?)
    };
    let options = SyncOptions {
        root: args.root,
        input_repos: args.input_repos,
        languages_dir: args.languages_dir,
        classifier: classifier(&args.classifier),
    };
    let api_token = inventory_token.clone().or_else(|| env("GITHUB_TOKEN"));
    let collected = inventory_sync::collect(&options, inventory_token, api_token).await?;
    let summary = inventory_sync::summary(&collected);
    let Some(database) = database else {
        println!("{summary}");
        return Ok(ExitCode::SUCCESS);
    };
    let report = database.record_inventory(&run_context(&args.trigger), &collected.record).await?;
    eprintln!(
        "profile-core: inventário: {} repositórios ({} novos, {} alterados, {} sumidos); {} mapas de linguagem \
         trocados; {} projetos criados (sync_run {})",
        collected.record.repos.len(),
        report.inserted,
        report.updated,
        report.gone,
        report.languages_changed,
        report.projects_created,
        report.sync_run_id
    );
    Ok(ExitCode::SUCCESS)
}

async fn sync_contributions(args: ContributionsArgs) -> Result<ExitCode, CliError> {
    let env = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
    let now = match &args.now {
        Some(value) => catalog::parse_now(value)?,
        None => chrono::Utc::now(),
    };
    let token = env("PROFILE_README_TOKEN").or_else(|| env("GITHUB_TOKEN")).ok_or(CliError::NoGraphQlToken)?;
    let database = if args.no_db {
        None
    } else {
        Some(Database::from_url(args.database_url.as_deref().ok_or(CliError::NoDatabase("sync contributions"))?)?)
    };
    let login = env("PROFILE_LOGIN").unwrap_or_else(|| "Lucas-Belucci-Bellini".into());
    let graphql = contributions::GitHub::new(token)?;
    let payload = contributions::collect(&graphql, &login, now).await.map_err(CliError::Timeline)?;
    let rows = payload["rows"].as_array().map_or(0, Vec::len);
    if args.write {
        contributions::write(&args.root, &payload)
            .map_err(|error| CliError::Io(args.root.join(contributions::DATA_FILE).display().to_string(), error))?;
        println!("timeline updated: rows={rows} html={}", contributions::HTML_FILE);
    } else {
        print!("{}", contributions::render_data(&payload));
    }
    if let Some(database) = database {
        let (samples, skipped) = contributions::samples(&payload);
        for reason in &skipped {
            eprintln!("profile-core: aviso: {reason}; amostra não gravada");
        }
        let report = database.record_samples("contributions", &run_context(&args.trigger), &samples).await?;
        eprintln!(
            "profile-core: histórico: {} de {} amostras mudaram e foram gravadas (sync_run {})",
            report.recorded,
            samples.len(),
            report.sync_run_id
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// `sync all`: os passos em ordem, parando no primeiro erro. Sem banco
/// (`--no-db`), os manifestos não são importados — eles só vão para o banco.
async fn sync_all(args: SyncAllArgs) -> Result<ExitCode, CliError> {
    let SyncAllArgs { root, now, write, database_url, no_db, trigger } = args;
    if !no_db && database_url.is_none() {
        return Err(CliError::NoDatabase("sync all"));
    }
    let step = |name: &str| eprintln!("profile-core: sync all: {name}");

    step("github");
    sync_github(GithubArgs {
        root: root.clone(),
        input_repos: None,
        languages_dir: None,
        database_url: database_url.clone(),
        no_db,
        trigger: trigger.clone(),
        classifier: PY_V1.into(),
    })
    .await?;

    match &database_url {
        Some(url) if !no_db => {
            step("manifestos");
            import_manifests(ImportArgs {
                root: root.clone(),
                connection: Connection { database_url: url.clone() },
                trigger: trigger.clone(),
            })
            .await?;
        }
        _ => eprintln!("profile-core: sync all: manifestos — pulado (sem banco, não há onde importar)"),
    }

    step("commits");
    sync_commits(CommitsArgs {
        root: root.clone(),
        now: now.clone(),
        write,
        database_url: database_url.clone(),
        no_db,
        trigger: trigger.clone(),
    })
    .await?;

    step("contribuições");
    sync_contributions(ContributionsArgs {
        root: root.clone(),
        now,
        write,
        database_url: database_url.clone(),
        no_db,
        trigger: trigger.clone(),
    })
    .await?;

    step("sites");
    check_sites(SitesArgs {
        json: false,
        fail_on_down: false,
        timeout: 15.0,
        retries: 1,
        max_workers: 6,
        root,
        database_url,
        no_db,
        trigger,
    })
    .await
}

async fn import_manifests(args: ImportArgs) -> Result<ExitCode, CliError> {
    let database = Database::from_url(&args.connection.database_url)?;
    let (manifests, ignored) = imports::read_manifests(&args.root)?;
    for reason in &ignored {
        eprintln!("profile-core: aviso: {reason}");
    }
    let report = database.import_manifests(&run_context(&args.trigger), &manifests).await?;
    for reason in &report.skipped_sites {
        eprintln!("profile-core: aviso: site manual ignorado: {reason}");
    }
    println!(
        "manifestos importados (sync_run {}): exclusões {} (−{}), curadoria {} (−{}), arsenal {} (−{}), \
         sites manuais {} ativos ({} aposentados)",
        report.sync_run_id,
        report.exclusions.0,
        report.exclusions.1,
        report.featured.0,
        report.featured.1,
        report.tools.0,
        report.tools.1,
        report.sites.0,
        report.sites.1
    );
    Ok(ExitCode::SUCCESS)
}

async fn import_legacy(args: ImportArgs) -> Result<ExitCode, CliError> {
    let database = Database::from_url(&args.connection.database_url)?;
    let owner = std::env::var("GH_USER").unwrap_or_else(|_| catalog::OWNER.into());
    let record = imports::read_legacy(&args.root, &owner)?;
    let report = database.import_legacy(&run_context(&args.trigger), &record).await?;
    for reason in &report.skipped {
        eprintln!("profile-core: aviso: não importado: {reason}");
    }
    let verified = record.sites.iter().filter(|site| site.outcome == "verified").count();
    println!("carga legada (sync_run {}):", report.sync_run_id);
    println!("  resumos editoriais        {} gravados de {}", report.summaries, record.summaries.len());
    println!("  sobreposições de status   {} gravadas", report.overrides);
    println!(
        "  sites do catálogo         {} criados, {} checagens iniciais ({} no ar no catálogo)",
        report.sites_created, report.checks, verified
    );
    println!("  estado do monitor         {} de {} repositórios", report.heads, record.heads.len());
    println!("  amostras importadas       {} de {}", report.samples, record.samples.len());
    Ok(ExitCode::SUCCESS)
}

/// Imprime o resultado de um validador e devolve o código de saída: o do
/// script; `1` onde ele sairia com traceback; `2` para o valor JSON que o
/// Python aceita e o Rust não representa.
fn report(label: &str, result: Result<Outcome, Crash>) -> u8 {
    match result {
        Ok(outcome) => {
            print!("{}", outcome.stdout);
            eprint!("{}", outcome.stderr);
            outcome.code
        }
        Err(crash) => {
            eprintln!("profile-core: erro: validate {label}: {}: {}", crash.kind, crash.message);
            if crash.kind == "Unsupported" { 2 } else { 1 }
        }
    }
}

/// Os arquivos que o `update-profile.yml` exige não vazios.
fn generated_files(root: &std::path::Path) -> Outcome {
    const FILES: [&str; 4] =
        ["README.md", "assets/lang-stats.svg", "assets/profile-snapshot.svg", "docs/project-catalog.json"];
    let empty: Vec<&str> = FILES
        .into_iter()
        .filter(|file| std::fs::metadata(root.join(file)).map(|meta| meta.len()).unwrap_or(0) == 0)
        .collect();
    if empty.is_empty() {
        return Outcome {
            stdout: format!("generated files present: {}\n", FILES.len()),
            stderr: String::new(),
            code: 0,
        };
    }
    Outcome {
        stdout: String::new(),
        stderr: format!("generated files missing or empty: {}\n", empty.join(", ")),
        code: 1,
    }
}

async fn run_validate(command: ValidateCommand) -> ExitCode {
    let code = match command {
        ValidateCommand::Readme { before, after } => report("readme", validate::readme(&before, &after)),
        ValidateCommand::Exclusions(args) => report("exclusions", validate::exclusions(&args.root)),
        ValidateCommand::Links(args) => report("links", validate::links(&args.root)),
        ValidateCommand::Visual(args) => report("visual", validate::visual(&args.root)),
        ValidateCommand::Badges { root, offline } => report("badges", validate::badges(&root.root, !offline).await),
        ValidateCommand::Catalog(args) => {
            report("catalog", validate::catalog(&args.root).map(|errors| validate::catalog_outcome(&errors)))
        }
        ValidateCommand::All { root, before, online } => {
            let root = root.root;
            let codes = [
                report("readme", validate::readme(&before, &root.join("README.md"))),
                report("exclusions", validate::exclusions(&root)),
                report("links", validate::links(&root)),
                report("visual", validate::visual(&root)),
                report("badges", validate::badges(&root, online).await),
                report("catalog", validate::catalog(&root).map(|errors| validate::catalog_outcome(&errors))),
                report("files", Ok(generated_files(&root))),
            ];
            codes.into_iter().max().unwrap_or(0)
        }
    };
    ExitCode::from(code)
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
