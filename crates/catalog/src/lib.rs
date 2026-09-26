//! Inventário → `docs/project-catalog.json` (schema `project-catalog@1`).
//!
//! Reproduz o caminho do `scripts/update_profile.py` até o catálogo:
//!
//! ```text
//! inventário ─▶ exclusões ─▶ descoberta de site ─▶ verificação ─▶ apresentação ─▶ JSON
//!               README_EXCLUDED   homepage → README_SITES          README_FEATURED
//! ```
//!
//! As regras (classificação, status, prioridade, CTA) estão em
//! `ecosystem-domain` e têm paridade provada caso a caso; este crate só as
//! encadeia na mesma ordem e escreve o arquivo com os mesmos bytes que o
//! `json.dumps(..., ensure_ascii=False, indent=2)` do Python. A prova é
//! `tests/golden.rs`, sobre a mesma raiz sintética do teste golden do Python
//! (`tests/fixtures/profile`).
//!
//! A leitura do GitHub está em [`inventory`].

pub mod inventory;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDateTime, Utc};
use ecosystem_domain::classify::classify_py_v1;
use ecosystem_domain::discovery::{
    WebsiteSource, discover_project_website, json_truthy, normalize_site_overrides, py_str,
};
use ecosystem_domain::lifecycle::status_py_v1;
use ecosystem_domain::presentation::{
    CheckStatus, Presentation, WebsiteCheck, catalog_entry, describe, resolve_presentation,
};
use ecosystem_domain::pyjson::py_int;
use ecosystem_domain::repo::RepoFacts;
use ecosystem_domain::taxonomy::CATEGORIES;
use ecosystem_domain::timestamps::{PyLocalDatetime, py_fromisoformat_local};
use serde_json::{Map, Value, json};

/// Dono do perfil (`OWNER` do `update_profile.py`).
pub const OWNER: &str = "Lucas-Belucci-Bellini";
/// `schema` do arquivo.
pub const SCHEMA: &str = "lucas-belucci-bellini/project-catalog@1";
/// `note` do arquivo — o texto do Python, que continua sendo o gerador oficial.
pub const NOTE: &str = "Gerado por scripts/update_profile.py. Não editar à mão: ajuste \
docs/README_SITES.json, docs/README_FEATURED.json ou a lógica do gerador.";
/// Onde o catálogo mora, relativo à raiz.
pub const CATALOG_FILE: &str = "docs/project-catalog.json";
/// Manifesto de sites.
pub const SITES_FILE: &str = "docs/README_SITES.json";
/// Curadoria da vitrine.
pub const FEATURED_FILE: &str = "docs/README_FEATURED.json";
/// Exclusões editoriais.
pub const EXCLUDED_FILE: &str = "docs/README_EXCLUDED.json";

/// Falha que o Python também teria — a mensagem é a mesma, sem o prefixo
/// `profile refresh failed:` que o chamador põe.
#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    /// Manifesto ilegível ou que não é objeto (`load_json_object`).
    #[error("{0}")]
    Manifest(String),
    /// Entrada inválida (inventário local, fixture de checagens, relógio).
    #[error("{0}")]
    Input(String),
    /// Onde o Python quebraria com traceback (`int()` de um `order` inválido,
    /// curadoria que não é lista de objetos): aqui é erro com o nome da exceção.
    #[error("{kind} em {path}: {detail}")]
    PythonCrash {
        /// Exceção que o Python levantaria.
        kind: &'static str,
        /// Arquivo de origem.
        path: String,
        /// O que estava errado.
        detail: String,
    },
    /// Falha de disco ao gravar.
    #[error("{path}: {source}")]
    Io {
        /// Arquivo.
        path: String,
        /// Erro.
        source: std::io::Error,
    },
}

