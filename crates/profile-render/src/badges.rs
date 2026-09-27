//! Badges e chamadas para ação — a parte "render" de `scripts/project_catalog.py`.
//!
//! A regra do perfil fica visível aqui: com site verificado, o site vem
//! primeiro (em ouro) e o código depois (apagado); sem site, só o código.

use ecosystem_domain::presentation::Presentation;

use crate::pytext::quote;

/// Base dos badges do shields.io.
pub const BADGE: &str = "https://img.shields.io/badge/";
/// Fundo.
pub const COLOR_BACKGROUND: &str = "0e0c16";
/// Ouro: a ação principal.
pub const COLOR_GOLD: &str = "d4a24e";
/// Tom apagado: a ação secundária.
pub const COLOR_MUTED: &str = "4b3a5c";
/// Verde: site verificado.
pub const COLOR_GREEN: &str = "3ddc84";

/// `_badge(text, color, style=...)`: badge de uma parte, com o texto escapado.
pub fn badge(text: &str, color: &str, style: &str) -> String {
    format!("{BADGE}{}-{color}?style={style}&labelColor={COLOR_BACKGROUND}", quote(text))
}

fn live_website(presentation: &Presentation) -> Option<&str> {
    presentation.website.as_deref().filter(|website| !website.is_empty())
}

/// `cta_buttons(presentation)`: os dois botões do card.
pub fn cta_buttons(presentation: &Presentation) -> String {
    match live_website(presentation) {
        Some(website) => format!(
            "[![Abrir site]({})]({website}) [![Código]({})]({})",
            badge("▸ ABRIR SITE", COLOR_GOLD, "for-the-badge"),
            badge("CÓDIGO", COLOR_MUTED, "for-the-badge"),
            presentation.github
        ),
        None => format!("[![Código]({})]({})", badge("CÓDIGO", COLOR_GOLD, "for-the-badge"), presentation.github),
    }
}

/// `status_pill(presentation)`: selo derivado da verificação.
pub fn status_pill(presentation: &Presentation) -> String {
    if presentation.has_live_website() {
        format!("![Site verificado]({})", badge("● SITE VERIFICADO", COLOR_GREEN, "flat-square"))
    } else if presentation.website_declared.as_deref().is_some_and(|declared| !declared.is_empty()) {
        format!("![Site fora do ar]({})", badge("○ SITE FORA DO AR", COLOR_MUTED, "flat-square"))
    } else {
        format!("![Código aberto]({})", badge("○ CÓDIGO ABERTO", COLOR_MUTED, "flat-square"))
    }
}

/// `cta_cell(presentation)`: a célula de acesso das tabelas.
pub fn cta_cell(presentation: &Presentation) -> String {
    match live_website(presentation) {
        Some(website) => format!("**[▸ Abrir site]({website})** · [código]({})", presentation.github),
        None => format!("[código]({})", presentation.github),
    }
}
