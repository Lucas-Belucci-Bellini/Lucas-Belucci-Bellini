//! `render all` e `render … --check` — o pipeline único do D-018.
//!
//! Hoje dois workflows geram o que é publicado: o `update-profile.yml` (README,
//! snapshot e catálogo, diário) e o `lang-stats.yml` (os seis SVGs de
//! `assets/`, semanal). O alvo do D-018 é um job que roda tudo e faz um
//! commit; `render all` é esse "tudo", num processo, na ordem README →
//! lang-stats → cards, parando no primeiro que falhar:
//!
//! - `--write` grava na raiz, como os três scripts;
//! - `--out-dir DIR` grava em `DIR` com o mesmo layout da raiz (`DIR/README.md`,
//!   `DIR/assets/…`, `DIR/docs/project-catalog.json`) — os SVGs publicados
//!   são copiados antes, para o "mudou?" do lang-stats comparar com eles;
//! - `--check` gera numa cópia temporária, não grava nada e sai com `1` se
//!   algum arquivo publicado mudaria. README, snapshot e catálogo contam byte
//!   a byte; os SVGs, sem o carimbo de hora (a regra do `lang_stats.py`) — sem
//!   isso os cards reprovariam sempre, porque são regravados a cada execução
//!   só pelo carimbo (A27).

use std::path::{Path, PathBuf};

use profile_render::lang_stats;

use crate::assets::{self, AssetsError};
use crate::catalog_build::{BuildError, Tokens};
use crate::render::{self, OUTPUT_FILES, RenderOptions};

/// Os seis SVGs de `assets/` que o `lang-stats.yml` publica.
pub const ASSET_FILES: [&str; 6] = [
    "assets/lang-stats.svg",
    "assets/profile-top-langs.svg",
    "assets/profile-stats.svg",
    "assets/profile-streak.svg",
    "assets/profile-trophies.svg",
    "assets/profile-projects.svg",
];

/// Onde as saídas vão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Na raiz (como os scripts).
    Write,
    /// Num diretório à parte, com o layout da raiz.
    OutDir(PathBuf),
    /// Em lugar nenhum: só diz o que mudaria.
    Check,
}

