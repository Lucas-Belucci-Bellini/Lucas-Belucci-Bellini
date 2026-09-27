//! `json.loads` e `int()` do Python.
//!
//! Os manifestos são editados à mão e o Python converte campos com `int(...)`
//! (`order`, `priority`, `http_status`, `ahead_by`). O que ele aceita e o que
//! ele recusa decide se a entrada vale, cai no padrão ou derruba a execução.
//!
//! [`loads`] é o decodificador do CPython 3.12 (`json/decoder.py` +
//! `Modules/_json.c`) portado: aceita o que ele aceita, recusa o que ele
//! recusa e, quando recusa, com a mesma mensagem — `Expecting ',' delimiter:
//! line 3 column 5 (char 17)`, posição em code points. Chave repetida fica
//! com o último valor na posição da primeira, como no `dict`. O que o Python
//! aceita e um [`Value`] não representa (`NaN`, `Infinity`, inteiro fora de 64
//! bits, float que estoura, surrogate isolado) volta como
//! [`JsonError::Unsupported`], nunca como um valor inventado.

use serde_json::{Map, Number, Value};

use crate::monitor::PyException;
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

/// Nome da exceção do `json.loads`, como o traceback a imprime.
pub const JSON_DECODE_ERROR: &str = "json.decoder.JSONDecodeError";

/// Por que o [`loads`] não devolveu um valor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonError {
    /// `json.JSONDecodeError`: o Python recusaria com esta mensagem.
    Decode {
        /// A mensagem sem a posição (`Expecting value`).
        message: &'static str,
        /// Posição, em code points.
        pos: usize,
        /// Linha, a partir de 1.
        line: usize,
        /// Coluna, a partir de 1.
        column: usize,
    },
    /// O Python aceitaria, mas o valor não cabe num [`Value`].
    Unsupported {
        /// O quê.
        what: &'static str,
        /// Posição, em code points.
        pos: usize,
    },
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decode { message, pos, line, column } => {
                write!(f, "{message}: line {line} column {column} (char {pos})")
            }
            Self::Unsupported { what, pos } => {
                write!(f, "{what} (char {pos}): o Python aceita, mas o valor não tem representação no Rust")
            }
        }
    }
}

impl std::error::Error for JsonError {}

impl JsonError {
    /// A exceção que o Python levantaria (ou `Unsupported`, que ele não levanta).
    pub fn exception(&self) -> PyException {
        match self {
            Self::Decode { .. } => PyException::new(JSON_DECODE_ERROR, self.to_string()),
            Self::Unsupported { .. } => PyException::new("Unsupported", self.to_string()),
        }
    }
}

/// Profundidade máxima de listas e objetos (o CPython pararia antes, com
/// `RecursionError`).
const MAX_DEPTH: usize = 900;

enum Fail {
    /// `StopIteration(pos)` do scanner: vira `Expecting value` lá em cima.
    Stop(usize),
    Json(JsonError),
}

struct Scanner<'a> {
    s: &'a [char],
    /// O primeiro valor que o Python aceitaria e o Rust não representa. Não
    /// interrompe a leitura: um erro de sintaxe mais adiante ainda é o que o
    /// Python relataria.
    unsupported: std::cell::Cell<Option<(&'static str, usize)>>,
}

fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

