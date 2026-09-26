//! Taxonomia do catálogo — `CATEGORIES` e `CATEGORY_ALIASES` de
//! `scripts/project_catalog.py`, e `DOMAINS` de `scripts/update_profile.py`.
//!
//! O banco guarda a mesma taxonomia como dado de referência (migration 0003);
//! um teste do crate `store` confere que as duas listas não divergem.

/// As 12 categorias canônicas, na ordem de `CATEGORIES`.
pub const CATEGORIES: [&str; 12] = [
    "AI & Intelligence",
    "Web & SaaS",
    "Games",
    "Infrastructure",
    "Automation",
    "Education",
    "Productivity",
    "Research",
    "Security",
    "Hardware & Simulation",
    "Software & Tools",
    "Experimental",
];

/// Rótulo exibido no README (saída de `classify`) → categoria canônica.
pub const CATEGORY_ALIASES: [(&str, &str); 9] = [
    ("Digital Logic / Hardware", "Hardware & Simulation"),
    ("Ecossistema Baluarte", "Web & SaaS"),
    ("Academia", "Education"),
    ("Games", "Games"),
    ("IA & Automação", "AI & Intelligence"),
    ("Infraestrutura / Backend / Dados", "Infrastructure"),
    ("Web", "Web & SaaS"),
    ("Experimentos", "Experimental"),
    ("Software & Ferramentas", "Software & Tools"),
];

/// Domínios da vitrine (bloco `WHAT-I-BUILD`): título e rótulos cobertos.
pub const DOMAINS: [(&str, &[&str]); 6] = [
    ("WEB & SAAS", &["Ecossistema Baluarte", "Web"]),
    ("AI & AUTOMATION", &["IA & Automação"]),
    ("HARDWARE & LOGIC", &["Digital Logic / Hardware"]),
    ("GAMES & WORLDS", &["Games"]),
    ("TOOLS & SYSTEMS", &["Software & Ferramentas", "Infraestrutura / Backend / Dados"]),
    ("ACADEMIC & LABS", &["Academia", "Experimentos"]),
];

/// Rótulo do README → categoria canônica; desconhecido cai em `Experimental`
/// em vez de virar categoria nova (a taxonomia só cresce por edição deliberada).
pub fn canonical_category(label: &str) -> &'static str {
    CATEGORY_ALIASES.iter().find(|(alias, _)| *alias == label).map_or("Experimental", |(_, category)| category)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_rotulo_tem_categoria_e_dominio() {
        for (label, category) in CATEGORY_ALIASES {
            assert!(CATEGORIES.contains(&category), "{label} → {category}");
            assert!(
                DOMAINS.iter().any(|(_, labels)| labels.contains(&label)),
                "{label} não cai em nenhum domínio da vitrine"
            );
        }
    }
}
