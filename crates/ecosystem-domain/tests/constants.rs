//! Dado editorial que ainda mora no código Python e foi copiado para cá: as
//! duas cópias não podem divergir enquanto o Python publicar o README.

use std::path::Path;

use ecosystem_domain::lifecycle::{ALWAYS_IN_DEVELOPMENT, BACKLOG_EXCEPTION, BACKLOG_PREFIX};
use ecosystem_domain::presentation::FEATURED_SUMMARIES;
use regex::Regex;

fn update_profile() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/update_profile.py")).unwrap()
}

#[test]
fn resumos_iguais_aos_do_python() {
    let source = update_profile();
    let start = source.find("FEATURED_SUMMARIES = {").expect("dicionário no Python");
    let block = &source[start..start + source[start..].find("\n}\n").expect("fim do dicionário")];
    let entry = Regex::new(r#"(?m)^\s+"([^"]+)": "([^"]*)",$"#).unwrap();
    let python: Vec<(String, String)> =
        entry.captures_iter(block).map(|c| (c[1].to_string(), c[2].to_string())).collect();
    let rust: Vec<(String, String)> =
        FEATURED_SUMMARIES.iter().map(|(name, summary)| (name.to_string(), summary.to_string())).collect();
    assert_eq!(python, rust);
}

#[test]
fn nomes_fixos_do_status_iguais_aos_do_python() {
    let source = update_profile();
    let names = ALWAYS_IN_DEVELOPMENT.map(|name| format!("\"{name}\"")).join(", ");
    assert!(source.contains(&format!("if name in {{{names}}}:")), "status_for() mudou os nomes fixos");
    assert!(source.contains(&format!("if name.startswith(\"{BACKLOG_PREFIX}\") and name != \"{BACKLOG_EXCEPTION}\":")));
}
