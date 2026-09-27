//! `profile-core validate` — os validadores do `scripts/` em Rust.
//!
//! | subcomando   | script Python                      | no CI hoje            |
//! |:-------------|:-----------------------------------|:----------------------|
//! | `readme`     | `validate_dynamic_sections.py`     | `update-profile.yml`  |
//! | `exclusions` | `validate_exclusions.py`           | `update-profile.yml`  |
//! | `links`      | `validate_project_links.py`        | os dois               |
//! | `visual`     | `validate_restored_style.py`       | `v2-validation.yml`   |
//! | `badges`     | `validate_language_badges.py`      | `v2-validation.yml`   |
//! | `catalog`    | `validate_profile.py` (sem a rede) | fora do CI            |
//!
//! Os cinco primeiros produzem a mesma saída padrão, o mesmo stderr e o mesmo
//! código de saída do script, byte a byte — inclusive as mensagens da
//! biblioteca padrão (`json.JSONDecodeError`, `OSError`, `repr`). Onde o
//! script sairia com traceback, o Rust devolve a exceção (tipo e texto) e sai
//! com `1`, como o Python. A prova é `tests/fixtures/parity/validators.json`,
//! gerado pelos scripts reais. O `catalog` devolve os mesmos erros do
//! `validate_profile.py`, sem o que ele confere de fora do gerador (a rede, o
//! YAML do workflow, o Markdown com pacote opcional); a ordem dos excluídos
//! achados é a do manifesto (no Python, a de um `set`).
//!
//! Os validadores Python continuam no CI como oráculo (D-015): estes são o
//! equivalente que o §4 do plano exige para um dia sair do modo C.

use std::path::{Path, PathBuf};
use std::time::Duration;

use ecosystem_domain::discovery::json_truthy;
use ecosystem_domain::monitor::{PyException, py_get, py_prefix, py_splitlines, py_type_name};
use ecosystem_domain::pyio::{decode_utf8, is_os_error, os_error, universal_newlines};
use ecosystem_domain::pyjson;
use ecosystem_domain::pyrepr::{py_repr, py_str};
use ecosystem_domain::text::py_strip;
use profile_render::pytext::py_dumps;
use regex::Regex;
use serde_json::{Value, json};

/// O que um validador imprimiu e com que código saiu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Saída padrão.
    pub stdout: String,
    /// Saída de erro.
    pub stderr: String,
    /// Código de saída.
    pub code: u8,
}

impl Outcome {
    fn pass(stdout: String) -> Self {
        Self { stdout, stderr: String::new(), code: 0 }
    }

    fn fail(stderr: String, code: u8) -> Self {
        Self { stdout: String::new(), stderr, code }
    }
}

/// Onde o script Python sairia com traceback (ou, `Unsupported`, onde um
/// valor JSON aceito por ele não cabe no Rust).
pub type Crash = PyException;

/// Os 13 marcadores, na ordem do `validate_dynamic_sections.py`.
pub const MARKERS: [&str; 13] = [
    "PROFILE-DASHBOARD",
    "WHAT-I-BUILD",
    "PRODUCT-CARDS",
    "FEATURED-PROJECTS",
    "ARSENAL-STACK",
    "LANGUAGE-BADGES",
    "LANGUAGE-STATS",
    "PUBLIC-PROJECTS",
    "PRIVATE-PROJECTS",
    "WEBSITE-DIRECTORY",
    "LIVE-PROJECTS",
    "ECOSYSTEM-MAP",
    "PROJECT-MAP",
];

/// `ROOT` do script: `Path(__file__).resolve().parents[1]`.
pub fn resolve_root(root: &Path) -> PathBuf {
    root.canonicalize().or_else(|_| std::path::absolute(root)).unwrap_or_else(|_| root.to_path_buf())
}

fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// `path.read_text(encoding="utf-8")`.
pub fn read_text(path: &Path) -> Result<String, Crash> {
    let bytes = std::fs::read(path).map_err(|error| os_error(&error, &display(path)))?;
    Ok(universal_newlines(&decode_utf8(&bytes)?))
}

/// `json.loads(path.read_text(encoding="utf-8"))`.
fn load_json(path: &Path) -> Result<Value, Crash> {
    pyjson::loads(&read_text(path)?).map_err(|error| error.exception())
}

