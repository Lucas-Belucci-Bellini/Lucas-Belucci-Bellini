//! `profile-core import manifests` e `import legacy`: leitura dos arquivos.
//!
//! A gravação (e as regras de o que substitui o quê) está em
//! `store::editorial` e `store::legacy`; aqui só se lê e valida o formato, com
//! as mesmas funções que o catálogo usa (a descoberta de site é a mesma).

use std::path::Path;

use catalog::{CatalogError, EXCLUDED_FILE, FEATURED_FILE, OWNER, load_json_object};
use ecosystem_domain::discovery::{json_truthy, py_str};
use ecosystem_domain::lifecycle::{ALWAYS_IN_DEVELOPMENT, BACKLOG_EXCEPTION, BACKLOG_PREFIX};
use ecosystem_domain::presentation::FEATURED_SUMMARIES;
use ecosystem_domain::pyjson::py_int;
use ecosystem_domain::text::py_strip;
use ecosystem_domain::url::looks_like_http_url;
use serde_json::{Map, Value};
use store::editorial::{FeaturedEntry, Manifests, StackTool};
use store::legacy::{CatalogSite, LegacyRecord};
use store::metrics::Sample;

/// Arsenal editorial.
pub const STACK_FILE: &str = "docs/README_STACK.json";

fn invalid(path: &str, detail: String) -> CatalogError {
    CatalogError::Manifest(format!("{path}: {detail}"))
}

fn text_field(entry: &Map<String, Value>, key: &str) -> Option<String> {
    entry.get(key).and_then(Value::as_str).map(str::to_string).filter(|value| !value.is_empty())
}

/// Os quatro manifestos, validados no formato que o banco aceita.
pub fn read_manifests(root: &Path) -> Result<(Manifests, Vec<String>), CatalogError> {
    let mut ignored = Vec::new();
    let excluded = load_json_object(&root.join(EXCLUDED_FILE))?;
    let exclusions: Vec<String> = catalog::load_excluded(root)?.into_iter().collect();
    let exclusion_reason = excluded.get("reason").map(py_str).unwrap_or_else(|| "decisão editorial".into());

    let featured_manifest = load_json_object(&root.join(FEATURED_FILE))?;
    let mut featured = Vec::new();
    for entry in featured_manifest.get("projects").and_then(Value::as_array).into_iter().flatten() {
        let Value::Object(entry) = entry else {
            return Err(invalid(FEATURED_FILE, "entrada de \"projects\" que não é objeto".into()));
        };
        let Some(name) = entry.get("name").filter(|name| json_truthy(name)).map(py_str) else { continue };
        let position = py_int(entry.get("order").unwrap_or(&Value::from(999)))
            .ok()
            .and_then(|order| i16::try_from(order).ok())
            .filter(|order| *order > 0)
            .ok_or_else(|| invalid(FEATURED_FILE, format!("\"order\" de {name:?} precisa ser inteiro positivo")))?;
        let priority = match entry.get("priority") {
            None | Some(Value::Null) => None,
            Some(value) => Some(
                py_int(value)
                    .ok()
                    .and_then(|p| i16::try_from(p).ok())
                    .filter(|p| (0..=100).contains(p))
                    .ok_or_else(|| invalid(FEATURED_FILE, format!("\"priority\" de {name:?} fora de 0–100")))?,
            ),
        };
        let label =
            text_field(entry, "label").ok_or_else(|| invalid(FEATURED_FILE, format!("{name:?} sem \"label\"")))?;
        featured.push(FeaturedEntry {
            repository: text_field(entry, "repository").unwrap_or_else(|| format!("{OWNER}/{name}")),
            position,
            priority,
            label,
            focus: text_field(entry, "focus"),
            reason: text_field(entry, "reason"),
            website_required: entry.get("website_required").is_some_and(json_truthy),
        });
    }

    let stack = load_json_object(&root.join(STACK_FILE))?;
    let mut tools = Vec::new();
    for tool in stack.get("tools").and_then(Value::as_array).into_iter().flatten() {
        let Value::Object(tool) = tool else {
            return Err(invalid(STACK_FILE, "ferramenta que não é objeto".into()));
        };
        let field = |key: &str| {
            text_field(tool, key)
                .filter(|value| !py_strip(value).is_empty())
                .ok_or_else(|| invalid(STACK_FILE, format!("ferramenta sem {key:?}: {}", Value::Object(tool.clone()))))
        };
        tools.push(StackTool {
            name: field("name")?,
            category: field("category")?,
            family: field("family")?,
            evidence: field("evidence")?,
        });
    }

    let mut sites = Vec::new();
    for (full_name, url) in catalog::load_site_overrides(root) {
        let url = py_strip(&url);
        if looks_like_http_url(url) {
            sites.push((full_name, url.to_string()));
        } else {
            ignored.push(format!("site de {full_name}: {url:?} não é URL http(s)"));
        }
    }

    Ok((Manifests { exclusions, exclusion_reason, featured, tools, sites }, ignored))
}

