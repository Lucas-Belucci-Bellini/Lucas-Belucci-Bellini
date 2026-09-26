//! Linguagens: a matriz pública (`language_rows`), o peso legível
//! (`format_bytes`) e a pilha de cada projeto (`stack_for`).

use std::collections::HashMap;

use ecosystem_domain::repo::RepoFacts;

/// Linguagens de um repositório, na ordem em que a API (ou o arquivo) as dá.
pub type LanguageMap = HashMap<String, Vec<(String, i64)>>;

/// `LANGUAGE_DISPLAY`: nome exibido quando difere do nome do GitHub.
pub const LANGUAGE_DISPLAY: [(&str, &str); 3] =
    [("Batchfile", "Batch"), ("Dockerfile", "Dockerfile"), ("PLpgSQL", "PL/pgSQL")];

/// `LANGUAGE_COLORS`: cor do badge.
pub const LANGUAGE_COLORS: [(&str, &str); 19] = [
    ("JavaScript", "F7DF1E"),
    ("TypeScript", "3178C6"),
    ("HTML", "E34F26"),
    ("CSS", "1572B6"),
    ("Java", "ED8B00"),
    ("Python", "3776AB"),
    ("CSharp", "239120"),
    ("C#", "239120"),
    ("PLpgSQL", "336791"),
    ("PL/pgSQL", "336791"),
    ("Rust", "DEA584"),
    ("Shell", "4EAA25"),
    ("GDScript", "478CBF"),
    ("PowerShell", "5391FE"),
    ("Portugol", "6A5ACD"),
    ("Batchfile", "5C2D91"),
    ("Batch", "5C2D91"),
    ("ShaderLab", "A48EFA"),
    ("Dockerfile", "2496ED"),
];

/// Cor sem entrada na tabela.
pub const DEFAULT_COLOR: &str = "6E6E6E";

fn lookup(table: &'static [(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    table.iter().find(|(name, _)| *name == key).map(|(_, value)| *value)
}

/// `LANGUAGE_DISPLAY.get(language, language)`.
pub fn display(language: &str) -> &str {
    lookup(&LANGUAGE_DISPLAY, language).unwrap_or(language)
}

/// `LANGUAGE_COLORS.get(language, LANGUAGE_COLORS.get(display, "6E6E6E"))`.
pub fn color(language: &str, display: &str) -> &'static str {
    lookup(&LANGUAGE_COLORS, language).or_else(|| lookup(&LANGUAGE_COLORS, display)).unwrap_or(DEFAULT_COLOR)
}

/// Uma linha da matriz de linguagens.
#[derive(Debug, Clone, PartialEq)]
pub struct LanguageRow {
    /// Nome do GitHub.
    pub language: String,
    /// Nome exibido.
    pub display: String,
    /// Bytes somados.
    pub bytes: i64,
    /// Repositórios em que aparece.
    pub repositories: i64,
    /// Participação, em %.
    pub share: f64,
}

/// `language_rows(repos, languages, public_only=True)`.
///
/// Só os públicos; um repositório repetido no inventário conta duas vezes,
/// como no laço do Python. Ordem: bytes decrescentes, depois o nome sem caixa;
/// o empate total fica na ordem em que a linguagem apareceu.
pub fn language_rows(repos: &[RepoFacts], languages: &LanguageMap) -> Vec<LanguageRow> {
    let mut totals: Vec<(String, i64, i64)> = Vec::new();
    let mut index: HashMap<&str, usize> = HashMap::new();
    for repo in repos.iter().filter(|repo| !repo.private) {
        for (language, bytes) in languages.get(&repo.full_name).map_or(&[][..], Vec::as_slice) {
            let position = *index.entry(language.as_str()).or_insert_with(|| {
                totals.push((language.clone(), 0, 0));
                totals.len() - 1
            });
            totals[position].1 += bytes;
            totals[position].2 += 1;
        }
    }
    let total = match totals.iter().map(|(_, bytes, _)| bytes).sum::<i64>() {
        0 => 1,
        sum => sum,
    };
    totals.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
    totals
        .into_iter()
        .map(|(language, bytes, repositories)| LanguageRow {
            display: display(&language).to_string(),
            share: bytes as f64 / total as f64 * 100.0,
            language,
            bytes,
            repositories,
        })
        .collect()
}

/// `format_bytes(value)`.
pub fn format_bytes(value: i64) -> String {
    if value < 1024 {
        format!("{value} B")
    } else if value < 1024 * 1024 {
        format!("{:.1} KB", value as f64 / 1024.0)
    } else {
        format!("{:.2} MB", value as f64 / (1024.0 * 1024.0))
    }
}

/// `stack_for(repo, languages)`: até cinco linguagens, em crases.
pub fn stack_for(repo: &RepoFacts, languages: &LanguageMap) -> String {
    let items: Vec<&str> =
        languages.get(&repo.full_name).map_or(&[][..], Vec::as_slice).iter().map(|(name, _)| display(name)).collect();
    if items.is_empty() {
        return "—".into();
    }
    items.iter().take(5).map(|item| format!("`{item}`")).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peso_legivel() {
        assert_eq!("0 B", format_bytes(0));
        assert_eq!("1023 B", format_bytes(1023));
        assert_eq!("1.0 KB", format_bytes(1024));
        assert_eq!("1024.0 KB", format_bytes(1024 * 1024 - 1));
        assert_eq!("1.00 MB", format_bytes(1024 * 1024));
        assert_eq!("-5 B", format_bytes(-5));
    }

    #[test]
    fn cor_pelo_nome_do_github_e_depois_pelo_exibido() {
        assert_eq!("5C2D91", color("Batchfile", "Batch"));
        assert_eq!("336791", color("Desconhecida", "PL/pgSQL"));
        assert_eq!(DEFAULT_COLOR, color("Zig", "Zig"));
    }
}