/// `for item in value`: lista, texto (caracteres) e dicionário (chaves).
fn py_iter(value: &Value) -> Result<Vec<Value>, Crash> {
    match value {
        Value::Array(items) => Ok(items.clone()),
        Value::String(text) => Ok(text.chars().map(|c| Value::String(c.to_string())).collect()),
        Value::Object(fields) => Ok(fields.keys().map(|key| Value::String(key.clone())).collect()),
        other => Err(PyException::new("TypeError", format!("'{}' object is not iterable", py_type_name(other)))),
    }
}

/// `value.get(key, default)`.
fn get_or<'a>(value: &'a Value, key: &str, default: &'a Value) -> Result<&'a Value, Crash> {
    Ok(py_get(value, key)?.unwrap_or(default))
}

/// `re.search(START(.*?)END, text, re.S)`: o miolo do primeiro par.
fn block<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = text.find(start)? + start.len();
    let length = text[from..].find(end)?;
    Some(&text[from..from + length])
}

fn marker_pair(marker: &str) -> (String, String) {
    (format!("<!-- {marker}:START -->"), format!("<!-- {marker}:END -->"))
}

/// `str.find` como posição comparável: `-1` quando não acha.
fn find(line: &str, needle: &str) -> i64 {
    line.find(needle).map_or(-1, |index| index as i64)
}

// ---------------------------------------------------------------- readme

/// `canonicalize()` do `validate_dynamic_sections.py`.
fn canonicalize(text: &str) -> Result<String, Crash> {
    let mut text = text.to_string();
    for marker in MARKERS {
        let (start, end) = marker_pair(marker);
        let missing = || PyException::new("ValueError", format!("README marker not found exactly once: {marker}"));
        let begin = text.find(&start).ok_or_else(missing)?;
        let body = begin + start.len();
        let stop = body + text[body..].find(&end).ok_or_else(missing)? + end.len();
        text.replace_range(begin..stop, &format!("{start}\n__GENERATED_{marker}__\n{end}"));
    }
    Ok(text)
}

/// `validate_dynamic_sections.py --before B --after A`: fora dos 13 blocos,
/// nada mudou.
pub fn readme(before: &Path, after: &Path) -> Result<Outcome, Crash> {
    let before = canonicalize(&read_text(before)?)?;
    let after = canonicalize(&read_text(after)?)?;
    if before != after {
        return Ok(Outcome::fail("static README content changed outside generated markers\n".into(), 1));
    }
    Ok(Outcome::pass("dynamic-only README validation: pass\n".into()))
}

// ------------------------------------------------------------ exclusions

/// `validate_exclusions.py`: nenhum nome excluído aparece no README.
pub fn exclusions(root: &Path) -> Result<Outcome, Crash> {
    let root = resolve_root(root);
    let manifest = load_json(&root.join("docs/README_EXCLUDED.json"))?;
    let readme = read_text(&root.join("README.md"))?;
    let excluded: Vec<String> = py_iter(get_or(&manifest, "repositories", &json!([]))?)?.iter().map(py_str).collect();
    let found: Vec<&str> = excluded.iter().map(String::as_str).filter(|name| readme.contains(name)).collect();
    if !found.is_empty() {
        return Ok(Outcome::fail(format!("excluded repository names found in README: {}\n", found.join(", ")), 1));
    }
    Ok(Outcome::pass(format!("excluded repositories absent from README: {}\n", excluded.len())))
}

// ----------------------------------------------------------------- links

const SHOWCASE_MARKERS: [&str; 4] = ["PRODUCT-CARDS", "FEATURED-PROJECTS", "WEBSITE-DIRECTORY", "LIVE-PROJECTS"];

