use super::support::required_string;
use super::{CATEGORIES, STACK_SCHEMA, SiteError, object, sort_json};
use regex::{Regex, RegexBuilder};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

fn normalize_evidence(value: &str) -> String {
    let folded = value.to_lowercase().replace('ß', "ss");
    let mut normalized = String::new();
    for character in folded.nfkd() {
        if is_combining_mark(character) {
            continue;
        }
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
        } else if !normalized.ends_with(' ') {
            normalized.push(' ');
        }
    }
    normalized.trim().to_string()
}

fn redact_private_names(value: &str, private_names: &BTreeSet<String>) -> String {
    let mut output = value.to_string();
    for name in private_names {
        if name.is_empty() {
            continue;
        }
        if let Ok(pattern) = RegexBuilder::new(&regex::escape(name)).case_insensitive(true).build() {
            output = pattern.replace_all(&output, "").into_owned();
        }
    }
    output.trim_matches(|character| matches!(character, ' ' | ',' | ';' | ':' | '-')).to_string()
}

fn parse_language_bytes(value: &str) -> Result<u64, SiteError> {
    let pattern =
        Regex::new(r"^([0-9]+(?:\.[0-9]+)?)\s*(B|KB|MB|GB)$").map_err(|error| SiteError::Invalid(error.to_string()))?;
    let captures = pattern
        .captures(value.trim())
        .ok_or_else(|| SiteError::Invalid(format!("unsupported language size: {value:?}")))?;
    let amount: f64 =
        captures[1].parse().map_err(|_| SiteError::Invalid(format!("invalid language size: {value:?}")))?;
    let multiplier = match &captures[2] {
        "B" => 1.0,
        "KB" => 1024.0,
        "MB" => 1024.0 * 1024.0,
        "GB" => 1024.0 * 1024.0 * 1024.0,
        _ => unreachable!(),
    };
    Ok((amount * multiplier).round_ties_even() as u64)
}