impl Scanner<'_> {
    fn error(&self, message: &'static str, pos: usize) -> Fail {
        let before = &self.s[..pos.min(self.s.len())];
        let line = before.iter().filter(|&&c| c == '\n').count() + 1;
        let column = match before.iter().rposition(|&c| c == '\n') {
            Some(newline) => pos - newline,
            None => pos + 1,
        };
        Fail::Json(JsonError::Decode { message, pos, line, column })
    }

    fn unsupported(what: &'static str, pos: usize) -> Fail {
        Fail::Json(JsonError::Unsupported { what, pos })
    }

    /// Anota o valor sem representação e segue com um marcador no lugar.
    fn defer(&self, what: &'static str, pos: usize) -> Value {
        if self.unsupported.get().is_none() {
            self.unsupported.set(Some((what, pos)));
        }
        Value::Null
    }

    fn skip_ws(&self, mut idx: usize) -> usize {
        while idx < self.s.len() && is_ws(self.s[idx]) {
            idx += 1;
        }
        idx
    }

    fn follows(&self, idx: usize, word: &str) -> bool {
        word.chars().enumerate().all(|(offset, c)| self.s.get(idx + offset) == Some(&c))
    }

    /// `scan_once_unicode`.
    fn scan_once(&self, idx: usize, depth: usize) -> Result<(Value, usize), Fail> {
        let len = self.s.len();
        if idx >= len {
            return Err(Fail::Stop(idx));
        }
        match self.s[idx] {
            '"' => {
                let (text, next) = self.scanstring(idx + 1)?;
                return Ok((Value::String(text), next));
            }
            '{' | '[' if depth >= MAX_DEPTH => return Err(Self::unsupported("aninhamento profundo demais", idx)),
            '{' => return self.parse_object(idx + 1, depth + 1),
            '[' => return self.parse_array(idx + 1, depth + 1),
            'n' if idx + 3 < len && self.follows(idx + 1, "ull") => return Ok((Value::Null, idx + 4)),
            't' if idx + 3 < len && self.follows(idx + 1, "rue") => return Ok((Value::Bool(true), idx + 4)),
            'f' if idx + 4 < len && self.follows(idx + 1, "alse") => return Ok((Value::Bool(false), idx + 5)),
            'N' if idx + 2 < len && self.follows(idx + 1, "aN") => return Ok((self.defer("NaN", idx), idx + 3)),
            'I' if idx + 7 < len && self.follows(idx + 1, "nfinity") => {
                return Ok((self.defer("Infinity", idx), idx + 8));
            }
            '-' if idx + 8 < len && self.follows(idx + 1, "Infinity") => {
                return Ok((self.defer("-Infinity", idx), idx + 9));
            }
            _ => {}
        }
        self.match_number(idx)
    }

    /// `_match_number_unicode`.
    fn match_number(&self, start: usize) -> Result<(Value, usize), Fail> {
        let s = self.s;
        let len = s.len();
        let digit = |idx: usize| s.get(idx).is_some_and(char::is_ascii_digit);
        let mut idx = start;
        if s[idx] == '-' {
            idx += 1;
            if idx >= len {
                return Err(Fail::Stop(start));
            }
        }
        if ('1'..='9').contains(&s[idx]) {
            idx += 1;
            while digit(idx) {
                idx += 1;
            }
        } else if s[idx] == '0' {
            idx += 1;
        } else {
            return Err(Fail::Stop(start));
        }
        let mut is_float = false;
        if idx + 1 < len && s[idx] == '.' && digit(idx + 1) {
            is_float = true;
            idx += 2;
            while digit(idx) {
                idx += 1;
            }
        }
        if idx + 1 < len && matches!(s[idx], 'e' | 'E') {
            let e_start = idx;
            idx += 1;
            if idx + 1 < len && matches!(s[idx], '-' | '+') {
                idx += 1;
            }
            while digit(idx) {
                idx += 1;
            }
            if s[idx - 1].is_ascii_digit() {
                is_float = true;
            } else {
                idx = e_start;
            }
        }
        let text: String = s[start..idx].iter().collect();
        let value = if is_float {
            let float: f64 = text.parse().expect("o scanner só aceita float bem formado");
            Number::from_f64(float).map_or_else(|| self.defer("float fora do intervalo do f64", start), Value::Number)
        } else if let Ok(int) = text.parse::<i64>() {
            Value::Number(Number::from(int))
        } else if let Ok(int) = text.parse::<u64>() {
            Value::Number(Number::from(int))
        } else {
            self.defer("inteiro fora de 64 bits", start)
        };
        Ok((value, idx))
    }

    fn hex4(&self, from: usize) -> Option<u32> {
        (from..from + 4).try_fold(0u32, |acc, idx| Some(acc * 16 + self.s[idx].to_digit(16)?))
    }

    /// `scanstring_unicode` (modo estrito): `end` é o índice depois da aspa.
    fn scanstring(&self, mut end: usize) -> Result<(String, usize), Fail> {
        let s = self.s;
        let len = s.len();
        let begin = end - 1;
        let mut out = String::new();
        loop {
            let mut next = end;
            while next < len && s[next] != '"' && s[next] != '\\' {
                if u32::from(s[next]) <= 0x1f {
                    return Err(self.error("Invalid control character at", next));
                }
                next += 1;
            }
            if next >= len {
                return Err(self.error("Unterminated string starting at", begin));
            }
            out.extend(&s[end..next]);
            if s[next] == '"' {
                return Ok((out, next + 1));
            }
            next += 1;
            if next == len {
                return Err(self.error("Unterminated string starting at", begin));
            }
            if s[next] != 'u' {
                end = next + 1;
                out.push(match s[next] {
                    '"' => '"',
                    '\\' => '\\',
                    '/' => '/',
                    'b' => '\u{8}',
                    'f' => '\u{c}',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    _ => return Err(self.error("Invalid \\escape", end - 2)),
                });
                continue;
            }
            next += 1;
            end = next + 4;
            if end >= len {
                return Err(self.error("Invalid \\uXXXX escape", next - 1));
            }
            let mut code = self.hex4(next).ok_or_else(|| self.error("Invalid \\uXXXX escape", end - 5))?;
            if (0xD800..=0xDBFF).contains(&code) && end + 6 < len && s[end] == '\\' && s[end + 1] == 'u' {
                let low = self.hex4(end + 2).ok_or_else(|| self.error("Invalid \\uXXXX escape", end + 1))?;
                if (0xDC00..=0xDFFF).contains(&low) {
                    code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                    end += 6;
                }
            }
            match char::from_u32(code) {
                Some(c) => out.push(c),
                None => {
                    self.defer("surrogate isolado", end - 6);
                    out.push(char::REPLACEMENT_CHARACTER);
                }
            }
        }
    }

    /// `_parse_object_unicode`: `idx` é o índice depois do `{`.
    fn parse_object(&self, idx: usize, depth: usize) -> Result<(Value, usize), Fail> {
        let s = self.s;
        let len = s.len();
        let mut map = Map::new();
        let mut idx = self.skip_ws(idx);
        if idx >= len || s[idx] != '}' {
            loop {
                if idx >= len || s[idx] != '"' {
                    return Err(self.error("Expecting property name enclosed in double quotes", idx));
                }
                let (key, next) = self.scanstring(idx + 1)?;
                idx = self.skip_ws(next);
                if idx >= len || s[idx] != ':' {
                    return Err(self.error("Expecting ':' delimiter", idx));
                }
                idx = self.skip_ws(idx + 1);
                let (value, next) = self.scan_once(idx, depth)?;
                // IndexMap: a chave repetida troca o valor e fica na posição da primeira.
                map.insert(key, value);
                idx = self.skip_ws(next);
                if idx < len && s[idx] == '}' {
                    break;
                }
                if idx >= len || s[idx] != ',' {
                    return Err(self.error("Expecting ',' delimiter", idx));
                }
                idx = self.skip_ws(idx + 1);
            }
        }
        Ok((Value::Object(map), idx + 1))
    }

    /// `_parse_array_unicode`: `idx` é o índice depois do `[`.
    fn parse_array(&self, idx: usize, depth: usize) -> Result<(Value, usize), Fail> {
        let s = self.s;
        let len = s.len();
        let mut items = Vec::new();
        let mut idx = self.skip_ws(idx);
        if idx >= len || s[idx] != ']' {
            loop {
                let (value, next) = self.scan_once(idx, depth)?;
                items.push(value);
                idx = self.skip_ws(next);
                if idx < len && s[idx] == ']' {
                    break;
                }
                if idx >= len || s[idx] != ',' {
                    return Err(self.error("Expecting ',' delimiter", idx));
                }
                idx = self.skip_ws(idx + 1);
            }
        }
        Ok((Value::Array(items), idx + 1))
    }
}

