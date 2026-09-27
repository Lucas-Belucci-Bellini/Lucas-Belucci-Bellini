//! Semântica de texto do Python de que as regras dependem.
//!
//! `str.isspace()` do Python considera espaço a propriedade Unicode
//! `White_Space` **e também** U+001C–U+001F (separadores de arquivo, grupo,
//! registro e unidade, de classe bidirecional B/S). O `char::is_whitespace`
//! do Rust não inclui esses quatro. Usar `trim()`/`split_whitespace()` direto
//! mudaria, por exemplo, se uma descrição tem 25 caracteres "de verdade".
//! O teste de paridade confere [`py_is_space`] contra a lista do Python em
//! todos os code points.

/// `str.isspace()` do Python para um caractere.
pub fn py_is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `str.strip()` do Python (sem argumento).
pub fn py_strip(value: &str) -> &str {
    value.trim_matches(py_is_space)
}

/// `" ".join(value.split())` do Python: colapsa qualquer sequência de
/// espaços num espaço só e remove as pontas.
pub fn py_split_join(value: &str) -> String {
    value.split(py_is_space).filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" ")
}

/// Verdade do Python para um texto opcional: `None` e `""` são falsos;
/// `"   "` é verdadeiro.
pub fn truthy(value: Option<&str>) -> bool {
    value.is_some_and(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separadores_de_controle_sao_espaco_como_no_python() {
        assert!(py_is_space('\u{1f}'));
        assert!(!'\u{1f}'.is_whitespace(), "a diferença que este módulo existe para cobrir");
        assert_eq!("x", py_strip("\u{1c} x \u{1f}"));
        assert_eq!("a b c", py_split_join("a\u{1d}b  \u{1e}c"));
    }

    #[test]
    fn verdade_de_texto() {
        assert!(!truthy(None));
        assert!(!truthy(Some("")));
        assert!(truthy(Some("   ")));
    }
}
