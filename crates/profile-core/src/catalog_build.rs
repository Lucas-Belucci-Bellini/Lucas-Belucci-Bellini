//! `profile-core catalog build` — o caminho do `update_profile.py` até o
//! `docs/project-catalog.json`, sem o README.
//!
//! Mesmas entradas e mesmas regras do Python: inventário do GitHub (ou
//! `--input-repos`), exclusões, descoberta de site, verificação (HTTP de
//! verdade, `--site-checks-fixture` ou `--skip-site-check`), curadoria e
//! relógio (`--now`). O arquivo sai byte a byte igual ao do Python — provado
//! pelo golden (`crates/catalog/tests/golden.rs`) e pelo teste de ponta a
//! ponta contra um GitHub simulado (`tests/e2e/github_parity.py`).
//!
//! Sem `--write`, o catálogo vai para a saída padrão e nada é gravado.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

use catalog::{CatalogError, Curadoria, DiscoveredSite, inventory};
use chrono::{DateTime, NaiveDateTime, Utc};
use ecosystem_domain::presentation::{CheckStatus, Presentation, WebsiteCheck};
use ecosystem_domain::repo::RepoFacts;
use github_client::{ApiError, Client, Settings};
use profile_render::{LanguageMap, RenderError};
use serde_json::{Map, Value, json};

/// De onde vêm os resultados de verificação dos sites.
#[derive(Debug, Clone)]
pub enum ChecksSource {
    /// HTTP de verdade, com o `site-monitor` (paridade da Fase 2).
    Live {
        /// Timeout de cada conexão e leitura.
        timeout: Duration,
        /// Verificações simultâneas.
        workers: usize,
    },
    /// Arquivo no formato de `--site-checks-fixture` do Python.
    Fixture(PathBuf),
    /// Nenhuma verificação: todo site sai como não verificado.
    Skip,
}

/// O que o comando recebeu.
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Raiz com `docs/` (manifestos e catálogo).
    pub root: PathBuf,
    /// Inventário local em vez do GitHub.
    pub input_repos: Option<PathBuf>,
    /// Linguagens de arquivo (`owner__nome.json`), com `input_repos`.
    pub languages_dir: Option<PathBuf>,
    /// Verificação dos sites.
    pub checks: ChecksSource,
    /// Relógio fixo.
    pub now: Option<String>,
    /// Grava `docs/project-catalog.json` (só se mudou).
    pub write: bool,
    /// Grava também os resultados de verificação usados, no formato do fixture.
    pub checks_out: Option<PathBuf>,
}

/// Falhas do comando. As mensagens são as do Python (sem o prefixo
/// `profile refresh failed:`).
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// Manifesto, fixture, relógio ou inventário local inválido.
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    /// A API do GitHub falhou.
    #[error(transparent)]
    GitHub(#[from] ApiError),
    /// Cliente HTTP não montou.
    #[error(transparent)]
    Client(#[from] github_client::BuildError),
    /// Cliente do monitor não montou.
    #[error(transparent)]
    Monitor(#[from] site_monitor::check::BuildError),
    /// `--write` sem inventário completo apagaria os privados do catálogo.
    #[error("refusing write mode without PROFILE_GITHUB_TOKEN; this could erase private-project entries")]
    NoToken,
    /// Inventário vazio depois das exclusões.
    #[error("GitHub returned no repositories")]
    Empty,
    /// Falha fora do `try` do `main()` do Python — lá, traceback e código 1:
    /// curadoria ou arsenal ilegível ou com formato errado.
    #[error("{}", traceback_tail(.0))]
    Unguarded(CatalogError),
    /// Bloco do README que o Python não conseguiria montar (código 1).
    #[error(transparent)]
    Render(#[from] RenderError),
    /// Falha de disco lendo o README ou gravando as saídas (código 1 no Python).
    #[error("{}: {path}: {source}", os_error_name(source))]
    Disk {
        /// Arquivo.
        path: String,
        /// Erro.
        source: std::io::Error,
    },
}

impl BuildError {
    /// Onde o Python sairia com traceback (código 1) em vez de
    /// `profile refresh failed:` (código 2).
    pub fn is_python_crash(&self) -> bool {
        matches!(
            self,
            Self::Unguarded(_) | Self::Render(_) | Self::Disk { .. } | Self::Catalog(CatalogError::PythonCrash { .. })
        )
    }

    pub(crate) fn disk(path: &Path) -> impl FnOnce(std::io::Error) -> Self + '_ {
        move |source| Self::Disk { path: path.display().to_string(), source }
    }
}

/// A exceção que o Python levantaria para uma falha de disco.
fn os_error_name(error: &std::io::Error) -> &'static str {
    match error.kind() {
        std::io::ErrorKind::NotFound => "FileNotFoundError",
        std::io::ErrorKind::PermissionDenied => "PermissionError",
        std::io::ErrorKind::IsADirectory => "IsADirectoryError",
        std::io::ErrorKind::NotADirectory => "NotADirectoryError",
        _ => "OSError",
    }
}

/// A última linha do traceback do Python: `load_json_object` levanta `ValueError`.
fn traceback_tail(error: &CatalogError) -> String {
    match error {
        CatalogError::PythonCrash { .. } => error.to_string(),
        other => format!("ValueError: {other}"),
    }
}

/// Os tokens, como o `build_data()` do Python os escolhe.
#[derive(Debug, Clone, Default)]
pub struct Tokens {
    /// `PROFILE_GITHUB_TOKEN`: o inventário com os privados.
    pub inventory: Option<String>,
    /// O das linguagens: o de inventário ou, sem ele, o `GITHUB_TOKEN`.
    pub api: Option<String>,
}

impl Tokens {
    /// Lê do ambiente; valor vazio conta como ausente.
    pub fn from_env() -> Self {
        let env = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
        let inventory = env("PROFILE_GITHUB_TOKEN");
        Self { api: inventory.clone().or_else(|| env("GITHUB_TOKEN")), inventory }
    }
}

/// Para que o preparo serve: o catálogo sozinho não precisa das linguagens
/// nem do arsenal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// `catalog build`.
    Catalog,
    /// `render readme`: linguagens e `README_STACK.json` também.
    Readme,
}