/// `json.loads(text)` do CPython 3.12.
pub fn loads(text: &str) -> Result<Value, JsonError> {
    let chars: Vec<char> = text.chars().collect();
    let scanner = Scanner { s: &chars, unsupported: std::cell::Cell::new(None) };
    let result = (|| {
        if chars.first() == Some(&'\u{feff}') {
            return Err(scanner.error("Unexpected UTF-8 BOM (decode using utf-8-sig)", 0));
        }
        let start = scanner.skip_ws(0);
        let (value, end) = match scanner.scan_once(start, 0) {
            Err(Fail::Stop(pos)) => return Err(scanner.error("Expecting value", pos)),
            other => other?,
        };
        let end = scanner.skip_ws(end);
        if end != chars.len() {
            return Err(scanner.error("Extra data", end));
        }
        match scanner.unsupported.get() {
            Some((what, pos)) => Err(Scanner::unsupported(what, pos)),
            None => Ok(value),
        }
    })();
    result.map_err(|fail| match fail {
        Fail::Json(error) => error,
        Fail::Stop(pos) => match scanner.error("Expecting value", pos) {
            Fail::Json(error) => error,
            Fail::Stop(_) => unreachable!("error() sempre devolve Json"),
        },
    })
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

    #[test]
    fn loads_com_as_mensagens_do_cpython() {
        assert_eq!(json!({"a": 2, "b": 3}), loads(r#"{"a":1,"b":3,"a":2}"#).unwrap());
        let error = |text: &str| loads(text).unwrap_err().to_string();
        assert_eq!("Expecting value: line 1 column 1 (char 0)", error(""));
        assert_eq!("Expecting property name enclosed in double quotes: line 1 column 8 (char 7)", error(r#"{"a":1,}"#));
        assert_eq!("Expecting value: line 4 column 2 (char 7)", error("\n\n  [\n x]"));
        assert!(matches!(loads("NaN"), Err(JsonError::Unsupported { .. })));
    }
}
