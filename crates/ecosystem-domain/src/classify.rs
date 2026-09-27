//! Classificação editorial — `classify()` de `scripts/update_profile.py`.
//!
//! Duas versões, com o nome gravado em `ecosystem.projects.classifier_version`:
//!
//! - **`py-classify@1`** ([`classify_py_v1`], o padrão) reproduz o Python,
//!   **inclusive o defeito A6** (auditoria de 2026-09-25): as palavras-chave
//!   são procuradas como *substring*, então `"ai"` casa com "d**ai**ly" e
//!   `"java"` com "**java**script".
//! - **`classifier@2`** ([`classify_v2`]) corrige A6: as mesmas listas, na
//!   mesma precedência, casadas como **palavras inteiras**. Tudo que não é
//!   letra nem dígito separa palavras (`g-mod` → "g mod"); no **nome** do
//!   repositório, também `camelCase` e a fronteira letra/dígito
//!   (`DailyPlanner` → "daily planner", `chips8` → "chips 8"). Na descrição
//!   não: ela é prosa, e "JavaScript" viraria "java script" (Academia).
//!   Muda a saída pública, então só entra por escolha
//!   (`--classifier classifier@2`) e com o diff revisado (D-006, D-039).

use crate::repo::RepoFacts;
use crate::text::truthy;

/// Identificador desta versão da heurística; vai para
/// `ecosystem.projects.classifier_version`.
pub const PY_V1: &str = "py-classify@1";

/// Listas em ordem de precedência: a primeira que casa decide o rótulo.
const RULES: [(&[&str], &str); 6] = [
    (&["veritas", "digital logic", "chips", "umbra lima"], "Digital Logic / Hardware"),
    (&["baluarte", "llbr", "vanguard"], "Ecossistema Baluarte"),
    (&["academic", "atividade", "decision", "flowgorithm", "java", "python", "pseudocode", "teste aula"], "Academia"),
    (&["game", "games", "g-mod", "black mesa", "fallout", "mod-pack", "catacombs", "ossuary", "recycle"], "Games"),
    (&["ai", "artificial", "jarvis", "claude", "kizeo"], "IA & Automação"),
    (&["sujok", "banco de dados", "backend", "local de trabalho", "backup"], "Infraestrutura / Backend / Dados"),
];

const WEB_TOKENS: [&str; 5] = ["portfolio", "site", "furniture", "construction", "invitation"];

/// Identificador da versão que casa palavras inteiras (corrige A6).
pub const V2: &str = "classifier@2";

/// A versão da heurística.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Classifier {
    /// `py-classify@1`: o Python, com A6.
    #[default]
    PyV1,
    /// `classifier@2`: palavras inteiras.
    V2,
}

impl Classifier {
    /// Nome gravado em `classifier_version`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PyV1 => PY_V1,
            Self::V2 => V2,
        }
    }

    /// Lê o nome de uma versão.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            PY_V1 => Some(Self::PyV1),
            V2 => Some(Self::V2),
            _ => None,
        }
    }

    /// O rótulo nesta versão.
    pub fn classify(self, repo: &RepoFacts) -> &'static str {
        match self {
            Self::PyV1 => classify_py_v1(repo),
            Self::V2 => classify_v2(repo),
        }
    }
}

/// A regra comum às duas versões; só muda o que é "conter a palavra-chave".
fn classify_with(repo: &RepoFacts, has: impl Fn(&[&str]) -> bool) -> &'static str {
    for (tokens, label) in RULES {
        if has(tokens) {
            return label;
        }
    }
    if truthy(repo.homepage.as_deref()) || has(&WEB_TOKENS) {
        return "Web";
    }
    if repo.fork {
        return "Experimentos";
    }
    if !truthy(repo.description.as_deref()) && repo.name.is_empty() {
        return "Experimentos";
    }
    "Software & Ferramentas"
}

/// Rótulo editorial (em português) de um repositório — versão `py-classify@1`.
pub fn classify_py_v1(repo: &RepoFacts) -> &'static str {
    let text = format!("{} {}", repo.name, repo.description.as_deref().unwrap_or("")).to_lowercase();
    classify_with(repo, |tokens| tokens.iter().any(|token| text.contains(token)))
}

