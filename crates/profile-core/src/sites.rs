//! `profile-core check sites` — port de `scripts/check_websites.py`.
//!
//! As URLs vêm do catálogo gerado (`docs/project-catalog.json`,
//! `website_declared` ou `website`) e, para os repositórios que não estão lá,
//! do manifesto `docs/README_SITES.json`. O relatório (texto e `--json`) e o
//! código de saída são os do Python, byte a byte, exceto o `checked_at`
//! (conferido por `tests/fixtures/parity/site_monitor.json` e pelo teste de
//! ponta a ponta `tests/e2e/check_sites_parity.py`).

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use ecosystem_domain::discovery::{json_truthy, py_str};
use ecosystem_domain::normalize_site_overrides;
use serde_json::{Map, Value, json};
use site_monitor::{Status, WebsiteCheck};

/// Catálogo gerado, relativo à raiz.
pub const CATALOG_FILE: &str = "docs/project-catalog.json";
/// Manifesto editorial de sites, relativo à raiz.
pub const SITES_FILE: &str = "docs/README_SITES.json";
/// O que o Python imprime quando não há URL nenhuma.
pub const NO_URLS: &str = "nenhuma URL conhecida; rode o gerador ou preencha docs/README_SITES.json\n";

/// Catálogo num formato em que o `check_websites.py` quebraria com exceção.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{CATALOG_FILE} tem formato inesperado ({detail}); o check_websites.py quebraria com {python}")]
pub struct CollectError {
    /// Exceção do Python (`AttributeError`, `TypeError`).
    pub python: &'static str,
    /// O que está fora do formato.
    pub detail: String,
}

/// JSON de um arquivo, ou `None` onde o Python engoliria o erro
/// (`OSError`, `ValueError`: ausente, ilegível, UTF-8 ou JSON inválido).
fn read_json(path: &Path) -> Option<Value> {
    let bytes = std::fs::read(path).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    serde_json::from_str(&text).ok()
}

/// `collect_urls(root)`: repositório → URL.
pub fn collect_urls(root: &Path) -> Result<BTreeMap<String, String>, CollectError> {
    let mut urls = BTreeMap::new();
    if let Some(catalog) = read_json(&root.join(CATALOG_FILE)) {
        for project in catalog_projects(&catalog)? {
            let declared = project.get("website_declared").unwrap_or(&Value::Null);
            let website = if json_truthy(declared) { declared } else { project.get("website").unwrap_or(&Value::Null) };
            if json_truthy(website) {
                urls.insert(py_str(project.get("repository").unwrap_or(&Value::Null)), py_str(website));
            }
        }
    }
    if let Some(manifest) = read_json(&root.join(SITES_FILE)) {
        for (repository, website) in normalize_site_overrides(&manifest) {
            urls.entry(repository).or_insert(website);
        }
    }
    Ok(urls)
}

/// `catalog.get("projects", [])` iterado como o Python iteraria — ou o erro
/// com que ele quebraria.
fn catalog_projects(catalog: &Value) -> Result<Vec<&Map<String, Value>>, CollectError> {
    let shape = |python, detail: &str| CollectError { python, detail: detail.to_string() };
    let Some(catalog) = catalog.as_object() else {
        return Err(shape("AttributeError", "a raiz não é um objeto"));
    };
    let projects: Vec<&Value> = match catalog.get("projects") {
        None => Vec::new(),
        Some(Value::Array(items)) => items.iter().collect(),
        Some(Value::Object(fields)) if fields.is_empty() => Vec::new(),
        Some(Value::String(text)) if text.is_empty() => Vec::new(),
        Some(Value::Object(_) | Value::String(_)) => {
            return Err(shape("AttributeError", "\"projects\" não é uma lista"));
        }
        Some(_) => return Err(shape("TypeError", "\"projects\" não é iterável")),
    };
    projects
        .into_iter()
        .map(|project| {
            project.as_object().ok_or_else(|| shape("AttributeError", "um item de \"projects\" não é um objeto"))
        })
        .collect()
}

/// Uma linha do relatório.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// `owner/nome`.
    pub repository: String,
    /// URL verificada.
    pub website: String,
    /// `verified` | `unreachable` | `invalid` | `unknown`.
    pub status: &'static str,
    /// Código HTTP; 0 sem resposta.
    pub http_status: u16,
    /// Destino após redirects.
    pub final_url: String,
    /// Quando foi verificada.
    pub checked_at: String,
}

impl Row {
    fn is_down(&self) -> bool {
        self.status != Status::Verified.as_str()
    }
}

/// Linhas em ordem de repositório; URL sem checagem (vazia) é `unknown`.
pub fn build_rows(urls: &BTreeMap<String, String>, checks: &HashMap<String, WebsiteCheck>) -> Vec<Row> {
    urls.iter()
        .map(|(repository, website)| {
            let check = checks.get(website);
            Row {
                repository: repository.clone(),
                website: website.clone(),
                status: check.map_or("unknown", |c| c.status.as_str()),
                http_status: check.map_or(0, |c| c.http_status),
                final_url: check.map_or_else(String::new, |c| c.final_url.clone()),
                checked_at: check.map_or_else(String::new, |c| c.checked_at.clone()),
            }
        })
        .collect()
}

/// `--json`: `json.dumps(…, ensure_ascii=False, indent=2)` e a quebra de linha do `print`.
pub fn render_json(rows: &[Row]) -> String {
    let sites: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "repository": row.repository,
                "website": row.website,
                "status": row.status,
                "http_status": row.http_status,
                "final_url": row.final_url,
                "checked_at": row.checked_at,
            })
        })
        .collect();
    let down = rows.iter().filter(|row| row.is_down()).count();
    let report = json!({ "checked": rows.len(), "down": down, "sites": sites });
    serde_json::to_string_pretty(&report).expect("JSON serializável") + "\n"
}

/// Saída de texto.
pub fn render_text(rows: &[Row]) -> String {
    let mut out = String::new();
    for row in rows {
        let mark = if row.is_down() { "FORA" } else { "ok  " };
        let http = if row.http_status == 0 { "—".to_string() } else { row.http_status.to_string() };
        out.push_str(&format!("  {mark} {http:>4}  {}  →  {}\n", row.repository, row.website));
        if !row.final_url.is_empty() && row.final_url != row.website {
            out.push_str(&format!("            redireciona para: {}\n", row.final_url));
        }
    }
    let down = rows.iter().filter(|row| row.is_down()).count();
    out.push_str(&format!("\n{} sites verificados · {down} fora do ar\n", rows.len()));
    out
}

/// `1` só com `--fail-on-down` e algum site fora.
pub fn exit_code(rows: &[Row], fail_on_down: bool) -> u8 {
    u8::from(fail_on_down && rows.iter().any(Row::is_down))
}