/// Tudo o que o `main()` do Python tem em mãos antes de montar o README.
#[derive(Debug)]
pub struct Prepared {
    /// Inventário depois das exclusões.
    pub repos: Vec<RepoFacts>,
    /// Exclusões editoriais.
    pub excluded: BTreeSet<String>,
    /// Linguagens por `full_name` (vazio para [`Purpose::Catalog`]).
    pub languages: LanguageMap,
    /// Relógio.
    pub now: DateTime<Utc>,
    /// O mesmo relógio no fuso em que foi dado.
    pub now_local: NaiveDateTime,
    /// `README_FEATURED.json`.
    pub featured: Map<String, Value>,
    /// `README_STACK.json` (só para [`Purpose::Readme`]).
    pub stack: Option<Map<String, Value>>,
    /// Sites descobertos (públicos).
    pub sites: Vec<DiscoveredSite>,
    /// Verificações usadas.
    pub checks: HashMap<String, WebsiteCheck>,
    /// Uma apresentação por `full_name`.
    pub presentations: Vec<Presentation>,
}

/// O caminho do `main()` do Python até `build_presentations()`, na mesma
/// ordem (é ela que decide qual erro aparece primeiro).
pub async fn prepare(options: &BuildOptions, tokens: &Tokens, purpose: Purpose) -> Result<Prepared, BuildError> {
    // build_data(): o que falha aqui sai com código 2.
    let inventory_token = tokens.inventory.clone().filter(|t| !t.is_empty());
    if options.write && options.input_repos.is_none() && inventory_token.is_none() {
        return Err(BuildError::NoToken);
    }
    let excluded = catalog::load_excluded(&options.root)?;
    let client = Client::new(Settings::new(inventory::USER_AGENT))?;
    let raw = match &options.input_repos {
        Some(path) => inventory::load_local_repositories(path)?,
        None => inventory::fetch_repositories(&client.with_token(inventory_token)).await?,
    };
    let repos = catalog::exclude(inventory::facts(&raw)?, &excluded);
    let mut languages = LanguageMap::new();
    if purpose == Purpose::Readme {
        match (&options.input_repos, &options.languages_dir) {
            (Some(_), Some(dir)) => {
                for repo in &repos {
                    languages.insert(repo.full_name.clone(), inventory::load_local_languages(dir, &repo.full_name));
                }
            }
            (Some(_), None) => {
                for repo in &repos {
                    languages.insert(repo.full_name.clone(), Vec::new());
                }
            }
            (None, _) => {
                let client = client.with_token(tokens.api.clone().filter(|t| !t.is_empty()));
                for repo in &repos {
                    let map = inventory::fetch_languages(&client, &repo.full_name).await;
                    languages.insert(repo.full_name.clone(), map);
                }
            }
        }
    }
    if repos.is_empty() {
        return Err(BuildError::Empty);
    }
    let (now, now_local) = match &options.now {
        Some(value) => catalog::parse_now_local(value)?,
        None => {
            let now = Utc::now();
            (now, now.naive_utc())
        }
    };

    // Daqui em diante, fora do `try` do Python: manifesto ruim é traceback.
    let overrides = catalog::load_site_overrides(&options.root);
    let featured_path = options.root.join(catalog::FEATURED_FILE);
    let featured = catalog::load_json_object(&featured_path).map_err(BuildError::Unguarded)?;
    let stack = match purpose {
        Purpose::Readme => {
            Some(catalog::load_json_object(&options.root.join(STACK_FILE)).map_err(BuildError::Unguarded)?)
        }
        Purpose::Catalog => None,
    };
    let sites = catalog::discover(&repos, &overrides);
    let candidates = catalog::candidates(&sites);
    let checks = match &options.checks {
        ChecksSource::Fixture(path) => catalog::load_checks_fixture(path, &candidates)?,
        ChecksSource::Skip => HashMap::new(),
        ChecksSource::Live { timeout, workers } => live_checks(&candidates, *timeout, *workers).await?,
    };
    if let Some(path) = &options.checks_out {
        write_checks(path, &sites, &checks)?;
    }
    let curadoria =
        Curadoria::from_manifest(&featured, &featured_path.display().to_string()).map_err(BuildError::Unguarded)?;
    let presentations =
        catalog::presentations(&repos, &sites, &checks, &curadoria, now).map_err(BuildError::Unguarded)?;
    Ok(Prepared { repos, excluded, languages, now, now_local, featured, stack, sites, checks, presentations })
}

