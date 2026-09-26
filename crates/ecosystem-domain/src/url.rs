//! Validação de URL de site — `_looks_like_http_url()` de `scripts/project_catalog.py`.
//!
//! A expressão do Python é `^https?://[^\s/$.?#].[^\s]*$` com `re.match`.
//! Duas diferenças de semântica precisam ser reproduzidas à mão:
//!
//! * o `\s` do `re` do Python inclui U+001C–U+001F; o do crate `regex`, não
//!   (ver [`crate::text`]) — a classe é escrita com eles explicitamente;
//! * sem `re.MULTILINE`, o `$` do Python casa no fim **ou antes de um `\n`
//!   final**: `"https://example.org\n"` é válida no Python. Aqui isso vira
//!   `\n?\z`.
//!
//! A mesma expressão está na `CHECK` de `ecosystem.websites.url`
//! (`db/migrations/0004_websites.up.sql`).

use regex::Regex;
use std::sync::LazyLock;

static HTTP_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^https?://[^\s\x1C-\x1F/$.?#].[^\s\x1C-\x1F]*\n?\z").expect("expressão válida"));

/// `True` quando o texto parece uma URL http(s) — mesma resposta do Python.
pub fn looks_like_http_url(value: &str) -> bool {
    !value.is_empty() && HTTP_URL.is_match(value)
}

#[cfg(test)]
mod tests {
    use super::looks_like_http_url;

    #[test]
    fn casos_basicos() {
        assert!(looks_like_http_url("https://example.org"));
        assert!(!looks_like_http_url("ftp://example.org"));
        assert!(!looks_like_http_url(""));
    }

    #[test]
    fn quebra_de_linha_final_como_o_dolar_do_python() {
        assert!(looks_like_http_url("https://example.org\n"));
        assert!(!looks_like_http_url("https://example.org\n\n"));
    }
}
