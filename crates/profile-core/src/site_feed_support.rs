use super::{STATUSES, SiteError, object, sort_json};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub(super) fn required_string<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a str, SiteError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| SiteError::Invalid(format!("{context}.{key} must be a non-empty string")))
}

fn safe_https_url(value: &str) -> bool {
    if value.trim() != value || value.chars().any(char::is_whitespace) || value.contains('\\') {
        return false;
    }
    let Some(rest) = value.strip_prefix("https://") else { return false };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    let host = authority.split(':').next().unwrap_or_default().to_ascii_lowercase();
    !host.is_empty() && host != "localhost" && host != "localhost.localdomain" && !host.ends_with(".local")
}

fn github_repository_url(value: &str) -> bool {
    if !safe_https_url(value) {
        return false;
    }
    let Some(rest) = value.strip_prefix("https://") else { return false };
    let mut parts = rest.splitn(2, '/');
    let host = parts.next().unwrap_or_default().to_ascii_lowercase();
    if host != "github.com" && host != "www.github.com" {
        return false;
    }
    let Some(path) = parts.next() else { return false };
    let path = path.split(['?', '#']).next().unwrap_or_default().trim_end_matches('/');
    let segments: Vec<_> = path.split('/').collect();
    segments.len() == 2
        && segments.iter().all(|segment| !segment.is_empty() && *segment != "." && *segment != ".." && !segment.contains('@'))
}

fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

pub(super) fn public_projects(catalog: &Value) -> Result<(Vec<Value>, BTreeSet<String>), SiteError> {
    if catalog.get("schema").and_then(Value::as_str) != Some("lucas-belucci-bellini/project-catalog@1") {
        return Err(SiteError::Invalid("unsupported project catalog schema".into()));
    }
    let source_categories = catalog
        .get("categories")
        .and_then(Value::as_array)
        .ok_or_else(|| SiteError::Invalid("catalog.categories must be an array".into()))?;
    let mut categories = BTreeSet::new();
    for (index, category) in source_categories.iter().enumerate() {
        let category = category
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| SiteError::Invalid(format!("categories[{index}] must be a non-empty string")))?;
        categories.insert(category.to_string());
    }
    let source_projects = catalog
        .get("projects")
        .and_then(Value::as_array)
        .ok_or_else(|| SiteError::Invalid("catalog.projects must be an array".into()))?;

    let mut private_names = BTreeSet::new();
    for project in source_projects {
        if project.get("private").and_then(Value::as_bool) == Some(true) {
            if let Some(name) = project.get("name").and_then(Value::as_str) {
                if !name.trim().is_empty() {
                    private_names.insert(name.trim().to_string());
                }
            }
        }
    }

    let mut result = Vec::new();
    let mut seen_slugs = BTreeSet::new();
    for (index, project) in source_projects.iter().enumerate() {
        let context = format!("projects[{index}]");
        let private = project
            .get("private")
            .and_then(Value::as_bool)
            .ok_or_else(|| SiteError::Invalid(format!("{context}.private must be a boolean")))?;
        if private || project.get("github_visible").and_then(Value::as_bool) != Some(true) {
            continue;
        }

        let name = required_string(project, "name", &context)?;
        let slug = required_string(project, "slug", &context)?;
        let description = required_string(project, "description", &context)?;
        let category = required_string(project, "category", &context)?;
        let category_label = required_string(project, "category_label", &context)?;
        let status = required_string(project, "status", &context)?;
        if !valid_slug(slug) {
            return Err(SiteError::Invalid(format!("{context}.slug is unsafe")));
        }
        if !seen_slugs.insert(slug.to_string()) {
            return Err(SiteError::Invalid(format!("duplicate public slug: {slug:?}")));
        }
        if !categories.contains(category) {
            return Err(SiteError::Invalid(format!("{context}.category is not declared in catalog.categories")));
        }
        if !STATUSES.contains(&status) {
            return Err(SiteError::Invalid(format!("{context}.status is not an allowed public status")));
        }
        let github = required_string(project, "github", &context)?;
        if !github_repository_url(github) {
            continue;
        }

        let mut item = Map::new();
        item.insert("name".into(), Value::String(name.into()));
        item.insert("slug".into(), Value::String(slug.into()));
        item.insert("description".into(), Value::String(description.into()));
        item.insert("category".into(), Value::String(category.into()));
        item.insert("category_label".into(), Value::String(category_label.into()));
        item.insert("status".into(), Value::String(status.into()));
        item.insert("github".into(), Value::String(github.into()));
        if project.get("website_status").and_then(Value::as_str) == Some("verified") {
            if let Some(website) = project.get("website").and_then(Value::as_str) {
                if safe_https_url(website) {
                    item.insert("website".into(), Value::String(website.into()));
                }
            }
        }
        result.push(sort_json(Value::Object(item)));
    }

    result.sort_by(|left, right| {
        let left_name = left.get("name").and_then(Value::as_str).unwrap_or_default();
        let right_name = right.get("name").and_then(Value::as_str).unwrap_or_default();
        left_name.to_lowercase().cmp(&right_name.to_lowercase()).then_with(|| {
            left.get("slug")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .cmp(right.get("slug").and_then(Value::as_str).unwrap_or_default())
        })
    });
    if result.is_empty() {
        return Err(SiteError::Invalid("no eligible public projects remain after filtering".into()));
    }
    Ok((result, private_names))
}
