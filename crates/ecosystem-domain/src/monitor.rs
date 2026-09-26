//! Regras do monitor de commits — `.github/scripts/ecosystem_watch.py`.
//!
//! O monitor guarda, por repositório público, o último commit do branch
//! padrão; soma os commits novos (`compare`) num contador acumulado que parte
//! de [`LEGACY_BASELINE`]; e só publica um snapshot quando o estado muda.
//!
//! O estado grava **o que o Python fez com a resposta**, inclusive o texto das
//! exceções (`"list index out of range"` para uma mensagem de commit vazia,
//! `"'NoneType' object has no attribute 'get'"` para `commit: null`), e esse
//! texto entra na comparação entre varreduras. Por isso as funções daqui
//! devolvem o `str(exc)` que o Python gravaria, e o fixture
//! `tests/fixtures/parity/monitor.json` confere cada formato.

use serde_json::{Map, Value};

use crate::discovery::{json_truthy, py_str};
use crate::pyjson::py_int;

/// Contador herdado do monitor anterior (não é o GitHub Contributions).
pub const LEGACY_BASELINE: i64 = 1538;
/// `schema` do `ECOSYSTEM-COMMIT-STATE.json`.
pub const STATE_SCHEMA: i64 = 4;
/// Mensagem do Python quando a varredura não mudou nada.
pub const UNCHANGED: &str = "Nenhuma mudança semântica; snapshot não será reescrito.\n";

/// Uma exceção que o Python levantaria: o tipo e o `str(exc)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyException {
    /// Nome da classe (`KeyError`, `TypeError`…).
    pub kind: &'static str,
    /// `str(exc)`.
    pub message: String,
}

impl PyException {
    fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }
}

/// Nome do tipo Python de um valor JSON (`type(x).__name__`).
pub fn py_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "NoneType",
        Value::Bool(_) => "bool",
        Value::Number(n) if n.is_f64() => "float",
        Value::Number(_) => "int",
        Value::String(_) => "str",
        Value::Array(_) => "list",
        Value::Object(_) => "dict",
    }
}

/// `str(KeyError(key))`: a chave no `repr`.
fn key_error(key: &str) -> PyException {
    PyException::new("KeyError", format!("'{key}'"))
}

/// `value.get(key, default)` — só dicionários têm `.get`.
fn py_get<'a>(value: &'a Value, key: &str) -> Result<Option<&'a Value>, PyException> {
    match value {
        Value::Object(map) => Ok(map.get(key)),
        other => {
            Err(PyException::new("AttributeError", format!("'{}' object has no attribute 'get'", py_type_name(other))))
        }
    }
}

/// `value[key]` com chave texto.
fn py_index_str<'a>(value: &'a Value, key: &str) -> Result<&'a Value, PyException> {
    match value {
        Value::Object(map) => map.get(key).ok_or_else(|| key_error(key)),
        Value::Array(_) => Err(PyException::new("TypeError", "list indices must be integers or slices, not str")),
        Value::String(_) => Err(PyException::new("TypeError", "string indices must be integers, not 'str'")),
        other => Err(PyException::new("TypeError", format!("'{}' object is not subscriptable", py_type_name(other)))),
    }
}

