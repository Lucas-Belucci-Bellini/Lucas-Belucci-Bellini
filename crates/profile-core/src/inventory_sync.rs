//! `profile-core sync github` — o inventário do GitHub no banco.
//!
//! Lê o inventário como o `update_profile.py` (token de inventário →
//! `/user/repos`, privados inclusive; ou `--input-repos`), tira os excluídos
//! (D-019: não são armazenados), consulta as linguagens e grava donos,
//! repositórios, linguagens e o projeto 1:1 de cada repositório.
//!
//! Gravar exige inventário **completo** (token ou arquivo): com um inventário
//! só de públicos, os privados pareceriam sumidos. Sem banco (`--no-db`), só
//! relata o que leu.

use std::path::PathBuf;

use catalog::{CatalogError, inventory};
use ecosystem_domain::classify::{PY_V1, classify_py_v1};
use ecosystem_domain::slug::slug;
use ecosystem_domain::timestamps::parse_github_timestamp;
use github_client::{ApiError, Client, Settings};
use serde_json::{Value, json};
use store::inventory::{InventoryRecord, OwnerRecord, RepoRecord};

/// O que o comando recebeu.
#[derive(Debug, Clone)]
pub struct SyncOptions {
    /// Raiz com `docs/README_EXCLUDED.json`.
    pub root: PathBuf,
    /// Inventário de arquivo.
    pub input_repos: Option<PathBuf>,
    /// Linguagens de arquivo (`owner__nome.json`).
    pub languages_dir: Option<PathBuf>,
}