/// `validate_project_links.py`: com site verificado, o site vem primeiro.
pub fn links(root: &Path) -> Result<Outcome, Crash> {
    let root = resolve_root(root);
    let catalog_path = root.join("docs/project-catalog.json");
    if !catalog_path.exists() {
        return Ok(Outcome::fail(format!("catálogo ausente: {}\n", display(&catalog_path)), 2));
    }
    let catalog = load_json(&catalog_path)?;
    let projects = get_or(&catalog, "projects", &json!([]))?.clone();
    if !json_truthy(&projects) {
        return Ok(Outcome::fail("catálogo sem projetos\n".into(), 2));
    }

    let items = py_iter(&projects)?;
    let mut problems: Vec<String> = Vec::new();
    let null = Value::Null;
    for project in &items {
        let name = py_str(get_or(project, "name", &json!("?"))?);
        let website = get_or(project, "website", &null)?;
        let primary = get_or(project, "primary_cta", &null)?;
        let secondary = get_or(project, "secondary_cta", &null)?;
        let has_site = json_truthy(website);
        let private = json_truthy(get_or(project, "private", &null)?);
        let status = get_or(project, "website_status", &null)?;
        let verified = *status == "verified";

        if has_site && *primary != "website" {
            problems.push(format!("{name}: tem site mas primary_cta é {}", py_repr(primary)));
        }
        if has_site && *secondary != "github" {
            problems.push(format!("{name}: tem site mas secondary_cta é {}", py_repr(secondary)));
        }
        if !has_site && *primary != "github" {
            problems.push(format!("{name}: sem site mas primary_cta é {}", py_repr(primary)));
        }
        if !has_site && !secondary.is_null() {
            problems.push(format!("{name}: sem site mas tem secondary_cta {}", py_repr(secondary)));
        }
        if private && has_site {
            problems.push(format!("{name}: repositório privado não pode anunciar site público"));
        }
        if has_site && !py_str(website).starts_with("https://") {
            problems.push(format!("{name}: site precisa ser https, veio {}", py_repr(website)));
        }
        if has_site && !verified {
            problems
                .push(format!("{name}: site publicado com status {}; só 'verified' pode aparecer", py_repr(status)));
        }
        let declared = json_truthy(get_or(project, "website_declared", &null)?);
        if declared && !verified && has_site {
            problems.push(format!("{name}: URL fora do ar publicada como site"));
        }
        if private && declared {
            problems.push(format!("{name}: repositório privado não declara site público"));
        }
    }

    let readme_path = root.join("README.md");
    if readme_path.exists() {
        let text = read_text(&readme_path)?;
        for marker in SHOWCASE_MARKERS {
            let (start, end) = marker_pair(marker);
            let body = block(&text, &start, &end).unwrap_or("");
            for line in py_splitlines(body) {
                if !line.contains("](http") {
                    continue;
                }
                let site_pos = find(line, "Abrir site").max(find(line, "ABRIR%20SITE"));
                let code_pos = find(line, "[código]").max(find(line, "C%C3%93DIGO"));
                if site_pos == -1 || code_pos == -1 {
                    continue;
                }
                if code_pos < site_pos {
                    problems.push(format!(
                        "{marker}: código aparece antes do site em «{}…»",
                        py_prefix(py_strip(line), 70)
                    ));
                }
            }
        }
    }

    if !problems.is_empty() {
        let mut stderr = String::from("validação de CTA falhou:\n");
        for problem in &problems {
            stderr.push_str(&format!("  - {problem}\n"));
        }
        return Ok(Outcome::fail(stderr, 1));
    }
    let with_site = items.iter().filter(|project| project.get("website").is_some_and(json_truthy)).count();
    Ok(Outcome::pass(format!(
        "project-links validation: pass · {} projetos, {with_site} com site como CTA primário\n",
        items.len()
    )))
}

// ---------------------------------------------------------------- visual

const VISUAL_MARKERS: [&str; 7] = [
    "PROFILE-DASHBOARD",
    "FEATURED-PROJECTS",
    "LIVE-PROJECTS",
    "PROJECT-MAP",
    "PUBLIC-PROJECTS",
    "PRIVATE-PROJECTS",
    "LANGUAGE-STATS",
];

const VISUAL_TOKENS: [&str; 11] = [
    "capsule-render",
    "readme-typing-svg",
    "skillicons.dev",
    "assets/profile-projects.svg",
    "assets/jarvis-console.svg",
    "assets/profile-stats.svg",
    "assets/profile-top-langs.svg",
    "assets/profile-streak.svg",
    "assets/profile-trophies.svg",
    "github-contribution-grid-snake-dark.svg",
    "komarev.com/ghpvc",
];