/// `str.splitlines()` do Python: quebra em `\n`, `\r`, `\r\n`, `\v`, `\f`,
/// U+001C–U+001E, U+0085, U+2028 e U+2029.
pub fn py_splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        let breaks = matches!(
            c,
            '\n' | '\r' | '\u{0b}' | '\u{0c}' | '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        );
        if !breaks {
            continue;
        }
        lines.push(&text[start..index]);
        let mut end = index + c.len_utf8();
        if c == '\r' && chars.peek().is_some_and(|(_, next)| *next == '\n') {
            chars.next();
            end += 1;
        }
        start = end;
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// `text[:n]` — em code points.
pub fn py_prefix(text: &str, n: usize) -> &str {
    text.char_indices().nth(n).map_or(text, |(index, _)| &text[..index])
}

/// O corpo de `latest_commit()` depois da chamada à API: o dicionário
/// `{sha, date, message, url}` do primeiro commit, `None` quando a lista vem
/// vazia (repositório sem commits), ou a exceção que o Python levantaria.
pub fn latest_from_commits(commits: &Value) -> Result<Option<Map<String, Value>>, PyException> {
    if !json_truthy(commits) {
        return Ok(None);
    }
    let first = match commits {
        Value::Array(items) => &items[0],
        // `commits[0]` num dicionário: `KeyError(0)`, cujo `str` é `0`.
        Value::Object(_) => return Err(PyException::new("KeyError", "0")),
        Value::String(_) => {
            // `"abc"[0]` é `"a"`; o erro vem no `c["sha"]` seguinte.
            return Err(PyException::new("TypeError", "string indices must be integers, not 'str'"));
        }
        other => {
            return Err(PyException::new(
                "TypeError",
                format!("'{}' object is not subscriptable", py_type_name(other)),
            ));
        }
    };
    let sha = py_index_str(first, "sha")?.clone();
    let empty = Value::Object(Map::new());
    let date = py_get(py_get(first, "commit")?.unwrap_or(&empty), "committer")?
        .map_or(Ok(None), |committer| py_get(committer, "date"))?
        .cloned()
        .unwrap_or(Value::Null);
    let message = match py_get(py_get(first, "commit")?.unwrap_or(&empty), "message")? {
        None => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => {
            return Err(PyException::new(
                "AttributeError",
                format!("'{}' object has no attribute 'splitlines'", py_type_name(other)),
            ));
        }
    };
    let Some(line) = py_splitlines(&message).first().copied() else {
        return Err(PyException::new("IndexError", "list index out of range"));
    };
    let url = py_get(first, "html_url")?.cloned().unwrap_or(Value::Null);

    let mut latest = Map::new();
    latest.insert("sha".into(), sha);
    latest.insert("date".into(), date);
    latest.insert("message".into(), Value::String(py_prefix(line, 140).to_string()));
    latest.insert("url".into(), url);
    Ok(Some(latest))
}

/// `int(data.get("ahead_by", 0))` dentro do `try` de `compare_count()`:
/// qualquer falha é `None` ("quantidade não determinada").
pub fn ahead_by(data: &Value) -> Option<i64> {
    let Value::Object(map) = data else { return None };
    match map.get("ahead_by") {
        None => Some(0),
        Some(value) => py_int(value).ok(),
    }
}

/// `name.casefold() == perfil.casefold()`. Nomes de repositório do GitHub são
/// ASCII; fora dele, o `casefold` do Python e o `to_lowercase` do Rust diferem
/// em casos como `ß`.
pub fn is_profile_repository(name: &str, profile: &str) -> bool {
    name.to_lowercase() == profile.to_lowercase()
}

/// O `str()` de um SHA vindo do estado ou da API (o Python formata com f-string).
pub fn sha_text(value: &Value) -> String {
    py_str(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn splitlines_do_python() {
        assert_eq!(vec!["a", "b"], py_splitlines("a\r\nb"));
        assert_eq!(vec!["a", "b"], py_splitlines("a\u{85}b"));
        assert_eq!(vec!["", "x"], py_splitlines("\nx"));
        assert_eq!(vec!["a", ""], py_splitlines("a\n\n"));
        assert!(py_splitlines("").is_empty());
        assert_eq!(vec!["a\u{1f}b"], py_splitlines("a\u{1f}b"), "U+001F não quebra linha");
    }

    #[test]
    fn prefixo_em_code_points() {
        assert_eq!("éé", py_prefix("ééé", 2));
        assert_eq!("ab", py_prefix("ab", 140));
    }

    #[test]
    fn formatos_de_commit() {
        assert_eq!(Ok(None), latest_from_commits(&json!([])));
        assert_eq!(Ok(None), latest_from_commits(&json!({})));
        assert_eq!("0", latest_from_commits(&json!({"x": 1})).unwrap_err().message);
        let error = latest_from_commits(&json!([{"sha": "s", "commit": {"message": ""}}])).unwrap_err();
        assert_eq!(("IndexError", "list index out of range"), (error.kind, error.message.as_str()));
        let latest = latest_from_commits(&json!([{"sha": 5, "commit": {"message": "a\nb"}}])).unwrap().unwrap();
        assert_eq!(json!({"sha": 5, "date": null, "message": "a", "url": null}), Value::Object(latest));
    }

    #[test]
    fn ahead_by_como_o_int_do_python() {
        assert_eq!(Some(3), ahead_by(&json!({"ahead_by": 3})));
        assert_eq!(Some(0), ahead_by(&json!({"status": "diverged"})));
        assert_eq!(Some(7), ahead_by(&json!({"ahead_by": " 7 "})));
        assert_eq!(Some(2), ahead_by(&json!({"ahead_by": 2.9})));
        assert_eq!(None, ahead_by(&json!({"ahead_by": null})));
        assert_eq!(None, ahead_by(&json!([1])));
    }
}