/// Falhas.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// Manifesto ou inventário local inválido.
    #[error(transparent)]
    Catalog(#[from] CatalogError),
    /// A API falhou.
    #[error(transparent)]
    GitHub(#[from] ApiError),
    /// Cliente HTTP.
    #[error(transparent)]
    Client(#[from] github_client::BuildError),
    /// Inventário vazio depois das exclusões.
    #[error("GitHub returned no repositories")]
    Empty,
    /// Um repositório sem o que o banco exige.
    #[error("repositório {0}: {1}")]
    Invalid(String, &'static str),
}

/// O inventário lido.
#[derive(Debug)]
pub struct Collected {
    /// Pronto para o banco.
    pub record: InventoryRecord,
    /// Excluídos pelo manifesto que o GitHub listou.
    pub excluded: Vec<String>,
}

/// Lê o inventário e as linguagens. `inventory_token` é o
/// `PROFILE_GITHUB_TOKEN`; `api_token`, o que vale para as linguagens (ele ou
/// o `GITHUB_TOKEN`), como no Python.
pub async fn collect(
    options: &SyncOptions,
    inventory_token: Option<String>,
    api_token: Option<String>,
) -> Result<Collected, SyncError> {
    let excluded_names = catalog::load_excluded(&options.root)?;
    let client = Client::new(Settings::new(inventory::USER_AGENT))?;
    let inventory_token = inventory_token.filter(|t| !t.is_empty());
    let complete = options.input_repos.is_some() || inventory_token.is_some();
    let raw = match &options.input_repos {
        Some(path) => inventory::load_local_repositories(path)?,
        None => inventory::fetch_repositories(&client.with_token(inventory_token)).await?,
    };
    let seen: Vec<i64> = raw.iter().filter_map(|repo| repo.get("id").and_then(Value::as_i64)).collect();
    let facts = inventory::facts(&raw)?;
    let mut excluded = Vec::new();
    let mut kept = Vec::new();
    for (repo, fact) in raw.iter().zip(&facts) {
        if excluded_names.contains(&fact.name) || excluded_names.contains(&fact.full_name) {
            excluded.push(fact.full_name.clone());
        } else {
            kept.push((repo, fact));
        }
    }
    if kept.is_empty() {
        return Err(SyncError::Empty);
    }

    let languages_client = client.with_token(api_token);
    let mut repos = Vec::with_capacity(kept.len());
    for (repo, fact) in kept {
        let languages = match (&options.input_repos, &options.languages_dir) {
            (Some(_), Some(dir)) => inventory::try_load_local_languages(dir, &fact.full_name),
            (Some(_), None) => None,
            (None, _) => inventory::try_fetch_languages(&languages_client, &fact.full_name).await,
        };
        repos.push(record(repo, fact, languages)?);
    }
    excluded.sort();
    Ok(Collected { record: InventoryRecord { repos, seen, complete }, excluded })
}

fn record(
    repo: &Value,
    fact: &ecosystem_domain::repo::RepoFacts,
    languages: Option<Vec<(String, i64)>>,
) -> Result<RepoRecord, SyncError> {
    let invalid = |why| SyncError::Invalid(fact.full_name.clone(), why);
    let text = |key: &str| repo.get(key).and_then(Value::as_str).filter(|v| !v.is_empty()).map(str::to_string);
    let flag = |key: &str| repo.get(key).and_then(Value::as_bool).unwrap_or(false);
    let timestamp = |key: &str| text(key).and_then(|v| parse_github_timestamp(&v)).map(|t| t.to_rfc3339());
    let owner = repo.get("owner").ok_or_else(|| invalid("sem owner"))?;
    let owner = OwnerRecord {
        github_id: owner.get("id").and_then(Value::as_i64).ok_or_else(|| invalid("owner sem id"))?,
        login: owner.get("login").and_then(Value::as_str).ok_or_else(|| invalid("owner sem login"))?.into(),
        kind: owner.get("type").and_then(Value::as_str).unwrap_or("User").into(),
    };
    let visibility = match repo.get("visibility").and_then(Value::as_str) {
        Some(v @ ("public" | "private" | "internal")) => v.to_string(),
        _ if fact.private => "private".into(),
        _ => "public".into(),
    };
    let label = classify_py_v1(fact);
    Ok(RepoRecord {
        github_id: repo.get("id").and_then(Value::as_i64).ok_or_else(|| invalid("sem id do GitHub"))?,
        node_id: text("node_id"),
        slug_candidates: vec![slug(&fact.name), slug(&format!("{}-{}", owner.login, fact.name))],
        owner,
        name: fact.name.clone(),
        full_name: fact.full_name.clone(),
        description: fact.description.clone(),
        visibility,
        fork: fact.fork,
        archived: fact.archived,
        template: flag("is_template"),
        default_branch: text("default_branch"),
        primary_language: text("language"),
        homepage: text("homepage"),
        homepage_site: match ecosystem_domain::discovery::discover_project_website(fact, &Default::default()) {
            (Some(url), ecosystem_domain::discovery::WebsiteSource::GithubHomepage) if !fact.private => Some(url),
            _ => None,
        },
        topics: repo
            .get("topics")
            .and_then(Value::as_array)
            .map(|topics| topics.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default(),
        size_kb: fact.size.and_then(|size| i32::try_from(size).ok()),
        created_at: timestamp("created_at"),
        updated_at: timestamp("updated_at"),
        pushed_at: timestamp("pushed_at"),
        languages,
        label: label.to_string(),
        classifier_version: PY_V1.to_string(),
    })
}

/// Resumo em JSON do que foi lido (a saída de `--no-db`).
pub fn summary(collected: &Collected) -> Value {
    let repos = &collected.record.repos;
    let mut languages: Vec<&str> =
        repos.iter().flat_map(|r| r.languages.iter().flatten().map(|(l, _)| l.as_str())).collect();
    languages.sort_unstable();
    languages.dedup();
    json!({
        "repositories": repos.len(),
        "public": repos.iter().filter(|r| r.visibility == "public").count(),
        "private": repos.iter().filter(|r| r.visibility != "public").count(),
        "excluded": collected.excluded,
        "languages": languages.len(),
        "complete": collected.record.complete,
    })
}