pub(super) fn build_stack_inventory(
    archive: &str,
    stack: &Value,
    projects: &[Value],
    private_names: &BTreeSet<String>,
) -> Result<Value, SiteError> {
    let start_marker = "<!-- LANGUAGE-STATS:START -->";
    let end_marker = "<!-- LANGUAGE-STATS:END -->";
    let start = archive
        .find(start_marker)
        .ok_or_else(|| SiteError::Invalid("public language statistics start marker is missing".into()))?
        + start_marker.len();
    let end = archive[start..]
        .find(end_marker)
        .map(|offset| start + offset)
        .ok_or_else(|| SiteError::Invalid("public language statistics end marker is missing".into()))?;
    let section = &archive[start..end];

    let summary_pattern = Regex::new(
        r"\*\*(\d+) linguagens\*\*\s*·\s*\*\*(\d+) repositórios públicos\*\*\s*·\s*\*\*([0-9.]+ MB) de código detectado\*\*\s*·\s*atualizado em \x60([^\x60]+)\x60",
    )
    .map_err(|error| SiteError::Invalid(error.to_string()))?;
    let summary = summary_pattern
        .captures(section)
        .ok_or_else(|| SiteError::Invalid("public language statistics summary is invalid".into()))?;
    let expected_count: usize =
        summary[1].parse().map_err(|_| SiteError::Invalid("language count is invalid".into()))?;
    let public_repository_count: usize =
        summary[2].parse().map_err(|_| SiteError::Invalid("public repository count is invalid".into()))?;

    let language_pattern = Regex::new(
        r"^\|\s*(\d+)\s*\|\s*\*\*(.+?)\*\*\s*\|\s*\x60([^\x60]+)\x60\s*\|\s*\x60([0-9.]+)%\x60\s*\|\s*(\d+)\s*\|$",
    )
    .map_err(|error| SiteError::Invalid(error.to_string()))?;
    let mut languages = Vec::new();
    for line in section.lines() {
        let Some(captures) = language_pattern.captures(line.trim()) else {
            continue;
        };
        let rank: usize = captures[1].parse().map_err(|_| SiteError::Invalid("language rank is invalid".into()))?;
        let name = captures[2].to_string();
        let size = captures[3].to_string();
        let share: f64 =
            captures[4].parse().map_err(|_| SiteError::Invalid(format!("language share is invalid for {name}")))?;
        let repositories: usize =
            captures[5].parse().map_err(|_| SiteError::Invalid(format!("repository count is invalid for {name}")))?;
        if !share.is_finite() || !(0.0..=100.0).contains(&share) {
            return Err(SiteError::Invalid(format!("language share is outside 0..=100 for {name}")));
        }
        languages.push(object([
            ("rank", Value::from(rank)),
            ("name", Value::String(name)),
            ("bytes", Value::from(parse_language_bytes(&size)?)),
            ("size", Value::String(size)),
            ("share", Value::from(share)),
            ("repositories", Value::from(repositories)),
        ]));
    }
    if languages.len() != expected_count {
        return Err(SiteError::Invalid(format!(
            "language matrix is incomplete: expected {expected_count}, found {}",
            languages.len()
        )));
    }

    let source_tools = stack
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| SiteError::Invalid("README_STACK.json must contain a tools array".into()))?;
    let mut tools = Vec::new();
    for (index, tool) in source_tools.iter().enumerate() {
        let context = format!("tools[{index}]");
        let name = required_string(tool, "name", &context)?;
        let category = required_string(tool, "category", &context)?;
        let family = required_string(tool, "family", &context)?;
        let evidence = required_string(tool, "evidence", &context)?;
        if !CATEGORIES.contains(&category) {
            return Err(SiteError::Invalid(format!("{context}.category is unsupported: {category}")));
        }

        let mut public_evidence = redact_private_names(evidence, private_names);
        if public_evidence.is_empty() {
            public_evidence = "Evidência pública não informada.".into();
        }
        let normalized_evidence = normalize_evidence(&public_evidence);
        let mut evidence_projects = Vec::new();
        for project in projects {
            let project_name = project.get("name").and_then(Value::as_str).unwrap_or_default();
            if !project_name.is_empty() && normalized_evidence.contains(&normalize_evidence(project_name)) {
                let mut evidence_project = Map::new();
                evidence_project.insert("name".into(), Value::String(project_name.into()));
                if let Some(github) = project.get("github").and_then(Value::as_str) {
                    evidence_project.insert("github".into(), Value::String(github.into()));
                }
                if let Some(website) = project.get("website").and_then(Value::as_str) {
                    evidence_project.insert("website".into(), Value::String(website.into()));
                }
                evidence_projects.push(sort_json(Value::Object(evidence_project)));
            }
        }
        tools.push(object([
            ("name", Value::String(name.into())),
            ("category", Value::String(category.into())),
            ("family", Value::String(family.into())),
            ("evidence", Value::String(public_evidence)),
            ("projects", Value::Array(evidence_projects)),
        ]));
    }

    Ok(object([
        ("schema", Value::String(STACK_SCHEMA.into())),
        (
            "summary",
            object([
                ("language_count", Value::from(expected_count)),
                ("public_repository_count", Value::from(public_repository_count)),
                ("total_code", Value::String(summary[3].to_string())),
                ("updated_at", Value::String(summary[4].to_string())),
            ]),
        ),
        ("languages", Value::Array(languages)),
        ("tools", Value::Array(tools)),
        (
            "categories",
            Value::Array(CATEGORIES.iter().map(|value| Value::String((*value).into())).collect()),
        ),
        (
            "methodology",
            object([
                (
                    "languages",
                    Value::String("Agregadas dos mapas públicos de linguagens do GitHub; peso em bytes e participação sobre o total detectado.".into()),
                ),
                (
                    "tools",
                    Value::String("Curadoria baseada em documentação pública, manifests e evidências registradas no manifesto do perfil.".into()),
                ),
                (
                    "proficiency",
                    Value::String("A presença de uma tecnologia indica evidência de uso/documentação, não uma certificação nem domínio equivalente.".into()),
                ),
            ]),
        ),
    ]))
}