const VISUAL_ASSETS: [&str; 7] = [
    "jarvis-console.svg",
    "lang-stats.svg",
    "profile-projects.svg",
    "profile-stats.svg",
    "profile-streak.svg",
    "profile-top-langs.svg",
    "profile-trophies.svg",
];

/// `validate_restored_style.py`: a identidade visual do README continua lá.
pub fn visual(root: &Path) -> Result<Outcome, Crash> {
    let root = resolve_root(root);
    let text = read_text(&root.join("README.md"))?;
    let mut errors: Vec<String> = Vec::new();
    for marker in VISUAL_MARKERS {
        let (start, end) = marker_pair(marker);
        if text.matches(&start).count() != 1 || text.matches(&end).count() != 1 {
            errors.push(format!("marker pair: {marker}"));
        }
    }
    for token in VISUAL_TOKENS {
        if !text.contains(token) {
            errors.push(format!("missing original visual component: {token}"));
        }
    }
    for asset in VISUAL_ASSETS {
        let path = root.join("assets").join(asset);
        if !path.is_file() || std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0) == 0 {
            errors.push(format!("missing asset: {asset}"));
        }
    }
    match load_json(&root.join("docs/README_SITES.json")) {
        Ok(_) => {}
        Err(error) if error.kind == "Unsupported" => return Err(error),
        Err(error) => errors.push(format!("README_SITES.json: {}", error.message)),
    }
    let report = json!({
        "errors": errors,
        "visual_components": 11,
        "readme_bytes": text.len(),
        "public_count_markers": text.matches("| **").count(),
    });
    let stdout = format!("{}\n", py_dumps(&report, false));
    Ok(Outcome { stdout, stderr: String::new(), code: u8::from(!errors.is_empty()) })
}

// ---------------------------------------------------------------- badges

const EXPECTED_CATEGORIES: [&str; 4] =
    ["Frameworks & Web", "Infraestrutura & DevOps", "IA & Conhecimento", "Hardware & Simulação"];

/// O que a parte offline do `validate_language_badges.py` achou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Badges {
    /// Rótulo e URL de cada badge.
    pub badges: Vec<(String, String)>,
    /// Categorias casadas no Arsenal (com repetição).
    pub categories: usize,
}

/// Uma falha tratada pelo `main()` do script (`AssertionError` ou `OSError`).
fn assertion(message: impl Into<String>) -> BadgeFailure {
    BadgeFailure::Handled(message.into())
}

/// Como a parte offline pode falhar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BadgeFailure {
    /// `language badge validation failed: …` (código 1).
    Handled(String),
    /// Traceback.
    Crash(Crash),
}

impl From<Crash> for BadgeFailure {
    fn from(crash: Crash) -> Self {
        Self::Crash(crash)
    }
}

fn badges_block<'a>(text: &'a str, marker: &str) -> Result<&'a str, BadgeFailure> {
    let (start, end) = marker_pair(marker);
    block(text, &start, &end).ok_or_else(|| assertion(format!("marcadores ausentes: {start} / {end}")))
}

