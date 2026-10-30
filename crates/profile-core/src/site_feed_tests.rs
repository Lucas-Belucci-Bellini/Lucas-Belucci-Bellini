use super::{stack::build_stack_inventory, support::public_projects};
use serde_json::{Value, json};

fn public_project() -> Value {
    json!({
        "name": "Public Demo",
        "slug": "public-demo",
        "description": "A public sample project.",
        "category": "Web & SaaS",
        "category_label": "Web",
        "status": "🟢 Active",
        "github": "https://github.com/example/public-demo",
        "private": false,
        "github_visible": true,
        "website_status": "verified",
        "website": "https://example.com",
        "private_internal_note": "must not be emitted"
    })
}

fn catalog_fixture() -> Value {
    json!({
        "schema": "lucas-belucci-bellini/project-catalog@1",
        "categories": ["Web & SaaS"],
        "projects": [
            public_project(),
            {
                "name": "Private Atlas",
                "slug": "private-atlas",
                "private": true,
                "github_visible": false
            }
        ]
    })
}

#[test]
fn public_catalog_is_allowlisted_and_private_projects_are_excluded() {
    let (projects, private_names) = public_projects(&catalog_fixture()).expect("catalog");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["name"], "Public Demo");
    assert!(projects[0].get("private_internal_note").is_none());
    assert!(projects[0].get("private").is_none());
    assert!(private_names.contains("Private Atlas"));
}

#[test]
fn github_repository_names_may_contain_dots() {
    let mut catalog = catalog_fixture();
    catalog["projects"][0]["github"] = Value::String("https://github.com/example/repo.name".into());
    let (projects, _) = public_projects(&catalog).expect("catalog");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["github"], "https://github.com/example/repo.name");
}

#[test]
fn rejects_github_urls_with_extra_path_segments() {
    let mut catalog = catalog_fixture();
    catalog["projects"][0]["github"] =
        Value::String("https://github.com/example/repo.name/extra".into());
    assert!(public_projects(&catalog).is_err());
}

#[test]
fn only_verified_https_websites_are_emitted() {
    let mut catalog = catalog_fixture();
    catalog["projects"][0]["website"] = Value::String("http://example.com".into());
    let (projects, _) = public_projects(&catalog).expect("catalog");
    assert!(projects[0].get("website").is_none());
}

#[test]
fn private_names_are_redacted_from_public_tool_evidence() {
    let archive = concat!(
        "<!-- LANGUAGE-STATS:START -->\n",
        "**1 linguagens** · **1 repositórios públicos** · **1.00 MB de código detectado** · atualizado em \x602026-10-16\x60\n",
        "| # | Linguagem | Peso | Participação | Repositórios |\n",
        "|:--:|:---|---:|---:|---:|\n",
        "| 1 | **TypeScript** | \x601 MB\x60 | \x60100.00%\x60 | 1 |\n",
        "<!-- LANGUAGE-STATS:END -->\n"
    );
    let stack = json!({
        "tools": [{
            "name": "Vite",
            "category": "Frameworks & Web",
            "family": "build tool",
            "evidence": "Private Atlas and Public Demo use Vite."
        }]
    });
    let (projects, private_names) = public_projects(&catalog_fixture()).expect("catalog");
    let inventory = build_stack_inventory(archive, &stack, &projects, &private_names).expect("inventory");
    let serialized = inventory.to_string();
    assert!(!serialized.contains("Private Atlas"));
    assert!(serialized.contains("Public Demo"));
    assert_eq!(inventory["languages"][0]["name"], "TypeScript");
    assert_eq!(inventory["tools"][0]["projects"][0]["name"], "Public Demo");
}

#[test]
fn rejects_incomplete_language_matrix() {
    let archive = concat!(
        "<!-- LANGUAGE-STATS:START -->\n",
        "**2 linguagens** · **1 repositórios públicos** · **1.00 MB de código detectado** · atualizado em \x602026-10-16\x60\n",
        "| 1 | **TypeScript** | \x601 MB\x60 | \x60100.00%\x60 | 1 |\n",
        "<!-- LANGUAGE-STATS:END -->\n"
    );
    let stack = json!({"tools": []});
    let (projects, private_names) = public_projects(&catalog_fixture()).expect("catalog");
    assert!(build_stack_inventory(archive, &stack, &projects, &private_names).is_err());
}
