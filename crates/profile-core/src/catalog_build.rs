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

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use catalog::{CatalogError, Curadoria, DiscoveredSite, inventory};
use ecosystem_domain::presentation::{CheckStatus, WebsiteCheck};
use github_client::{ApiError, Client, Settings};
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
}

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

/// Roda o comando. `token` é o `PROFILE_GITHUB_TOKEN`.
pub async fn run(options: &BuildOptions, token: Option<String>) -> Result<Built, BuildError> {
    let token = token.filter(|t| !t.is_empty());
    if options.write && options.input_repos.is_none() && token.is_none() {
        return Err(BuildError::NoToken);
    }
    let now = match &options.now {
        Some(value) => catalog::parse_now(value)?,
        None => chrono::Utc::now(),
    };
    let excluded = catalog::load_excluded(&options.root)?;
    let raw = match &options.input_repos {
        Some(path) => inventory::load_local_repositories(path)?,
        None => {
            let client = Client::new(Settings::new(inventory::USER_AGENT))?.with_token(token);
            inventory::fetch_repositories(&client).await?
        }
    };
    let repos = catalog::exclude(inventory::facts(&raw)?, &excluded);
    if repos.is_empty() {
        return Err(BuildError::Empty);
    }

    let overrides = catalog::load_site_overrides(&options.root);
    let curadoria = Curadoria::load(&options.root)?;
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

    let presentations = catalog::presentations(&repos, &sites, &checks, &curadoria, now)?;
    let value = catalog::build(&presentations);
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