/// `assert_language_badges`, `assert_categories` e
/// `assert_language_count_matches` — tudo menos a rede.
pub fn badges_offline(root: &Path) -> Result<Badges, BadgeFailure> {
    let root = resolve_root(root);
    let readme = match read_text(&root.join("README.md")) {
        Ok(text) => text,
        // O `main()` do script trata `OSError`; `UnicodeDecodeError` vira traceback.
        Err(error) if is_os_error(&error) => return Err(assertion(error.message)),
        Err(error) => return Err(error.into()),
    };

    let badge = Regex::new(r"\[!\[([^\]]+)\]\((https://img\.shields\.io/badge/[^)]+)\)\]").expect("regex fixa");
    let badges: Vec<(String, String)> = badge
        .captures_iter(badges_block(&readme, "LANGUAGE-BADGES")?)
        .map(|captures| (captures[1].to_string(), captures[2].to_string()))
        .collect();
    if badges.is_empty() {
        return Err(assertion("nenhum badge de linguagem encontrado"));
    }
    let labels: Vec<&str> = badges.iter().map(|(label, _)| label.as_str()).collect();
    let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
    if unique.len() != labels.len() {
        return Err(assertion("há labels de linguagem duplicados"));
    }
    if labels.iter().any(|label| label.contains("%23") || label.contains("%2F")) {
        return Err(assertion("label percent-encoded visível no Markdown"));
    }
    if !labels.contains(&"C#") || !labels.contains(&"PL/pgSQL") {
        return Err(assertion("C# e PL/pgSQL precisam permanecer legíveis"));
    }
    for (label, url) in &badges {
        let parsed =
            site_monitor::pyurl::urlparse(url, "", true).map_err(|error| PyException::new("ValueError", error.0))?;
        if parsed.netloc != "img.shields.io" || !parsed.path.starts_with("/badge/") {
            return Err(assertion(format!("endpoint inesperado para {label}: {url}")));
        }
        if !parsed.query.contains("style=flat-square") || !parsed.query.contains("labelColor=0e0c16") {
            return Err(assertion(format!("estilo incompleto para {label}: {url}")));
        }
    }

    let category = Regex::new(
        r"(?m)^### [^\n]* (Frameworks & Web|Infraestrutura & DevOps|IA & Conhecimento|Hardware & Simulação)$",
    )
    .expect("regex fixa");
    let found: Vec<&str> = category
        .captures_iter(badges_block(&readme, "ARSENAL-STACK")?)
        .map(|captures| captures.get(1).expect("grupo 1").as_str())
        .collect();
    let mut missing: Vec<&str> = EXPECTED_CATEGORIES.into_iter().filter(|name| !found.contains(name)).collect();
    if !missing.is_empty() {
        missing.sort_unstable();
        return Err(assertion(format!("categorias ausentes no Arsenal: {}", missing.join(", "))));
    }

    let row = Regex::new(r"(?m)^\| \d+ \| \*\*[^|]+\*\* \|").expect("regex fixa");
    let rows = row.find_iter(badges_block(&readme, "LANGUAGE-STATS")?).count();
    if rows != badges.len() {
        return Err(assertion(format!("badges={}, linhas da matriz={rows}", badges.len())));
    }
    Ok(Badges { badges, categories: found.len() })
}

/// Sufixo da mensagem de sucesso quando as URLs não foram conferidas.
pub const BADGES_OFFLINE_NOTE: &str = "badge URLs not checked (offline)";
/// Sufixo da mensagem de sucesso do script, depois de conferir as URLs.
pub const BADGES_ONLINE_NOTE: &str = "HTTP 2xx/3xx for all badge URLs";

fn badges_failed(message: &str) -> Outcome {
    Outcome::fail(format!("language badge validation failed: {message}\n"), 1)
}

/// As URLs dos badges que não responderam, no texto `url (erro)` do script
/// (3 tentativas, 15 s, 8 simultâneas; o transporte é o do `site-monitor`).
async fn unavailable_badges(badges: &Badges) -> Result<Vec<String>, Crash> {
    let options = site_monitor::Options { timeout: Duration::from_secs(15), retries: 2, max_workers: 8 };
    let checker =
        site_monitor::Checker::new(options).map_err(|error| PyException::new("RuntimeError", error.to_string()))?;
    let checks = checker.check_websites(badges.badges.iter().map(|(_, url)| url.clone())).await;
    let mut failures: Vec<String> = badges
        .badges
        .iter()
        .filter_map(|(_, url)| {
            let check = checks.get(url)?;
            (check.status != site_monitor::Status::Verified).then(|| {
                let reason = check.error_message.clone().unwrap_or_else(|| format!("HTTP {}", check.http_status));
                format!("{url} ({reason})")
            })
        })
        .collect();
    failures.sort();
    Ok(failures)
}

/// `validate_language_badges.py`; sem `online`, as URLs não são conferidas
/// (e a mensagem de sucesso diz isso).
pub async fn badges(root: &Path, online: bool) -> Result<Outcome, Crash> {
    let found = match badges_offline(root) {
        Ok(found) => found,
        Err(BadgeFailure::Handled(message)) => return Ok(badges_failed(&message)),
        Err(BadgeFailure::Crash(crash)) => return Err(crash),
    };
    if online {
        let unavailable = unavailable_badges(&found).await?;
        if !unavailable.is_empty() {
            return Ok(badges_failed(&format!("badges indisponíveis: {}", unavailable.join("; "))));
        }
    }
    Ok(Outcome::pass(format!(
        "language badge validation passed: {} badges, {} tool categories, {}\n",
        found.badges.len(),
        found.categories,
        if online { BADGES_ONLINE_NOTE } else { BADGES_OFFLINE_NOTE }
    )))
}

