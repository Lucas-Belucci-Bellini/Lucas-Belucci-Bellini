//! A biblioteca padrão do CPython do CI, caso a caso: `json.loads`, `repr`,
//! `str`, `bytes.decode("utf-8")`, `OSError` e `str.isprintable()`.
//!
//! ```text
//! OLD PYTHON ──▶ tests/fixtures/parity/pytext.json ◀── NEW RUST
//! ```

use std::path::PathBuf;

use ecosystem_domain::pyio::{decode_utf8, os_error};
use ecosystem_domain::pyjson::{JsonError, loads};
use ecosystem_domain::pyrepr::{py_isprintable, py_repr, py_str};
use serde_json::Value;

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/parity/pytext.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture pytext.json")).expect("fixture é JSON")
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap()).collect()
}

#[test]
fn rust_reproduz_a_biblioteca_padrao() {
    let data = fixture();
    assert_eq!("lucas-belucci-bellini/parity-pytext@1", data["schema"]);
    let mut failures = Vec::new();

    let cases = data["json_loads"].as_array().unwrap();
    assert!(cases.len() > 300, "o fixture encolheu: {} casos de json.loads", cases.len());
    for case in cases {
        let text = case["input"].as_str().unwrap();
        let got = loads(text);
        let ok = match (&got, case.get("error"), case.get("repr"), case.get("unsupported")) {
            (Err(JsonError::Decode { .. }), Some(expected), _, _) => got.as_ref().unwrap_err().to_string() == *expected,
            (Ok(value), _, Some(expected), _) => py_repr(value) == *expected,
            (Err(JsonError::Unsupported { .. }), _, _, Some(_)) => true,
            _ => false,
        };
        if !ok {
            failures.push(format!("json.loads({text:?})\n  Python: {case}\n  Rust:   {got:?}"));
        }
    }

    for case in data["repr"].as_array().unwrap() {
        let value = loads(case["input"].as_str().unwrap()).expect("entrada de repr é JSON válido");
        if py_repr(&value) != case["repr"] || py_str(&value) != case["str"] {
            failures.push(format!(
                "repr/str de {}\n  Python: {} | {}\n  Rust:   {} | {}",
                case["input"],
                case["repr"],
                case["str"],
                py_repr(&value),
                py_str(&value)
            ));
        }
    }

    for case in data["utf8"].as_array().unwrap() {
        let got = decode_utf8(&hex(case["input"].as_str().unwrap()));
        let ok = match &got {
            Ok(text) => case["text"] == text.as_str(),
            Err(error) => case["error"] == error.message.as_str() && error.kind == "UnicodeDecodeError",
        };
        if !ok {
            failures.push(format!("decode {}\n  Python: {case}\n  Rust:   {got:?}", case["input"]));
        }
    }

    for case in data["os_error"].as_array().unwrap() {
        let errno = case["errno"].as_i64().unwrap() as i32;
        let got = os_error(&std::io::Error::from_raw_os_error(errno), case["path"].as_str().unwrap());
        if case["kind"] != got.kind || case["message"] != got.message.as_str() {
            failures.push(format!("OSError {errno}\n  Python: {case}\n  Rust:   {got:?}"));
        }
    }

    assert!(failures.is_empty(), "{} divergência(s):\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn isprintable_em_todos_os_code_points() {
    let data = fixture();
    let ranges: Vec<(u32, u32)> = data["nonprintable"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| (pair[0].as_u64().unwrap() as u32, pair[1].as_u64().unwrap() as u32))
        .collect();
    // Intervalos ordenados: o último que começa até `code` decide.
    let python_printable = |code: u32| {
        let after = ranges.partition_point(|&(start, _)| start <= code);
        after == 0 || ranges[after - 1].1 < code
    };
    let diverged: Vec<String> = (0..=0x10FFFF_u32)
        .filter_map(char::from_u32)
        .filter(|&c| py_isprintable(c) != python_printable(u32::from(c)))
        .take(20)
        .map(|c| format!("U+{:04X}", u32::from(c)))
        .collect();
    assert!(diverged.is_empty(), "isprintable diverge em {diverged:?}");
}