/// Palavras em minúsculas, separadas por um espaço: quebra em tudo que não é
/// letra nem dígito e, com `split_case`, nas fronteiras de `camelCase`
/// (`dailyPlanner`, `HTMLParser` → "html parser") e entre letra e dígito
/// (`chips8`).
pub fn words(text: &str, split_case: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 8);
    for (index, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            out.push(' ');
            continue;
        }
        if let Some(&previous) = index.checked_sub(1).and_then(|i| chars.get(i)).filter(|_| split_case) {
            let next = chars.get(index + 1).copied();
            let camel = previous.is_lowercase() && c.is_uppercase();
            let acronym_end = previous.is_uppercase() && c.is_uppercase() && next.is_some_and(char::is_lowercase);
            let digit_edge = previous.is_alphanumeric() && previous.is_numeric() != c.is_numeric();
            if camel || acronym_end || digit_edge {
                out.push(' ');
            }
        }
        out.extend(c.to_lowercase());
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Rótulo editorial — versão `classifier@2`: as palavras-chave casam como
/// palavras inteiras (ou sequência de palavras) do nome e da descrição.
pub fn classify_v2(repo: &RepoFacts) -> &'static str {
    let text = format!(" {} {} ", words(&repo.name, true), words(repo.description.as_deref().unwrap_or(""), false));
    classify_with(repo, |tokens| tokens.iter().any(|token| text.contains(&format!(" {} ", words(token, false)))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(name: &str, description: &str) -> RepoFacts {
        RepoFacts { name: name.into(), description: Some(description.into()), ..RepoFacts::default() }
    }

    #[test]
    fn defeito_a6_reproduzido_de_proposito() {
        assert_eq!("IA & Automação", classify_py_v1(&repo("DailyPlanner", "Agenda")));
        assert_eq!("Academia", classify_py_v1(&repo("x", "site em JavaScript")));
        assert_eq!("IA & Automação", classify_py_v1(&repo("plain", "")), "p-l-AI-n");
    }

    #[test]
    fn classifier_v2_casa_palavras_inteiras() {
        // Os exemplos do achado A6.
        assert_eq!("Software & Ferramentas", classify_v2(&repo("DailyPlanner", "Agenda diária em TypeScript/Vite")));
        assert_eq!("Software & Ferramentas", classify_v2(&repo("Mail-Helper", "Email automation")));
        assert_eq!("Web", classify_v2(&repo("Portfolio", "Personal site built with JavaScript")));
        assert_eq!("Software & Ferramentas", classify_v2(&repo("x", "App em JavaScript")), "não é Academia");
        assert_eq!("Ecossistema Baluarte", classify_v2(&repo("Project-Baluarte-DevFlow", "")));
        // O que era palavra continua casando.
        assert_eq!("IA & Automação", classify_v2(&repo("jarvis-core", "Assistente de AI local")));
        assert_eq!("IA & Automação", classify_v2(&repo("JarvisCore", "")), "camelCase separa");
        assert_eq!("Academia", classify_v2(&repo("curso", "Exercícios de Java")));
        assert_eq!("Games", classify_v2(&repo("gmod-addons", "Addons para G-Mod")), "hífen vira espaço dos dois lados");
        assert_eq!("Digital Logic / Hardware", classify_v2(&repo("sim", "Simulador de digital logic")));
        assert_eq!("Infraestrutura / Backend / Dados", classify_v2(&repo("x", "Modelagem de banco de dados")));
        assert_eq!("Digital Logic / Hardware", classify_v2(&repo("chips8", "")), "letra e dígito separam");
    }

    #[test]
    fn palavras() {
        assert_eq!("daily planner", words("DailyPlanner", true));
        assert_eq!("html parser 2 x", words("HTMLParser2-x", true));
        assert_eq!("g mod mod pack", words("g-mod mod_pack", true));
        assert_eq!("ação útil", words("  Ação/Útil  ", true));
        assert_eq!("javascript e typescript", words("JavaScript e TypeScript", false), "prosa não quebra marca");
    }

    #[test]
    fn nomes_das_versoes() {
        assert_eq!(Some(Classifier::V2), Classifier::parse("classifier@2"));
        assert_eq!(Some(Classifier::PyV1), Classifier::parse("py-classify@1"));
        assert_eq!(None, Classifier::parse("classifier@3"));
        assert_eq!(Classifier::PyV1, Classifier::default(), "o padrão é o Python");
    }

    #[test]
    fn precedencia() {
        assert_eq!("Digital Logic / Hardware", classify_py_v1(&repo("veritas-game", "")));
        assert_eq!("Software & Ferramentas", classify_py_v1(&repo("tool", "")));
    }
}