// --------------------------------------------------------------- catalog

const PROFILE_MARKERS: [&str; 13] = [
    "PROFILE-DASHBOARD",
    "WHAT-I-BUILD",
    "PRODUCT-CARDS",
    "FEATURED-PROJECTS",
    "ARSENAL-STACK",
    "LANGUAGE-BADGES",
    "WEBSITE-DIRECTORY",
    "LIVE-PROJECTS",
    "ECOSYSTEM-MAP",
    "PROJECT-MAP",
    "PUBLIC-PROJECTS",
    "PRIVATE-PROJECTS",
    "LANGUAGE-STATS",
];

const PROFILE_MANIFESTS: [&str; 5] =
    ["README_SITES.json", "README_FEATURED.json", "README_STACK.json", "README_EXCLUDED.json", "project-catalog.json"];

const FORBIDDEN: [&str; 5] = ["sk-", "ghp_", "BEGIN RSA PRIVATE KEY", "postgresql://", "mysql://"];

/// `set(value)` de valores JSON, como membros comparáveis a texto.
fn py_set_of_texts(value: &Value) -> Result<Vec<Value>, Crash> {
    let items = py_iter(value)?;
    for item in &items {
        if matches!(item, Value::Array(_) | Value::Object(_)) {
            return Err(PyException::new("TypeError", format!("unhashable type: '{}'", py_type_name(item))));
        }
    }
    Ok(items)
}

