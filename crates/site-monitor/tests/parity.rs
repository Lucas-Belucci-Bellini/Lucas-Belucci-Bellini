//! Paridade com o `urllib` do CPython 3.12.14: grupos `preflight` e `redirect`
//! de `tests/fixtures/parity/site_monitor.json` (gerado por
//! `tests/test_parity_site_monitor.py` chamando a biblioteca padrão de verdade).
//! Os grupos `collect_urls` e `report` são conferidos pelo `profile-core`.

use std::path::PathBuf;

use serde_json::{Value, json};
use site_monitor::USER_AGENT;
use site_monitor::pyrequest::{PreflightError, PyRequest, preflight};
use site_monitor::redirect::{RedirectStep, redirect_step};

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/parity/site_monitor.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).expect("fixture é JSON")
}

fn error_json(error: &PreflightError) -> Value {
    json!({ "error": error.kind(), "caught": matches!(error, PreflightError::Caught { .. }) })
}

fn preflight_json(url: &str) -> Value {
    let result = PyRequest::new(url).and_then(|request| preflight(&request));
    match result {
        Ok(target) => json!({
            "ok": true,
            "full_url": target.full_url,
            "connect_host": target.connect_host,
            "port": target.port,
            "request_target": target.request_target,
            "host_header": target.host_header,
        }),
        Err(error) => {
            let mut value = error_json(&error);
            value["ok"] = json!(false);
            value
        }
    }
}

fn redirect_json(base: &str, location: Option<&str>, uri: Option<&str>) -> Value {
    match redirect_step(base, location, uri) {
        RedirectStep::Follow { visited_key, request } => {
            json!({ "kind": "follow", "visited_key": visited_key, "full_url": request.full_url })
        }
        RedirectStep::NotAllowed(url) => json!({ "kind": "not_allowed", "url": url }),
        RedirectStep::NoLocation => json!({ "kind": "no_location" }),
        RedirectStep::Failed(error) => {
            let mut value = error_json(&error);
            value["kind"] = json!("error");
            value
        }
    }
}

/// Compara ignorando a ordem das chaves (o Python grava `ok` antes, o Rust depois).
fn diverged(group: &str, cases: &[Value], actual: impl Fn(&Value) -> Value) -> Vec<String> {
    cases
        .iter()
        .filter_map(|case| {
            let got = actual(&case["input"]);
            (got != case["expected"])
                .then(|| format!("{group}: {}\n    Python: {}\n    Rust:   {got}", case["input"], case["expected"]))
        })
        .collect()
}

#[test]
fn rust_reproduz_o_urllib_em_todos_os_casos() {
    let fixture = fixture();
    assert_eq!("lucas-belucci-bellini/parity-site-monitor@1", fixture["schema"]);
    assert_eq!(USER_AGENT, fixture["user_agent"], "User-Agent igual ao do Python");
    let cases = fixture["cases"].as_object().expect("grupos");

    let mut failures = Vec::new();
    let preflight_cases = cases["preflight"].as_array().expect("preflight");
    failures.extend(diverged("preflight", preflight_cases, |input| preflight_json(input.as_str().expect("url"))));
    let redirect_cases = cases["redirect"].as_array().expect("redirect");
    failures.extend(diverged("redirect", redirect_cases, |input| {
        redirect_json(input["base"].as_str().expect("base"), input["location"].as_str(), input["uri"].as_str())
    }));

    for group in cases.keys() {
        assert!(
            ["preflight", "redirect", "collect_urls", "report"].contains(&group.as_str()),
            "grupo `{group}` gerado pelo Python sem conferência no Rust"
        );
    }
    assert!(preflight_cases.len() + redirect_cases.len() > 60, "fixture encolheu");
    assert!(failures.is_empty(), "Rust diverge do urllib:\n{}", failures.join("\n"));
}
