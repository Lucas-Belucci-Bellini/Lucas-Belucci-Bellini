//! As operações de texto do Python que o README usa, com a mesma saída.
//!
//! Onde a regra conta caracteres (`len`, fatia `[:150]`, largura dos pontos
//! no mapa), o Python conta **code points**, não bytes: `chars()` aqui.

use ecosystem_domain::text::py_is_space;
use serde_json::Value;

/// `urllib.parse.quote(text, safe="")`: só letras e dígitos ASCII e `_.-~`
/// passam; o resto vira `%XX` (maiúsculo) sobre os bytes UTF-8.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `html.escape(text, quote=True)`.
pub fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            other => out.push(other),
        }
    }
    out
}

/// `str.rstrip()` sem argumento.
pub fn py_rstrip(text: &str) -> &str {
    text.trim_end_matches(py_is_space)
}

/// `len(text)` do Python.
pub fn py_len(text: &str) -> usize {
    text.chars().count()
}

/// `text[:limit]` do Python.
pub fn py_prefix(text: &str, limit: usize) -> &str {
    match text.char_indices().nth(limit) {
        Some((index, _)) => &text[..index],
        None => text,
    }
}

/// `text.replace("|", "\\|")` — o que quebraria uma célula de tabela.
pub fn md_cell(text: &str) -> String {
    text.replace('|', "\\|")
}

/// `json.dumps(value, ensure_ascii=False[, sort_keys=True])`: separadores
/// `", "` e `": "`, texto sem escapar o que não é ASCII.
pub fn py_dumps(value: &Value, sort_keys: bool) -> String {
    match value {
        Value::Object(map) => {
            let mut fields: Vec<(&String, &Value)> = map.iter().collect();
            if sort_keys {
                fields.sort_by(|a, b| a.0.cmp(b.0));
            }
            let fields: Vec<String> = fields
                .into_iter()
                .map(|(key, value)| format!("{}: {}", Value::String(key.clone()), py_dumps(value, sort_keys)))
                .collect();
            format!("{{{}}}", fields.join(", "))
        }
        Value::Array(items) => {
            format!("[{}]", items.iter().map(|item| py_dumps(item, sort_keys)).collect::<Vec<_>>().join(", "))
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dumps_como_o_python() {
        let value = serde_json::json!({"b": ["x", "ç\"\n"], "a": 1, "c": true, "d": []});
        assert_eq!(r#"{"b": ["x", "ç\"\n"], "a": 1, "c": true, "d": []}"#, py_dumps(&value, false));
        assert_eq!(r#"{"a": 1, "b": ["x", "ç\"\n"], "c": true, "d": []}"#, py_dumps(&value, true));
    }

    #[test]
    fn quote_sem_nada_seguro() {
        assert_eq!("C%23", quote("C#"));
        assert_eq!("PL%2FpgSQL", quote("PL/pgSQL"));
        assert_eq!("%F0%9F%8C%90%20DEMO-X_Y.Z~", quote("🌐 DEMO-X_Y.Z~"));
        assert_eq!("12%20repos", quote("12 repos"));
    }

    #[test]
    fn escape_de_html() {
        assert_eq!("a&quot;b&#x27;&lt;&gt;&amp;", html_escape("a\"b'<>&"));
    }

    #[test]
    fn fatia_por_code_point() {
        assert_eq!("çã", py_prefix("çãõ", 2));
        assert_eq!("ab", py_prefix("ab", 5));
        assert_eq!(3, py_len("çãõ"));
    }
}
