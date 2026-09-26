//! `profile-core render readme` — o `scripts/update_profile.py` inteiro:
//! `README.md`, `assets/profile-snapshot.svg` e `docs/project-catalog.json`.
//!
//! O preparo (inventário, exclusões, linguagens, sites, verificação,
//! curadoria) é o mesmo do `catalog build` ([`catalog_build::prepare`]); os
//! blocos vêm do crate `profile-render`. A saída padrão é a do Python, byte a
//! byte: sem `--write`, os 500 primeiros caracteres do README; depois, o
//! resumo em JSON (`json.dumps(..., ensure_ascii=False)`).
//!
//! - `--write` grava os três arquivos na raiz (o catálogo, só se mudou) e,
//!   sem inventário de arquivo, exige `PROFILE_GITHUB_TOKEN`: um inventário só
//!   de públicos apagaria os privados do README.
//! - `--out-dir DIR` grava os três em `DIR` sem tocar no que é publicado —
//!   é como o modo sombra compara com o Python, com ou sem token.
//! - `--catalog-out FILE` grava só o catálogo (o gancho da Fase 3).

use std::path::{Path, PathBuf};

use profile_render::pytext::py_prefix;
use profile_render::snapshot::generated_at;
use profile_render::{Inputs, Profile};
use serde_json::{Value, json};

use crate::catalog_build::{self, BuildError, BuildOptions, Prepared, Purpose, Tokens};

/// O que o comando recebeu.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// O mesmo do `catalog build` (com `languages_dir`).
    pub build: BuildOptions,
    /// Grava também o catálogo neste arquivo.
    pub catalog_out: Option<PathBuf>,
    /// Grava README, snapshot e catálogo neste diretório, sem tocar na raiz.
    pub out_dir: Option<PathBuf>,
    /// `--skip-site-check` (vai para o resumo).
    pub site_check_skipped: bool,
}

/// Resultado de uma execução.
#[derive(Debug)]
pub struct Rendered {
    /// O que o Python imprimiria, byte a byte.
    pub stdout: String,
    /// Linha de resumo para o stderr.
    pub note: String,
}

/// `README.read_text()`: modo texto, com as quebras universais do Python
/// (`\r\n` e `\r` viram `\n`).
fn read_text(path: &Path) -> Result<String, BuildError> {
    let text = std::fs::read_to_string(path).map_err(BuildError::disk(path))?;
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

fn write(path: &Path, text: &str) -> Result<(), BuildError> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(BuildError::disk(parent))?;
    }
    std::fs::write(path, text).map_err(BuildError::disk(path))
}

fn write_catalog(text: &str, path: &Path) -> Result<bool, BuildError> {
    catalog::write_if_changed(text, path).map_err(|error| match error {
        catalog::CatalogError::Io { path, source } => BuildError::Disk { path, source },
        other => BuildError::Catalog(other),
    })
}

/// `json.dumps(summary, ensure_ascii=False)`: separadores `", "` e `": "`.
fn py_dumps(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let fields: Vec<String> =
                map.iter().map(|(key, value)| format!("{}: {}", Value::String(key.clone()), py_dumps(value))).collect();
            format!("{{{}}}", fields.join(", "))
        }
        Value::Array(items) => format!("[{}]", items.iter().map(py_dumps).collect::<Vec<_>>().join(", ")),
        other => other.to_string(),
    }
}

/// O resumo que o `main()` do Python imprime por último.
fn summary(prepared: &Prepared, rows: usize, catalog_written: bool, options: &RenderOptions) -> Value {
    let public = prepared.repos.iter().filter(|repo| !repo.private).count();
    let mut unreachable: Vec<&str> =
        prepared.presentations.iter().filter(|p| p.website_status == "unreachable").map(|p| p.name.as_str()).collect();
    unreachable.sort_unstable();
    json!({
        "repositories": prepared.repos.len(),
        "public_repositories": public,
        "private_repositories": prepared.repos.len() - public,
        "excluded_repositories": prepared.excluded,
        "public_languages": rows,
        "declared_sites": catalog::candidates(&prepared.sites).len(),
        "verified_sites": prepared.presentations.iter().filter(|p| p.has_live_website()).count(),
        "unreachable_sites": unreachable,
        "catalog_projects": prepared.presentations.len(),
        "catalog_written": catalog_written,
        "site_check_skipped": options.site_check_skipped,
        "write": options.build.write,
    })
}

/// Roda o comando.
pub async fn run(options: &RenderOptions, tokens: &Tokens) -> Result<Rendered, BuildError> {
    let root = &options.build.root;
    let prepared = catalog_build::prepare(&options.build, tokens, Purpose::Readme).await?;
    let stack = prepared.stack.as_ref().expect("Purpose::Readme lê o arsenal");
    let stamp = generated_at(&prepared.repos, prepared.now_local)?;
    let template = read_text(&root.join("README.md"))?;
    let profile = Profile::new(Inputs {
        repos: &prepared.repos,
        languages: &prepared.languages,
        presentations: &prepared.presentations,
        excluded: prepared.excluded.len(),
        now: prepared.now,
        featured: &prepared.featured,
        stack,
        generated_at: &stamp,
    });
    let readme = profile.readme(&template)?;
    let catalog_text = catalog::render(&catalog::build(&prepared.presentations));

    let mut stdout = String::new();
    let mut catalog_written = false;
    if options.build.write {
        write(&root.join("README.md"), &readme)?;
        write(&root.join("assets/profile-snapshot.svg"), &profile.snapshot_svg())?;
        catalog_written = write_catalog(&catalog_text, &catalog::catalog_path(root))?;
    } else {
        stdout.push_str(py_prefix(&readme, 500));
        stdout.push('\n');
    }
    if let Some(path) = &options.catalog_out {
        write_catalog(&catalog_text, path)?;
    }
    if let Some(dir) = &options.out_dir {
        write(&dir.join("README.md"), &readme)?;
        write(&dir.join("profile-snapshot.svg"), &profile.snapshot_svg())?;
        write_catalog(&catalog_text, &dir.join("project-catalog.json"))?;
    }
    let summary = summary(&prepared, profile.rows.len(), catalog_written, options);
    stdout.push_str(&py_dumps(&summary));
    stdout.push('\n');
    let note = format!(
        "README: {} repositórios, {} linguagens públicas, {} sites no ar; carimbo {stamp}{}",
        prepared.repos.len(),
        profile.rows.len(),
        profile.verified_sites(),
        if options.build.write { "; gravado" } else { "" }
    );
    Ok(Rendered { stdout, note })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dumps_como_o_python() {
        let value = json!({"a": 1, "b": ["x", "ç\"\n"], "c": true, "d": []});
        assert_eq!(r#"{"a": 1, "b": ["x", "ç\"\n"], "c": true, "d": []}"#, py_dumps(&value));
    }
}
