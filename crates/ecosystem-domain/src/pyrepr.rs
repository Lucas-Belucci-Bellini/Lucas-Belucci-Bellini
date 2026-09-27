//! `repr()` e `str()` do Python para o que o `json.loads` devolve.
//!
//! Os validadores escrevem valores lidos de JSON nas mensagens —
//! `f"primary_cta é {primary!r}"`, `f"{name}: …"` — e o texto tem de ser o do
//! Python: aspas escolhidas pelo conteúdo, escapes decididos pelo
//! `str.isprintable()` do Unicode do próprio CPython ([`crate::printable`],
//! gerado dele), float no formato curto (`1e+16`, `1e-05`, `100.0`), lista e
//! dicionário com `", "` e `": "`. A prova é `tests/pytext_parity.rs`.

use serde_json::{Number, Value};

use crate::printable::NONPRINTABLE;

/// `str.isprintable()` do Python para um caractere.
pub fn py_isprintable(c: char) -> bool {
    let code = u32::from(c);
    NONPRINTABLE
        .binary_search_by(|&(start, end)| {
            if end < code {
                std::cmp::Ordering::Less
            } else if start > code {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_err()
}

/// `repr(text)`: aspas simples, a menos que o texto tenha `'` e não tenha `"`.
pub fn py_repr_str(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') { '"' } else { '\'' };
    let mut out = String::with_capacity(text.len() + 2);
    out.push(quote);
    for c in text.chars() {
        match c {
            _ if c == quote || c == '\\' => {
                out.push('\\');
                out.push(c);
            }
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ if u32::from(c) < 0x20 || c == '\u{7f}' => out.push_str(&format!("\\x{:02x}", u32::from(c))),
            _ if c.is_ascii() || py_isprintable(c) => out.push(c),
            _ if u32::from(c) <= 0xff => out.push_str(&format!("\\x{:02x}", u32::from(c))),
            _ if u32::from(c) <= 0xffff => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            _ => out.push_str(&format!("\\U{:08x}", u32::from(c))),
        }
    }
    out.push(quote);
    out
}

/// `repr(float)`: os dígitos mais curtos que voltam ao mesmo valor; notação
/// científica quando o expoente decimal sai de `-4 < decpt <= 16`.
pub fn py_float_repr(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() { "-0.0".into() } else { "0.0".into() };
    }
    if !value.is_finite() {
        // `json.loads` nunca entrega estes num Value; ficam por completude.
        return if value.is_nan() {
            "nan".into()
        } else if value > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        };
    }
    // "{:e}" é a representação mais curta que volta ao mesmo f64: "1.2345e17".
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').expect("formato {:e}");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exponent: i32 = exponent.parse().expect("expoente do {:e}");
    let decpt = exponent + 1;
    let sign = if value < 0.0 { "-" } else { "" };
    if decpt <= -4 || decpt > 16 {
        let (first, rest) = digits.split_at(1);
        let fraction = if rest.is_empty() { String::new() } else { format!(".{rest}") };
        let magnitude = (decpt - 1).abs();
        let exp_sign = if decpt - 1 < 0 { '-' } else { '+' };
        return format!("{sign}{first}{fraction}e{exp_sign}{magnitude:02}");
    }
    let body = if decpt <= 0 {
        format!("0.{}{digits}", "0".repeat(decpt.unsigned_abs() as usize))
    } else if decpt as usize >= digits.len() {
        format!("{digits}{}.0", "0".repeat(decpt as usize - digits.len()))
    } else {
        let (whole, fraction) = digits.split_at(decpt as usize);
        format!("{whole}.{fraction}")
    };
    format!("{sign}{body}")
}

fn number_repr(number: &Number) -> String {
    match number.as_f64() {
        Some(float) if number.is_f64() => py_float_repr(float),
        _ => number.to_string(),
    }
}

/// `repr(value)` para um valor do `json.loads`.
pub fn py_repr(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Number(number) => number_repr(number),
        Value::String(text) => py_repr_str(text),
        Value::Array(items) => format!("[{}]", items.iter().map(py_repr).collect::<Vec<_>>().join(", ")),
        Value::Object(fields) => format!(
            "{{{}}}",
            fields
                .iter()
                .map(|(key, value)| format!("{}: {}", py_repr_str(key), py_repr(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// `str(value)`: o próprio texto para `str`; o `repr` para o resto.
pub fn py_str(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => py_repr(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn float_no_formato_curto() {
        assert_eq!("1e+16", py_float_repr(1e16));
        assert_eq!("1000000000000000.0", py_float_repr(1e15));
        assert_eq!("1e-05", py_float_repr(1e-5));
        assert_eq!("0.0001", py_float_repr(0.0001));
        assert_eq!("-0.0", py_float_repr(-0.0));
        assert_eq!("1.2345678901234568e+17", py_float_repr(123_456_789_012_345_680.0));
        assert_eq!("5e-324", py_float_repr(5e-324));
    }

    #[test]
    fn aspas_e_escapes() {
        assert_eq!("\"it's\"", py_repr_str("it's"));
        assert_eq!("'a\\'b\"c'", py_repr_str("a'b\"c"));
        assert_eq!(
            "'\\x7f\\x85\\xa0\\u200b😀\\ue000\\u0378'",
            py_repr_str("\u{7f}\u{85}\u{a0}\u{200b}😀\u{e000}\u{378}")
        );
        assert_eq!("[1, 'a', None, True, {'k': 1.0}]", py_repr(&json!([1, "a", null, true, {"k": 1.0}])));
        assert_eq!("texto", py_str(&json!("texto")));
    }
}