/// Os erros do `validate_profile.py` que dizem respeito ao gerador: pares
/// de marcadores, imagens locais, manifestos, exclusões, coerência do
/// catálogo com o README e padrões de segredo.
pub fn catalog(root: &Path) -> Result<Vec<String>, Crash> {
    let root = resolve_root(root);
    let text = read_text(&root.join("README.md"))?;
    let mut errors: Vec<String> = Vec::new();

    for marker in PROFILE_MARKERS {
        let (start, end) = marker_pair(marker);
        let paired = text.matches(&start).count() == 1
            && text.matches(&end).count() == 1
            && text.find(&start) <= text.find(&end);
        if !paired {
            errors.push(format!("invalid marker pair: {marker}"));
        }
    }

    let image = Regex::new(r"!\[[^\]]*\]\((\./[^)]+)\)").expect("regex fixa");
    for captures in image.captures_iter(&text) {
        let image = &captures[1];
        if !root.join(&image[2..]).exists() {
            errors.push(format!("missing local image: {image}"));
        }
    }

    for name in PROFILE_MANIFESTS {
        match load_json(&root.join("docs").join(name)) {
            Ok(Value::Object(_)) => {}
            Ok(_) => errors.push(format!("{name} is not an object")),
            Err(error) if error.kind == "Unsupported" => return Err(error),
            Err(error) => errors.push(format!("{name} error: {}", error.message)),
        }
    }

    let mut excluded: Vec<String> = Vec::new();
    let loaded = load_json(&root.join("docs/README_EXCLUDED.json")).and_then(|data| {
        // `{str(name) for name in ...}`: vira texto antes de entrar no set.
        Ok(py_iter(get_or(&data, "repositories", &json!([]))?)?.iter().map(py_str).collect::<Vec<_>>())
    });
    match loaded {
        Ok(names) => {
            for name in names {
                if !excluded.contains(&name) {
                    excluded.push(name);
                }
            }
        }
        Err(error) if error.kind == "Unsupported" => return Err(error),
        Err(_) => errors.push("README_EXCLUDED.json could not be loaded".into()),
    }
    for name in &excluded {
        if text.contains(name.as_str()) {
            errors.push(format!("excluded repository appears in README: {name}"));
        }
    }

    let catalog_path = root.join("docs/project-catalog.json");
    if !catalog_path.exists() {
        errors.push("docs/project-catalog.json ausente; rode scripts/update_profile.py --write".into());
    } else {
        let catalog = match load_json(&catalog_path) {
            Ok(value) => value,
            Err(error) if error.kind == "Unsupported" => return Err(error),
            Err(error) => {
                errors.push(format!("project-catalog.json inválido: {}", error.message));
                json!({})
            }
        };
        let projects = get_or(&catalog, "projects", &json!([]))?.clone();
        if !json_truthy(&projects) {
            errors.push("project-catalog.json sem projetos".into());
        }
        let valid = py_set_of_texts(get_or(&catalog, "categories", &json!([]))?)?;
        let null = Value::Null;
        let mut seen_repos: Vec<String> = Vec::new();
        let mut seen_slugs: Vec<String> = Vec::new();
        let mut sites: Vec<(String, String)> = Vec::new();
        let items = py_iter(&projects)?;
        for project in &items {
            let name = py_str(get_or(project, "name", &json!("?"))?);
            let repository = py_str(get_or(project, "repository", &json!(""))?);
            let slug = py_str(get_or(project, "slug", &json!(""))?);
            if seen_repos.contains(&repository) {
                errors.push(format!("repositório duplicado no catálogo: {repository}"));
            }
            seen_repos.push(repository.clone());
            if seen_slugs.contains(&slug) {
                errors.push(format!("slug duplicado no catálogo: {slug}"));
            }
            seen_slugs.push(slug);
            if excluded.contains(&name) || excluded.contains(&repository) {
                errors.push(format!("repositório excluído presente no catálogo: {name}"));
            }
            let category = py_str(get_or(project, "category", &json!(""))?);
            if !valid.is_empty() && !valid.iter().any(|member| *member == category.as_str()) {
                errors.push(format!("{name}: categoria fora da taxonomia: {}", py_repr(&json!(category))));
            }
            let website = get_or(project, "website", &null)?;
            if json_truthy(website) {
                let url = py_str(website);
                if !url.starts_with("https://") {
                    errors.push(format!("{name}: site publicado sem https: {url}"));
                }
                if *get_or(project, "website_status", &null)? != "verified" {
                    errors.push(format!("{name}: site publicado sem verificação"));
                }
                if json_truthy(get_or(project, "private", &null)?) {
                    errors.push(format!("{name}: repositório privado com site público no catálogo"));
                }
                let previous = sites.iter().rev().find(|(site, _)| *site == url).map(|(_, owner)| owner.clone());
                if let Some(previous) = previous.filter(|owner| !owner.is_empty()) {
                    errors.push(format!("URL repetida em dois projetos: {url} ({previous} e {name})"));
                }
                sites.push((url.clone(), name.clone()));
                if !text.contains(url.as_str()) {
                    errors.push(format!("{name}: site do catálogo não aparece no README"));
                }
            }
            if !json_truthy(get_or(project, "github_visible", &json!(false))?) {
                errors.push(format!("{name}: github_visible precisa ser true"));
            }
        }
        let mut known: Vec<String> = sites.into_iter().map(|(site, _)| site).collect();
        for project in &items {
            if let Some(declared) = project.get("website_declared").filter(|value| json_truthy(value)) {
                known.push(py_str(declared));
            }
        }
        // `\s` do Python em texto: o White_Space do Unicode e U+001C–U+001F.
        let announced = Regex::new(r"\[▸ Abrir site\]\((https?://[^)\s\x{1c}-\x{1f}]+)\)").expect("regex fixa");
        for captures in announced.captures_iter(&text) {
            if !known.iter().any(|site| site == &captures[1]) {
                errors.push(format!("README anuncia site ausente do catálogo: {}", &captures[1]));
            }
        }
    }

    let lowered = text.to_lowercase();
    for token in FORBIDDEN {
        if lowered.contains(&token.to_lowercase()) {
            errors.push(format!("possible sensitive token pattern in README: {token}"));
        }
    }
    Ok(errors)
}

/// A saída do `validate catalog`.
pub fn catalog_outcome(errors: &[String]) -> Outcome {
    if errors.is_empty() {
        return Outcome::pass("catalog validation: pass\n".into());
    }
    let mut stderr = format!("catalog validation failed: {} erro(s)\n", errors.len());
    for error in errors {
        stderr.push_str(&format!("  - {error}\n"));
    }
    Outcome::fail(stderr, 1)
}
