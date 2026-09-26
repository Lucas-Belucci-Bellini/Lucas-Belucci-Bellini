//! Descoberta de site — `normalize_site_overrides()` e
//! `discover_project_website()` de `scripts/project_catalog.py`.
//!
//! Ordem: `homepage` do GitHub → manifesto `docs/README_SITES.json` → nada.
//! **Nenhuma URL é deduzida do nome do repositório.**

use std::collections::BTreeMap;

use serde_json::Value;

use crate::repo::RepoFacts;
use crate::text::py_strip;
use crate::url::looks_like_http_url;

/// De onde veio a URL de um projeto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebsiteSource {
    /// Campo `homepage` do repositório.
    GithubHomepage,
    /// `docs/README_SITES.json`.
    Manifest,
    /// Nenhuma fonte declarou site.
    None,
}

impl WebsiteSource {
    /// Texto gravado no catálogo (`website_source`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GithubHomepage => "github_homepage",
            Self::Manifest => "manifest",
            Self::None => "none",
        }
    }
}

/// Lê o manifesto nos dois formatos aceitos e devolve `owner/repo → website`.
///
/// ```json
/// { "owner/repo": "https://…" }                    // histórico
/// { "owner/repo": { "website": "https://…" } }     // estendido
/// ```
///
/// Entrada que não é texto nem objeto, ou objeto sem `website` verdadeiro (no
/// sentido do Python), é ignorada. Um `website` verdadeiro que não é texto
/// vira texto como a `str()` do Python faz para booleano e inteiro (`True`,
/// `42`). **Divergência conhecida:** float, lista e objeto saem em forma JSON,
/// não no `repr` do Python. Nenhum dos dois passa por [`looks_like_http_url`],
/// então a descoberta de site não muda; o que mudaria é o texto que
/// `scripts/check_websites.py` relata como URL inválida — a portar junto com ele.
pub fn normalize_site_overrides(raw: &Value) -> BTreeMap<String, String> {
    let Some(entries) = raw.as_object() else {
        return BTreeMap::new();
    };
    entries
        .iter()
        .filter_map(|(key, value)| {
            let website = match value {
                Value::String(url) => url.clone(),
                Value::Object(object) => match object.get("website") {
                    Some(Value::String(url)) if !url.is_empty() => url.clone(),
                    Some(other) if json_truthy(other) => py_str(other),
                    _ => return None,
                },
                _ => return None,
            };
            Some((key.clone(), website))
        })
        .collect()
}

/// `str()` do Python para o que `json.loads` devolve — exata para texto,
/// `None`, booleano e inteiro; ver a divergência em [`normalize_site_overrides`].
pub fn py_str(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Verdade do Python para um valor JSON: `null`, `false`, `0`, `""`, `[]` e
/// `{}` são falsos.
pub fn json_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64() != Some(0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
    }
}

/// Descobre a URL declarada de um projeto e a origem dela.
pub fn discover_project_website(
    repo: &RepoFacts,
    overrides: &BTreeMap<String, String>,
) -> (Option<String>, WebsiteSource) {
    let homepage = py_strip(repo.homepage.as_deref().unwrap_or(""));
    if looks_like_http_url(homepage) {
        return (Some(homepage.to_string()), WebsiteSource::GithubHomepage);
    }
    if let Some(entry) = overrides.get(&repo.full_name) {
        let url = py_strip(entry);
        if looks_like_http_url(url) {
            return (Some(url.to_string()), WebsiteSource::Manifest);
        }
    }
    (None, WebsiteSource::None)
}