/// O estado legado: catálogo, monitor e timeline, mais as constantes do código.
pub fn read_legacy(root: &Path, owner: &str) -> Result<LegacyRecord, CatalogError> {
    let mut record = LegacyRecord {
        summaries: FEATURED_SUMMARIES.iter().map(|(name, summary)| (name.to_string(), summary.to_string())).collect(),
        in_development: ALWAYS_IN_DEVELOPMENT.iter().map(|name| name.to_string()).collect(),
        in_development_prefix: Some((BACKLOG_PREFIX.to_string(), BACKLOG_EXCEPTION.to_string())),
        owner: owner.to_string(),
        ..LegacyRecord::default()
    };

    let catalog = load_json_object(&catalog::catalog_path(root))?;
    for project in catalog.get("projects").and_then(Value::as_array).into_iter().flatten() {
        let (Some(repository), Some(url)) =
            (project["repository"].as_str(), project.get("website_declared").and_then(Value::as_str))
        else {
            continue;
        };
        let outcome = project["website_status"].as_str().unwrap_or("none");
        if !matches!(outcome, "verified" | "unreachable" | "invalid") {
            continue;
        }
        let http = project["website_http_status"].as_i64().filter(|code| (100..=599).contains(code));
        record.sites.push(CatalogSite {
            repository: repository.to_string(),
            url: url.to_string(),
            source: match project["website_source"].as_str() {
                Some("github_homepage") => "github_homepage".into(),
                _ => "manifest".into(),
            },
            outcome: outcome.to_string(),
            http_status: http.and_then(|code| i16::try_from(code).ok()),
            final_url: project
                .get("website_final_url")
                .or_else(|| project.get("website"))
                .and_then(Value::as_str)
                .or(Some(url))
                .map(str::to_string),
        });
    }

    let state = load_json_object(&root.join(crate::commits::STATE_FILE))?;
    for (name, entry) in state.get("repositories").and_then(Value::as_object).into_iter().flatten() {
        if let Some(observation) = crate::commits::observation(name, entry, None) {
            record.heads.push(observation);
        }
    }
    let metrics = state.get("metrics").and_then(Value::as_object);
    for (field, key) in [
        ("project_commits", "ecosystem.commits.project_total"),
        ("monitor_commits", "ecosystem.commits.monitor_total"),
        ("tracked_commits", "ecosystem.commits.tracked_total"),
    ] {
        if let Some(value) = metrics.and_then(|m| m.get(field)).and_then(|v| py_int(v).ok()) {
            record.samples.push(Sample {
                metric_key: key.into(),
                value: value.to_string(),
                window_start: None,
                window_end: None,
            });
        }
    }

    let timeline = root.join(crate::contributions::DATA_FILE);
    if timeline.exists() {
        let payload = Value::Object(load_json_object(&timeline)?);
        record.samples.extend(crate::contributions::samples(&payload).0);
    }
    Ok(record)
}