/// Por que o pipeline parou.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// No README (o `update_profile.py`).
    #[error(transparent)]
    Readme(#[from] BuildError),
    /// Nos SVGs (o `lang_stats.py` ou o `profile_cards.py`).
    #[error(transparent)]
    Assets(#[from] AssetsError),
    /// Ao preparar a cópia.
    #[error("{0}: {1}")]
    Io(String, std::io::Error),
}

impl PipelineError {
    /// Onde o Python sairia com traceback (código 1).
    pub fn is_python_crash(&self) -> bool {
        match self {
            Self::Readme(error) => error.is_python_crash(),
            Self::Assets(error) => matches!(error, AssetsError::Crash(_)),
            Self::Io(..) => false,
        }
    }
}

/// O que o pipeline produziu.
#[derive(Debug, Default)]
pub struct Report {
    /// A saída padrão dos passos, em ordem (no `--check`, o veredito).
    pub stdout: String,
    /// Linhas para o stderr.
    pub notes: Vec<String>,
    /// Código de saída.
    pub code: u8,
    /// No `--check`: os arquivos que mudariam.
    pub changed: Vec<&'static str>,
}

/// Um diretório temporário que se apaga sozinho.
struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Result<Self, PipelineError> {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        let dir = std::env::temp_dir().join(format!("profile-core-{label}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|error| PipelineError::Io(dir.display().to_string(), error))?;
        Ok(Self(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Copia os SVGs publicados para `dir/assets/` (o ponto de partida do
/// "mudou?" do lang-stats).
fn seed_assets(root: &Path, dir: &Path) -> Result<(), PipelineError> {
    let target = dir.join("assets");
    std::fs::create_dir_all(&target).map_err(|error| PipelineError::Io(target.display().to_string(), error))?;
    for file in ASSET_FILES {
        let source = root.join(file);
        if source.is_file() {
            std::fs::copy(&source, dir.join(file))
                .map_err(|error| PipelineError::Io(source.display().to_string(), error))?;
        }
    }
    Ok(())
}

/// O README gerado: o `render readme` com a gravação decidida pelo modo.
async fn readme(options: &RenderOptions, tokens: &Tokens, mode: &Mode) -> Result<render::Rendered, PipelineError> {
    let mut options = options.clone();
    options.build.write = *mode == Mode::Write;
    options.out_dir = None;
    Ok(render::run(&options, tokens).await?)
}

/// `true` se `file` mudaria de `root` para `staged`.
fn differs(root: &Path, staged: &Path, file: &str) -> bool {
    let (old, new) = (std::fs::read(root.join(file)).ok(), std::fs::read(staged.join(file)).ok());
    match (old, new) {
        (Some(old), Some(new)) if file.ends_with(".svg") && file != OUTPUT_FILES[1] => {
            !lang_stats::same_content(&String::from_utf8_lossy(&new), &String::from_utf8_lossy(&old))
        }
        (old, new) => old != new,
    }
}

fn verdict(report: &mut Report, root: &Path, staged: &Path, files: &[&'static str]) {
    for &file in files {
        let changed = differs(root, staged, file);
        report.stdout.push_str(&format!("  {}  {file}\n", if changed { "mudaria" } else { "igual  " }));
        if changed {
            report.changed.push(file);
        }
    }
    report.stdout.push_str(&if report.changed.is_empty() {
        "nada mudaria: a publicação está em dia com as entradas\n".to_string()
    } else {
        format!("{} arquivo(s) mudaria(m)\n", report.changed.len())
    });
    report.code = u8::from(!report.changed.is_empty());
}

/// `render readme --check`: README, snapshot e catálogo contra os da raiz.
pub async fn check_readme(options: &RenderOptions, tokens: &Tokens) -> Result<Report, PipelineError> {
    let rendered = readme(options, tokens, &Mode::Check).await?;
    let scratch = Scratch::new("check")?;
    rendered.outputs.stage(&scratch.0)?;
    let mut report = Report { notes: vec![rendered.note], ..Report::default() };
    verdict(&mut report, &options.build.root, &scratch.0, &OUTPUT_FILES);
    Ok(report)
}

/// `render all`: README, lang-stats e cards, nessa ordem.
pub async fn render_all(
    options: &RenderOptions,
    assets_options: &assets::Options,
    tokens: &Tokens,
    mode: &Mode,
) -> Result<Report, PipelineError> {
    let root = options.build.root.clone();
    let scratch = if *mode == Mode::Check { Some(Scratch::new("check")?) } else { None };
    let staged: Option<PathBuf> = match mode {
        Mode::Write => None,
        Mode::OutDir(dir) => Some(dir.clone()),
        Mode::Check => scratch.as_ref().map(|scratch| scratch.0.clone()),
    };

    let rendered = readme(options, tokens, mode).await?;
    let mut report = Report { notes: vec![rendered.note.clone()], ..Report::default() };
    let mut stdout = rendered.stdout.clone();
    if let Some(dir) = &staged {
        rendered.outputs.stage(dir)?;
        seed_assets(&root, dir)?;
    }

    let assets_options = assets::Options { out: staged.clone(), ..assets_options.clone() };
    let lang = assets::run_lang_stats(&assets_options).await?;
    stdout.push_str(&lang.stdout);
    if lang.code != 0 {
        report.stdout = stdout;
        report.code = lang.code;
        report.notes.push("lang-stats sem linguagem nenhuma: os cards não rodaram".into());
        return Ok(report);
    }
    stdout.push_str(&assets::run_cards(&assets_options).await?.stdout);

    match (mode, &staged) {
        (Mode::Check, Some(dir)) => {
            let files: Vec<&'static str> = OUTPUT_FILES.into_iter().chain(ASSET_FILES).collect();
            verdict(&mut report, &root, dir, &files);
        }
        _ => report.stdout = stdout,
    }
    Ok(report)
}