/// Arsenal da vitrine, relativo à raiz.
pub const STACK_FILE: &str = "docs/README_STACK.json";

/// Resultado de uma execução.
#[derive(Debug)]
pub struct Built {
    /// O catálogo, como é gravado.
    pub text: String,
    /// O catálogo, como valor.
    pub catalog: Value,
    /// `Some(gravou?)` com `--write`.
    pub written: Option<bool>,
}

/// Roda o comando.
pub async fn run(options: &BuildOptions, tokens: &Tokens) -> Result<Built, BuildError> {
    let prepared = prepare(options, tokens, Purpose::Catalog).await?;
    let value = catalog::build(&prepared.presentations);
    let text = catalog::render(&value);
    let written = if options.write {
        Some(catalog::write_if_changed(&text, &catalog::catalog_path(&options.root))?)
    } else {
        None
    };
    Ok(Built { text, catalog: value, written })
}

async fn live_checks(
    urls: &[String],
    timeout: Duration,
    workers: usize,
) -> Result<HashMap<String, WebsiteCheck>, BuildError> {
    let checker = site_monitor::Checker::new(site_monitor::Options { timeout, retries: 1, max_workers: workers })?;
    let results = checker.check_websites(urls.iter().cloned()).await;
    for check in results.values().filter(|check| check.python_crash.is_some()) {
        eprintln!(
            "profile-core: aviso: {:?} derrubaria o update_profile.py ({}); aqui conta como fora do ar",
            check.url,
            check.python_crash.unwrap_or_default()
        );
    }
    Ok(results
        .into_iter()
        .map(|(url, check)| {
            let status = match check.status {
                site_monitor::Status::Verified => CheckStatus::Verified,
                site_monitor::Status::Unreachable => CheckStatus::Unreachable,
                site_monitor::Status::Invalid => CheckStatus::Invalid,
            };
            let converted = WebsiteCheck {
                url: url.clone(),
                status,
                http_status: i64::from(check.http_status),
                final_url: check.final_url,
            };
            (url, converted)
        })
        .collect())
}

/// Os resultados usados, no formato de `--site-checks-fixture`: o modo sombra
/// entrega ao Python as mesmas checagens, e a comparação fica só no catálogo.
fn write_checks(
    path: &Path,
    sites: &[DiscoveredSite],
    checks: &HashMap<String, WebsiteCheck>,
) -> Result<(), BuildError> {
    let mut out = Map::new();
    for url in catalog::candidates(sites) {
        if let Some(check) = checks.get(&url) {
            out.insert(
                url,
                json!({"status": check.status.as_str(), "http_status": check.http_status, "final_url": check.final_url}),
            );
        }
    }
    let text = catalog::render(&Value::Object(out));
    std::fs::write(path, text).map_err(|source| CatalogError::Io { path: path.display().to_string(), source })?;
    Ok(())
}

/// Resumo de uma linha para o stderr.
pub fn summary(built: &Built) -> String {
    let counts = &built.catalog["counts"];
    let state = match built.written {
        Some(true) => "; gravado",
        Some(false) => "; sem mudança",
        None => "",
    };
    format!(
        "catálogo: {} projetos ({} públicos, {} privados), {} com site no ar{state}",
        counts["projects"], counts["public"], counts["private"], counts["with_live_website"]
    )
}