/// `load_json_object()`: o arquivo precisa ser JSON e objeto.
pub fn load_json_object(path: &Path) -> Result<Map<String, Value>, CatalogError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| CatalogError::Manifest(format!("invalid JSON manifest: {}: {error}", path.display())))?;
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(object)) => Ok(object),
        Ok(_) => Err(CatalogError::Manifest(format!("JSON manifest must be an object: {}", path.display()))),
        Err(error) => Err(CatalogError::Manifest(format!("invalid JSON manifest: {}: {error}", path.display()))),
    }
}

/// `load_excluded_names()`: nomes curtos ou `owner/nome`, como no manifesto.
pub fn load_excluded(root: &Path) -> Result<BTreeSet<String>, CatalogError> {
    let path = root.join(EXCLUDED_FILE);
    let data = load_json_object(&path)?;
    match data.get("repositories") {
        None => Ok(BTreeSet::new()),
        Some(Value::Array(values)) => Ok(values.iter().map(py_str).collect()),
        Some(_) => Err(CatalogError::Manifest(format!("repositories must be a list: {}", path.display()))),
    }
}

/// `load_site_overrides()`: arquivo ausente ou ilegível é manifesto vazio.
pub fn load_site_overrides(root: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(root.join(SITES_FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .map(|raw| normalize_site_overrides(&raw))
        .unwrap_or_default()
}

/// Tira do inventário o que o manifesto exclui, por nome curto ou completo
/// (comparação exata, como no Python).
pub fn exclude(repos: Vec<RepoFacts>, excluded: &BTreeSet<String>) -> Vec<RepoFacts> {
    repos.into_iter().filter(|repo| !excluded.contains(&repo.name) && !excluded.contains(&repo.full_name)).collect()
}

/// A curadoria (`README_FEATURED.json`), indexada pelo `name` de cada entrada.
#[derive(Debug, Clone, Default)]
pub struct Curadoria {
    path: String,
    entries: HashMap<String, Map<String, Value>>,
}

impl Curadoria {
    /// Lê o manifesto (precisa ser objeto JSON).
    pub fn load(root: &Path) -> Result<Self, CatalogError> {
        let path = root.join(FEATURED_FILE);
        let manifest = load_json_object(&path)?;
        Self::from_manifest(&manifest, &path.display().to_string())
    }

    /// `{str(e.get("name", "")): e for e in manifest.get("projects", []) if e.get("name")}`.
    pub fn from_manifest(manifest: &Map<String, Value>, path: &str) -> Result<Self, CatalogError> {
        let crash = |kind, detail: &str| CatalogError::PythonCrash { kind, path: path.into(), detail: detail.into() };
        // O Python percorre o valor: objeto e texto vazios são listas vazias;
        // objeto e texto com conteúdo dão chaves e caracteres, `str` sem `.get`;
        // `null`, número e booleano não são iteráveis.
        let empty = Self { path: path.into(), entries: HashMap::new() };
        let projects = match manifest.get("projects") {
            None => return Ok(empty),
            Some(Value::Array(projects)) => projects,
            Some(Value::Object(map)) if map.is_empty() => return Ok(empty),
            Some(Value::String(text)) if text.is_empty() => return Ok(empty),
            Some(Value::Object(_) | Value::String(_)) => {
                return Err(crash("AttributeError", "\"projects\" não é uma lista de objetos"));
            }
            Some(_) => return Err(crash("TypeError", "\"projects\" não é iterável")),
        };
        let mut entries = HashMap::new();
        for entry in projects {
            let Value::Object(entry) = entry else {
                return Err(crash("AttributeError", "entrada de \"projects\" que não é objeto"));
            };
            if let Some(name) = entry.get("name").filter(|name| json_truthy(name)) {
                entries.insert(py_str(name), entry.clone());
            }
        }
        Ok(Self { path: path.into(), entries })
    }

    /// `featured_order()`: posição na curadoria (999 sem `order`), ou `None`
    /// fora dela. Um `order` que o `int()` recusa derrubaria o Python.
    pub fn order(&self, name: &str) -> Result<Option<i64>, CatalogError> {
        let Some(entry) = self.entries.get(name) else { return Ok(None) };
        match entry.get("order") {
            None => Ok(Some(999)),
            Some(value) => py_int(value).map(Some).map_err(|error| CatalogError::PythonCrash {
                kind: error.python_name(),
                path: self.path.clone(),
                detail: format!("\"order\" de {name:?} não é inteiro: {value}"),
            }),
        }
    }

    /// `featured_priority()`: `priority` explícita, quando for inteiro.
    pub fn priority(&self, name: &str) -> Option<i64> {
        let value = self.entries.get(name)?.get("priority")?;
        if value.is_null() {
            return None;
        }
        py_int(value).ok()
    }
}

/// Site descoberto de um repositório público.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredSite {
    /// `owner/nome`.
    pub full_name: String,
    /// URL declarada, se houver.
    pub url: Option<String>,
    /// De onde veio.
    pub source: WebsiteSource,
}

/// Descoberta para cada repositório **público** (privado nunca anuncia site),
/// na ordem do inventário. Repetido por `full_name` fica o último, na
/// posição do primeiro (o `dict` do Python).
pub fn discover(repos: &[RepoFacts], overrides: &BTreeMap<String, String>) -> Vec<DiscoveredSite> {
    let mut sites: Vec<DiscoveredSite> = Vec::new();
    for repo in repos.iter().filter(|repo| !repo.private) {
        let (url, source) = discover_project_website(repo, overrides);
        let site = DiscoveredSite { full_name: repo.full_name.clone(), url, source };
        match sites.iter_mut().find(|existing| existing.full_name == site.full_name) {
            Some(existing) => *existing = site,
            None => sites.push(site),
        }
    }
    sites
}

/// URLs a verificar, na ordem da descoberta (com repetições, como o Python).
pub fn candidates(sites: &[DiscoveredSite]) -> Vec<String> {
    sites.iter().filter_map(|site| site.url.clone()).collect()
}

/// `load_site_checks_fixture()`: resultados de checagem lidos de arquivo.
/// Toda URL descoberta precisa estar lá — uma URL sem resultado é erro.
pub fn load_checks_fixture(path: &Path, urls: &[String]) -> Result<HashMap<String, WebsiteCheck>, CatalogError> {
    let data = load_json_object(path)?;
    let missing: BTreeSet<&String> = urls.iter().filter(|url| !data.contains_key(url.as_str())).collect();
    if !missing.is_empty() {
        let list: Vec<&str> = missing.into_iter().map(String::as_str).collect();
        return Err(CatalogError::Input(format!("site check fixture has no result for: {}", list.join(", "))));
    }
    let mut checks = HashMap::new();
    for url in urls {
        let Value::Object(entry) = &data[url.as_str()] else {
            return Err(CatalogError::Input(format!("site check fixture entry is not an object: {url}")));
        };
        let status = entry.get("status").ok_or_else(|| CatalogError::Input("'status'".into()))?;
        let status = match py_str(status).as_str() {
            "verified" => CheckStatus::Verified,
            "unreachable" => CheckStatus::Unreachable,
            "invalid" => CheckStatus::Invalid,
            other => {
                return Err(CatalogError::Input(format!("invalid status in site check fixture for {url}: {other}")));
            }
        };
        let http_status = match entry.get("http_status") {
            None => 0,
            Some(value) => py_int(value).map_err(|error| {
                CatalogError::Input(format!("{}: http_status inválido para {url}", error.python_name()))
            })?,
        };
        let final_url = match entry.get("final_url") {
            Some(value) if json_truthy(value) => py_str(value),
            _ => url.clone(),
        };
        checks.insert(url.clone(), WebsiteCheck { url: url.clone(), status, http_status, final_url });
    }
    Ok(checks)
}

/// `parse_now()`: `--now` precisa de fuso.
pub fn parse_now(value: &str) -> Result<DateTime<Utc>, CatalogError> {
    parse_now_local(value).map(|(instant, _)| instant)
}

/// Como [`parse_now`], devolvendo também o relógio local (no fuso dado): é
/// o que o `strftime` do carimbo do README imprime quando não há data melhor.
pub fn parse_now_local(value: &str) -> Result<(DateTime<Utc>, NaiveDateTime), CatalogError> {
    match py_fromisoformat_local(&value.replace('Z', "+00:00")) {
        PyLocalDatetime::Aware { local, instant } => Ok((instant, local)),
        PyLocalDatetime::Naive(_) => {
            Err(CatalogError::Input("--now must include a timezone, e.g. 2026-09-25T12:00:00Z".into()))
        }
        PyLocalDatetime::Invalid => Err(CatalogError::Input(format!("Invalid isoformat string: {value:?}"))),
    }
}

/// `build_presentations()`: como cada repositório aparece na vitrine, na
/// ordem do inventário (repetido por `full_name` fica o último).
pub fn presentations(
    repos: &[RepoFacts],
    sites: &[DiscoveredSite],
    checks: &HashMap<String, WebsiteCheck>,
    curadoria: &Curadoria,
    now: DateTime<Utc>,
) -> Result<Vec<Presentation>, CatalogError> {
    let mut out: Vec<Presentation> = Vec::new();
    for repo in repos {
        let site = sites.iter().find(|site| site.full_name == repo.full_name);
        let url = site.and_then(|site| site.url.as_deref());
        let source = site.map_or(WebsiteSource::None, |site| site.source);
        let category = classify_py_v1(repo);
        let presentation = resolve_presentation(
            repo,
            url.and_then(|url| checks.get(url)),
            url,
            source.as_str(),
            category,
            status_py_v1(repo, category, now),
            &describe(repo.description.as_deref()),
            curadoria.order(&repo.name)?,
            curadoria.priority(&repo.name),
        );
        match out.iter_mut().find(|existing| existing.repository == presentation.repository) {
            Some(existing) => *existing = presentation,
            None => out.push(presentation),
        }
    }
    Ok(out)
}

/// `build_catalog()`: prioridade decrescente e, no empate, nome sem caixa —
/// nunca data. Os mesmos dados produzem o mesmo arquivo.
pub fn build(presentations: &[Presentation]) -> Value {
    let mut ordered: Vec<&Presentation> = presentations.iter().collect();
    ordered.sort_by(|a, b| {
        b.marketing_priority.cmp(&a.marketing_priority).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    let count = |keep: fn(&Presentation) -> bool| ordered.iter().filter(|p| keep(p)).count();
    json!({
        "schema": SCHEMA,
        "note": NOTE,
        "counts": {
            "projects": ordered.len(),
            "public": count(|p| !p.private),
            "private": count(|p| p.private),
            "with_live_website": count(Presentation::has_live_website),
        },
        "categories": CATEGORIES,
        "projects": ordered.iter().map(|p| catalog_entry(p)).collect::<Vec<_>>(),
    })
}

/// `json.dumps(catalog, ensure_ascii=False, indent=2) + "\n"`.
pub fn render(catalog: &Value) -> String {
    let mut text = serde_json::to_string_pretty(catalog).expect("Value sempre serializa");
    text.push('\n');
    text
}

/// `write_catalog_if_changed()`: grava só quando o conteúdo muda.
pub fn write_if_changed(text: &str, destination: &Path) -> Result<bool, CatalogError> {
    if std::fs::read_to_string(destination).is_ok_and(|current| current == text) {
        return Ok(false);
    }
    std::fs::write(destination, text)
        .map_err(|source| CatalogError::Io { path: destination.display().to_string(), source })?;
    Ok(true)
}

/// Caminho do catálogo versionado numa raiz.
pub fn catalog_path(root: &Path) -> PathBuf {
    root.join(CATALOG_FILE)
}
