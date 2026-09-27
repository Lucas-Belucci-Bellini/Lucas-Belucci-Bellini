//! Slug de rota — `Presentation.slug` de `scripts/project_catalog.py`.
//!
//! Python: `re.sub(r"-+", "-", re.sub(r"[^a-z0-9]+", "-", name.lower())).strip("-")`.
//! Minúsculas do Unicode primeiro (o `K` de Kelvin vira `k` ASCII e fica),
//! depois tudo que não é `[a-z0-9]` vira um hífen só.

/// Identidade estável do projeto para rota futura (`/projects/<slug>`).
pub fn slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut previous_dash = false;
    for c in name.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
            previous_dash = false;
        } else if !previous_dash {
            out.push('-');
            previous_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn exemplos_do_catalogo() {
        assert_eq!("veritas", slug("Veritas"));
        assert_eq!("banco-de-dados", slug("-BANCO-DE-DADOS-"));
        assert_eq!("projeto-01-endless-gnome", slug("Projeto_01_endless_gnome"));
        assert_eq!("", slug("---"));
    }
}
