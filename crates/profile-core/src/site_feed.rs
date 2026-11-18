//! Public-only static website feed builder.
//!
//! It reads the checked-in editorial snapshot, applies an explicit public
//! field allowlist, redacts private repository names from tool evidence, and
//! stages a small static artifact for Vercel.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};
use thiserror::Error;

/// Schema consumed by the TypeScript portfolio.
pub const PROJECT_SCHEMA: &str = "lucas-belucci-bellini/public-portfolio@1";
/// Schema consumed by the TypeScript stack inventory.
pub const STACK_SCHEMA: &str = "lucas-belucci-bellini/professional-stack@1";

const CATEGORIES: [&str; 4] =
    ["Frameworks & Web", "Infraestrutura & DevOps", "IA & Conhecimento", "Hardware & Simulação"];

const STATUSES: [&str; 5] = ["🟢 Active", "🟡 In Development", "🔵 Experimental", "🟣 Academic", "⚪ Archived"];

const STATIC_FILES: [&str; 3] = ["index.html", "assets/site.css", "assets/favicon.svg"];

#[path = "site_feed_stack.rs"]
pub(super) mod stack;
#[path = "site_feed_support.rs"]
pub(super) mod support;
#[cfg(test)]
#[path = "site_feed_tests.rs"]
mod tests;

use self::stack::build_stack_inventory;
use self::support::public_projects;

/// Errors while preparing the public static artifact.
#[derive(Debug, Error)]
pub enum SiteError {
    /// A required input or output file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// Path involved in the operation.
        path: String,
        /// Underlying filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// A JSON input could not be parsed.
    #[error("{path}: invalid JSON: {source}")]
    Json {
        /// Input file.
        path: String,
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },
    /// An input does not satisfy the documented public feed contract.
    #[error("invalid site input: {0}")]
    Invalid(String),
}

/// Summary returned after a successful build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteBuildSummary {
    /// Number of public projects emitted in data/projects.json.
    pub public_projects: usize,
}

/// Build the website artifact from the checked-in public snapshot.
///
/// frontend_build must contain the JavaScript modules emitted by tsc
/// (main.js and contracts.js). The function never calls GitHub, connects to
/// PostgreSQL, or requires a token.
pub fn build(root: &Path, out: &Path, frontend_build: &Path) -> Result<SiteBuildSummary, SiteError> {
    let root = root.canonicalize().map_err(|source| SiteError::Io { path: root.display().to_string(), source })?;
    let out = if out.is_absolute() { out.to_path_buf() } else { root.join(out) };
    let expected_out = root.join("dist");
    if out == root || (out.starts_with(&root) && out != expected_out) {
        return Err(SiteError::Invalid("output inside the repository is restricted to dist/".into()));
    }

    let catalog = read_json(&root.join("docs/project-catalog.json"))?;
    let stack = read_json(&root.join("docs/README_STACK.json"))?;
    let archive = read_text(&root.join("docs/PROFILE-ARCHIVE.md"))?;
    let (projects, private_names) = public_projects(&catalog)?;
    let inventory = build_stack_inventory(&archive, &stack, &projects, &private_names)?;

    let frontend_build =
        if frontend_build.is_absolute() { frontend_build.to_path_buf() } else { root.join(frontend_build) };
    let modules = [("main.js", frontend_build.join("main.js")), ("contracts.js", frontend_build.join("contracts.js"))];
    for (name, path) in &modules {
        if !path.is_file() {
            return Err(SiteError::Invalid(format!(
                "compiled TypeScript module is missing: {} ({name})",
                path.display()
            )));
        }
    }

    let parent =
        out.parent().ok_or_else(|| SiteError::Invalid(format!("output has no parent directory: {}", out.display())))?;
    fs::create_dir_all(parent).map_err(|source| SiteError::Io { path: parent.display().to_string(), source })?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let stage = parent.join(format!(".portfolio-site-build-{}-{nonce}", std::process::id()));
    fs::create_dir(&stage).map_err(|source| SiteError::Io { path: stage.display().to_string(), source })?;

    let result = (|| {
        for relative in STATIC_FILES {
            copy_file(&root.join(relative), &stage.join(relative))?;
        }
        for (name, source) in &modules {
            copy_file(source, &stage.join("assets").join(name))?;
        }

        let data_dir = stage.join("data");
        fs::create_dir_all(&data_dir)
            .map_err(|source| SiteError::Io { path: data_dir.display().to_string(), source })?;
        let projects_payload = object([
            ("schema", Value::String(PROJECT_SCHEMA.into())),
            ("project_count", Value::from(projects.len())),
            ("projects", Value::Array(projects.clone())),
        ]);
        write_json(&data_dir.join("projects.json"), &projects_payload)?;
        write_json(&data_dir.join("stack-inventory.json"), &inventory)?;
        write_text(&stage.join(".nojekyll"), "")?;

        if out.exists() {
            if !out.is_dir() {
                return Err(SiteError::Invalid(format!(
                    "output path exists and is not a directory: {}",
                    out.display()
                )));
            }
            fs::remove_dir_all(&out).map_err(|source| SiteError::Io { path: out.display().to_string(), source })?;
        }
        fs::rename(&stage, &out).map_err(|source| SiteError::Io { path: out.display().to_string(), source })?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&stage);
    }
    result?;
    Ok(SiteBuildSummary { public_projects: projects.len() })
}

fn read_json(path: &Path) -> Result<Value, SiteError> {
    let text = read_text(path)?;
    serde_json::from_str(&text).map_err(|source| SiteError::Json { path: path.display().to_string(), source })
}

fn read_text(path: &Path) -> Result<String, SiteError> {
    fs::read_to_string(path).map_err(|source| SiteError::Io { path: path.display().to_string(), source })
}

fn copy_file(source: &Path, target: &Path) -> Result<(), SiteError> {
    let parent = target
        .parent()
        .ok_or_else(|| SiteError::Invalid(format!("file has no parent directory: {}", target.display())))?;
    fs::create_dir_all(parent).map_err(|source| SiteError::Io { path: parent.display().to_string(), source })?;
    fs::copy(source, target).map_err(|source| SiteError::Io { path: target.display().to_string(), source })?;
    Ok(())
}

fn write_text(path: &Path, text: &str) -> Result<(), SiteError> {
    let parent =
        path.parent().ok_or_else(|| SiteError::Invalid(format!("file has no parent directory: {}", path.display())))?;
    fs::create_dir_all(parent).map_err(|source| SiteError::Io { path: parent.display().to_string(), source })?;
    fs::write(path, text).map_err(|source| SiteError::Io { path: path.display().to_string(), source })
}

fn write_json(path: &Path, value: &Value) -> Result<(), SiteError> {
    let sorted = sort_json(value.clone());
    let mut text = serde_json::to_string_pretty(&sorted)
        .map_err(|source| SiteError::Json { path: path.display().to_string(), source })?;
    text.push('\n');
    write_text(path, &text)
}

fn sort_json(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = Map::new();
            for (key, value) in entries {
                sorted.insert(key, sort_json(value));
            }
            Value::Object(sorted)
        }
        Value::Array(values) => Value::Array(values.into_iter().map(sort_json).collect()),
        other => other,
    }
}

fn object<const N: usize>(pairs: [(&str, Value); N]) -> Value {
    let mut map = Map::new();
    for (key, value) in pairs {
        map.insert(key.to_string(), value);
    }
    sort_json(Value::Object(map))
}
