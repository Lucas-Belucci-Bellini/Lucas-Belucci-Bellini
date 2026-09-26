//! Paridade com o `scripts/check_websites.py`: grupos `collect_urls` e
//! `report` de `tests/fixtures/parity/site_monitor.json`. As saídas esperadas
//! vêm do `collect_urls()` e do `main()` reais do Python.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use profile_core::sites::{self, CATALOG_FILE, NO_URLS, SITES_FILE};
use serde_json::{Value, json};
use site_monitor::{Status, WebsiteCheck};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/parity/site_monitor.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON")
}

/// Raiz temporária com `docs/`, apagada no fim.
struct Root(PathBuf);

impl Root {
    fn new(catalog: Option<&str>, manifest: Option<&str>) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "parity-sites-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(dir.join("docs")).expect("docs/");
        if let Some(text) = catalog {
            std::fs::write(dir.join(CATALOG_FILE), text).expect("catálogo");
        }
        if let Some(text) = manifest {
            std::fs::write(dir.join(SITES_FILE), text).expect("manifesto");
        }
        Self(dir)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn collect_json(input: &Value) -> Value {
    let root = Root::new(input["catalog"].as_str(), input["manifest"].as_str());
    match sites::collect_urls(&root.0) {
        Ok(urls) => json!({ "urls": urls.into_iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>() }),
        Err(error) => json!({ "crash": error.python }),
    }
}

fn status(text: &str) -> Status {
    match text {
        "verified" => Status::Verified,
        "unreachable" => Status::Unreachable,
        "invalid" => Status::Invalid,
        other => panic!("status desconhecido: {other}"),
    }
}

fn report_json(input: &Value) -> Value {
    let argv: Vec<&str> = input["argv"].as_array().expect("argv").iter().map(|a| a.as_str().expect("arg")).collect();
    let checked_at = input["checked_at"].as_str().expect("checked_at");
    let mut urls = BTreeMap::new();
    let mut checks = HashMap::new();
    for site in input["sites"].as_array().expect("sites") {
        let url = site["website"].as_str().expect("website").to_string();
        urls.insert(site["repository"].as_str().expect("repository").to_string(), url.clone());
        if let Some(check) = site["check"].as_object() {
            let check = WebsiteCheck {
                url: url.clone(),
                status: status(check["status"].as_str().expect("status")),
                http_status: u16::try_from(check["http_status"].as_u64().expect("http")).expect("u16"),
                final_url: check["final_url"].as_str().expect("final_url").into(),
                checked_at: checked_at.into(),
                attempts: 1,
                redirect_count: 0,
                response_time_ms: None,
                error_kind: None,
                error_message: None,
                python_crash: None,
            };
            checks.insert(url, check);
        }
    }
    if urls.is_empty() {
        return json!({ "stdout": NO_URLS, "exit": 0 });
    }
    let rows = sites::build_rows(&urls, &checks);
    let stdout = if argv.contains(&"--json") { sites::render_json(&rows) } else { sites::render_text(&rows) };
    json!({ "stdout": stdout, "exit": sites::exit_code(&rows, argv.contains(&"--fail-on-down")) })
}

#[test]
fn rust_reproduz_o_check_websites_py() {
    let fixture = fixture();
    let mut failures = Vec::new();
    for (group, actual) in [("collect_urls", collect_json as fn(&Value) -> Value), ("report", report_json)] {
        let cases = fixture["cases"][group].as_array().unwrap_or_else(|| panic!("grupo {group}"));
        assert!(!cases.is_empty());
        for case in cases {
            let got = actual(&case["input"]);
            if got != case["expected"] {
                failures
                    .push(format!("{group}: {}\n    Python: {}\n    Rust:   {got}", case["input"], case["expected"]));
            }
        }
    }
    assert!(failures.is_empty(), "Rust diverge do check_websites.py:\n{}", failures.join("\n"));
}
