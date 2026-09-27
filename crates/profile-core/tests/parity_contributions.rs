//! A coleta de contribuições contra `tests/fixtures/parity/contributions.json`:
//! as janelas do `period_ranges()` e cada execução do `main()` do Python —
//! mesmos pedidos, na mesma ordem, e os mesmos bytes no JSON e no HTML (ou a
//! mesma mensagem de erro).

use std::path::Path;
use std::sync::Mutex;

use chrono::NaiveDate;
use github_client::ApiError;
use profile_core::contributions::{self, GraphQl};
use serde_json::Value;

fn fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/parity/contributions.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Responde cada pedido com a resposta que o Python recebeu para as mesmas variáveis.
struct Replay {
    exchanges: Vec<Value>,
    asked: Mutex<Vec<Value>>,
}

impl GraphQl for Replay {
    async fn query(&self, variables: Value) -> Result<Value, ApiError> {
        self.asked.lock().unwrap().push(variables.clone());
        let exchange = self
            .exchanges
            .iter()
            .find(|e| e["variables"] == variables)
            .unwrap_or_else(|| panic!("o Python não fez este pedido: {variables}"));
        match exchange["status"].as_u64().unwrap() {
            200 => Ok(exchange["body"].clone()),
            status => Err(ApiError::Http {
                status: status as u16,
                reason: String::new(),
                retry_after: None,
                ratelimit_remaining: None,
            }),
        }
    }
}

fn date(value: &Value) -> NaiveDate {
    NaiveDate::parse_from_str(value.as_str().unwrap(), "%Y-%m-%d").unwrap()
}

#[test]
fn janelas_mensais() {
    for case in fixture()["periods"].as_array().unwrap() {
        let expected: Vec<(NaiveDate, NaiveDate)> =
            case["expected"].as_array().unwrap().iter().map(|pair| (date(&pair[0]), date(&pair[1]))).collect();
        assert_eq!(expected, contributions::period_ranges(date(&case["input"][0]), date(&case["input"][1])), "{case}");
    }
}

#[tokio::test]
async fn execucoes_do_python() {
    let mut failures = Vec::new();
    for run in fixture()["runs"].as_array().unwrap() {
        let name = run["name"].as_str().unwrap();
        let input = &run["input"];
        let expected = &run["expected"];
        if !input["token"].as_bool().unwrap() {
            continue; // sem token: coberto pelo teste do binário
        }
        let replay = Replay { exchanges: input["exchanges"].as_array().unwrap().clone(), asked: Mutex::default() };
        let now = catalog::parse_now(input["now"].as_str().unwrap()).unwrap();
        let result = contributions::collect(&replay, input["login"].as_str().unwrap(), now).await;
        let asked: Vec<Value> = replay.asked.lock().unwrap().clone();
        let sent: Vec<Value> = input["exchanges"].as_array().unwrap().iter().map(|e| e["variables"].clone()).collect();
        if asked != sent {
            failures.push(format!("{name}: pedidos diferentes"));
        }
        match result {
            Ok(payload) => {
                if expected["exit"] != 0 {
                    failures.push(format!("{name}: o Python falhou ({}), o Rust não", expected["stderr"]));
                }
                if expected["data"].as_str() != Some(contributions::render_data(&payload).as_str()) {
                    failures.push(format!("{name}: JSON difere"));
                }
                if expected["html"].as_str() != Some(contributions::render_html(&payload).as_str()) {
                    failures.push(format!("{name}: HTML difere"));
                }
            }
            Err(message) => {
                let stderr = format!("timeline generation failed: {message}\n");
                if expected["stderr"].as_str() != Some(stderr.as_str()) || !expected["data"].is_null() {
                    failures.push(format!("{name}: python {} / rust {stderr:?}", expected["stderr"]));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
