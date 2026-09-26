//! `int()` do Python sobre o que o `json.loads` devolve.
//!
//! Os manifestos são editados à mão e o Python converte campos com `int(...)`
//! (`order`, `priority`, `http_status`, `ahead_by`). O que ele aceita e o que
//! ele recusa decide se a entrada vale, cai no padrão ou derruba a execução.

use serde_json::Value;

use crate::text::py_strip;

/// Por que o `int()` falharia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyIntError {
    /// `ValueError`: texto que não é número inteiro.
    Value,
    /// `TypeError`: `None`, lista ou objeto.
    Type,
}

impl PyIntError {
    /// Nome da exceção do Python.
    pub fn python_name(self) -> &'static str {
        match self {
            Self::Value => "ValueError",
            Self::Type => "TypeError",
        }
    }
}

/// `int(value)` para um valor JSON: booleano vira 0/1, número com fração é
/// truncado em direção a zero, texto segue [`py_int_str`].
pub fn py_int(value: &Value) -> Result<i64, PyIntError> {
    match value {
        Value::Bool(flag) => Ok(i64::from(*flag)),
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_u64().map(|n| i64::try_from(n).unwrap_or(i64::MAX)))
            .or_else(|| number.as_f64().map(|f| f.trunc() as i64))
            .ok_or(PyIntError::Value),
        Value::String(text) => py_int_str(text).ok_or(PyIntError::Value),
        Value::Null | Value::Array(_) | Value::Object(_) => Err(PyIntError::Type),
    }
}

/// `int(text)` do Python: espaços nas pontas, sinal e `_` entre dígitos.
/// Dígitos fora do ASCII (que o Python também aceita) ficam de fora.
pub fn py_int_str(text: &str) -> Option<i64> {
    let trimmed = py_strip(text);
    let (negative, digits) = match trimmed.as_bytes().first() {
        Some(b'-') => (true, &trimmed[1..]),
        Some(b'+') => (false, &trimmed[1..]),
        _ => (false, trimmed),
    };
    if digits.is_empty() || digits.starts_with('_') || digits.ends_with('_') || digits.contains("__") {
        return None;
    }
    if !digits.chars().all(|c| c.is_ascii_digit() || c == '_') {
        return None;
    }
    let value: i64 = digits.replace('_', "").parse().ok()?;
    Some(if negative { -value } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn int_do_python() {
        assert_eq!(Ok(3), py_int(&json!(3)));
        assert_eq!(Ok(3), py_int(&json!(3.9)));
        assert_eq!(Ok(-3), py_int(&json!(-3.9)));
        assert_eq!(Ok(1), py_int(&json!(true)));
        assert_eq!(Ok(12), py_int(&json!(" 1_2 ")));
        assert_eq!(Ok(-7), py_int(&json!("-7")));
        assert_eq!(Err(PyIntError::Value), py_int(&json!("3.0")));
        assert_eq!(Err(PyIntError::Value), py_int(&json!("")));
        assert_eq!(Err(PyIntError::Type), py_int(&json!(null)));
        assert_eq!(Err(PyIntError::Type), py_int(&json!([1])));
        assert_eq!(Some(443), py_int_str("+443"));
        assert_eq!(None, py_int_str("4__43"));
    }
}
