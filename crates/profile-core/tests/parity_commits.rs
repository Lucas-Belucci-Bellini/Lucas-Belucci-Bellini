//! O monitor de commits contra `tests/fixtures/parity/monitor.json`: cada
//! cenário é o `main()` do `ecosystem_watch.py` sobre respostas fixas da API;
//! o Rust recebe as mesmas respostas e tem de produzir a mesma saída, o mesmo
//! estado e o mesmo relatório, byte a byte — ou quebrar onde o Python quebra.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use github_client::ApiError;
use profile_core::commits::{self, Identity, Previous, Source};
use serde_json::Value;

struct Fixed(HashMap<String, Value>, Mutex<Vec<String>>);

impl Source for Fixed {
    async fn get(&self, path: &str) -> Result<Value, ApiError> {
        self.1.lock().unwrap().push(path.to_string());
        let Some(response) = self.0.get(path) else {
            return Err(ApiError::Http {
                status: 404,
                reason: "Not Found".into(),
                retry_after: None,
                ratelimit_remaining: None,
            });
        };
        if let Some(json) = response.get("json") {
            return Ok(json.clone());
        }
        if let Some(status) = response.get("http") {
            return Err(ApiError::Http {
                status: status.as_u64().unwrap() as u16,
                reason: response["reason"].as_str().unwrap().into(),
                retry_after: None,
                ratelimit_remaining: None,
            });
        }
        if let Some(reason) = response.get("url_error") {
            return Err(ApiError::Url(reason.as_str().unwrap().into()));
        }
        Err(ApiError::Timeout)
    }
}

fn fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/parity/monitor.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// (stdout, estado reescrito?, relatório, exceção) de um cenário.
async fn run(case: &Value, identity: &Identity, now: &str) -> Value {
    let root = std::env::temp_dir().join(format!("parity-commits-{}-{}", std::process::id(), rand_suffix(case)));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("docs")).unwrap();
    let previous_text = case["input"]["previous"].as_str();
    if let Some(text) = previous_text {
        std::fs::write(root.join(commits::STATE_FILE), text).unwrap();
    }
    let api: HashMap<String, Value> =
        case["input"]["api"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    let now = catalog::parse_now(now).unwrap();

    let mut result = serde_json::Map::new();
    let mut stdout = String::new();
    let source = Fixed(api, Mutex::default());
    let outcome = async {
        let previous = Previous::read(&root)?;
        let scan = commits::scan(&source, identity, &previous).await?;
        if !scan.changed {
            stdout.push_str(ecosystem_domain::monitor::UNCHANGED);
            return Ok(());
        }
        let published = commits::publish(&scan, now);
        commits::write(&root, &published).unwrap();
        published.report.map(|_| ())
    }
    .await;
    if let Err(crash) = outcome {
        result.insert("crash".into(), Value::String(crash.kind.to_string()));
    }
    result.insert("stdout".into(), Value::String(stdout));
    let calls = source.1.lock().unwrap().iter().cloned().map(Value::String).collect();
    result.insert("calls".into(), Value::Array(calls));
    let state = std::fs::read_to_string(root.join(commits::STATE_FILE)).ok();
    let state = if state.as_deref() == previous_text { None } else { state };
    result.insert("state".into(), state.map_or(Value::Null, Value::String));
    let report = std::fs::read_to_string(root.join(commits::REPORT_FILE)).ok();
    result.insert("report".into(), report.map_or(Value::Null, Value::String));
    let _ = std::fs::remove_dir_all(&root);
    Value::Object(result)
}

fn rand_suffix(case: &Value) -> usize {
    case["name"].as_str().unwrap().len() * 7919 + case["input"]["api"].as_object().unwrap().len()
}

#[tokio::test]
async fn cenarios_do_python() {
    let data = fixture();
    let user = data["user"].as_str().unwrap().to_string();
    let identity = Identity { user: user.clone(), profile: user };
    let now = data["now"].as_str().unwrap();
    let mut failures = Vec::new();
    for case in data["cases"].as_array().unwrap() {
        let actual = run(case, &identity, now).await;
        let expected = &case["expected"];
        for key in ["crash", "stdout", "state", "report", "calls"] {
            let (e, a) = (expected.get(key).unwrap_or(&Value::Null), actual.get(key).unwrap_or(&Value::Null));
            if e != a {
                let detail = match (e.as_str(), a.as_str()) {
                    (Some(e), Some(a)) => {
                        let line = e.lines().zip(a.lines()).position(|(x, y)| x != y).unwrap_or(0);
                        format!("linha {}: python {:?} / rust {:?}", line + 1, e.lines().nth(line), a.lines().nth(line))
                    }
                    _ => format!("python {e} / rust {a}"),
                };
                failures.push(format!("{} [{key}]: {detail}", case["name"].as_str().unwrap()));
            }
        }
    }
    assert!(failures.is_empty(), "{} divergência(s):\n{}", failures.len(), failures.join("\n"));
}
