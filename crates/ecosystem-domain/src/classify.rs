//! Classificação editorial — `classify()` de `scripts/update_profile.py`.
//!
//! **Reproduz o defeito conhecido A6** (auditoria de 2026-09-25): as
//! palavras-chave são procuradas como *substring*, então `"ai"` casa com
//! "d**ai**ly" e `"java"` com "**java**script". A correção é uma decisão
//! editorial e vai entrar como outra versão, com o diff revisado (D-006).

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

/// Rótulo editorial (em português) de um repositório — versão `py-classify@1`.
pub fn classify_py_v1(repo: &RepoFacts) -> &'static str {
    let text = format!("{} {}", repo.name, repo.description.as_deref().unwrap_or("")).to_lowercase();
    let has = |tokens: &[&str]| tokens.iter().any(|token| text.contains(token));
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
    fn precedencia() {
        assert_eq!("Digital Logic / Hardware", classify_py_v1(&repo("veritas-game", "")));
        assert_eq!("Software & Ferramentas", classify_py_v1(&repo("tool", "")));
    }
}
