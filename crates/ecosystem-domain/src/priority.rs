//! Prioridades de exibição — `marketing_priority()` de
//! `scripts/project_catalog.py` e `featured_score()` de `scripts/update_profile.py`.

use chrono::{DateTime, Utc};

use crate::presentation::CheckStatus;
use crate::repo::RepoFacts;
use crate::text::truthy;
use crate::timestamps::{parse_github_timestamp, python_days};

/// Peso editorial por nome (`FEATURED_PRIORITY`). Dado escondido em código;
/// no banco vira `ecosystem.featured_entries.priority`.
const FEATURED_PRIORITY: [(&str, f64); 12] = [
    ("Projeto-Baluarte", 120.0),
    ("Veritas", 115.0),
    ("Ark-Initiative", 105.0),
    ("AEGIS", 100.0),
    ("baluarte-obra-segura", 100.0),
    ("Project-Vanguard", 95.0),
    ("Digital-Logic-Sim-CE", 90.0),
    ("CHIPS-Digital-Logic-Sim-Lucas-Belucci", 88.0),
    ("taxforge", 86.0),
    ("DailyPlanner", 82.0),
    ("Projeto-Baluarte-World-Game", 80.0),
    ("Recycle-game", 78.0),
];

/// `round()` do Python para floats: empate vai para o **par**
/// (`round(12.5) == 12`, `round(13.5) == 14`). O `f64::round` do Rust
/// arredondaria para longe do zero e daria 13.
pub fn py_round(value: f64) -> i64 {
    value.round_ties_even() as i64
}

/// Prioridade de exibição, 0–100. Só ordena a vitrine; nunca é mostrada.
pub fn marketing_priority(
    repo: &RepoFacts,
    check: Option<CheckStatus>,
    featured_order: Option<i64>,
    featured_priority: Option<i64>,
    description_ok: bool,
) -> i64 {
    let mut score = match check {
        Some(CheckStatus::Verified) => 45,
        Some(_) => 5,
        None => 0,
    };
    if let Some(priority) = featured_priority {
        // Mesmas operações em ponto flutuante do Python: 70/100*25 dá
        // exatamente 17.5, que arredonda para 18.
        score += py_round(priority.clamp(0, 100) as f64 / 100.0 * 25.0);
    } else if let Some(order) = featured_order {
        score += (25 - (order - 1) * 3).max(0);
    }
    if description_ok {
        score += 10;
    }
    if !repo.private {
        score += 10;
    }
    if !repo.fork {
        score += 5;
    }
    if !repo.archived {
        score += 5;
    }
    score.clamp(0, 100)
}

/// Pontuação heurística da tabela de destaques — versão `py-classify@1`.
///
/// Mesma ordem de somas do Python, para os mesmos bits em `f64`.
pub fn featured_score_py_v1(repo: &RepoFacts, now: DateTime<Utc>) -> f64 {
    let mut score = FEATURED_PRIORITY.iter().find(|(name, _)| *name == repo.name).map_or(0.0, |(_, weight)| *weight);
    score += (repo.size.unwrap_or(0) as f64 / 1000.0).min(30.0);
    score += if truthy(repo.description.as_deref()) { 8.0 } else { 0.0 };
    score += if truthy(repo.homepage.as_deref()) { 8.0 } else { 0.0 };
    score += if repo.fork { 0.0 } else { 5.0 };
    if let Some(timestamp) = repo.activity_timestamp().and_then(parse_github_timestamp) {
        let age = python_days(now, timestamp).max(0);
        score += if age <= 90 {
            20.0
        } else if age <= 365 {
            10.0
        } else {
            0.0
        };
    }
    score
}

#[cfg(test)]
mod tests {
    use super::py_round;

    #[test]
    fn empate_vai_para_o_par() {
        assert_eq!(12, py_round(12.5));
        assert_eq!(14, py_round(13.5));
        assert_eq!(0, py_round(0.5));
        assert_eq!(2, py_round(2.5));
    }
}
