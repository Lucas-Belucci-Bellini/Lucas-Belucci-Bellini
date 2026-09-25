//! Status de ciclo de vida — `status_for()` de `scripts/update_profile.py`.
//!
//! Depende do relógio (`now` é injetado): um projeto passa de ativo a "em
//! desenvolvimento" no 61º dia sem push, sem nenhum dado novo (auditoria, A19).
//! Os nomes fixos abaixo são dado editorial escondido em código; no banco eles
//! viram `ecosystem.projects.lifecycle_override`.

use chrono::{DateTime, Utc};

use crate::repo::RepoFacts;
use crate::text::truthy;
use crate::timestamps::{parse_github_timestamp, python_days};

const ALWAYS_IN_DEVELOPMENT: [&str; 3] =
    ["Projeto-Baluarte", "Ark-Initiative", "CHIPS-Digital-Logic-Sim-Lucas-Belucci"];

/// Idade usada quando não há data válida (a mesma sentinela do Python).
pub const UNKNOWN_AGE_DAYS: i64 = 9999;

/// Dias desde o último push (ou atualização), como o Python calcula.
pub fn activity_age_days(repo: &RepoFacts, now: DateTime<Utc>) -> i64 {
    repo.activity_timestamp()
        .and_then(parse_github_timestamp)
        .map_or(UNKNOWN_AGE_DAYS, |timestamp| python_days(now, timestamp).max(0))
}

/// Status exibido no README — versão `py-classify@1`.
pub fn status_py_v1(repo: &RepoFacts, category: &str, now: DateTime<Utc>) -> &'static str {
    if repo.private {
        return "🔒 Private";
    }
    if repo.archived {
        return "⚪ Archived";
    }
    if category == "Academia" {
        return "🟣 Academic";
    }
    if ALWAYS_IN_DEVELOPMENT.contains(&repo.name.as_str()) {
        return "🟡 In Development";
    }
    // Os domínios `baluarte-*` extraídos estão em backlog, exceto a Obra Segura.
    if repo.name.starts_with("baluarte-") && repo.name != "baluarte-obra-segura" {
        return "🟡 In Development";
    }
    if repo.fork && !truthy(repo.pushed_at.as_deref()) {
        return "🔵 Experimental";
    }
    match activity_age_days(repo, now) {
        age if age <= 60 => "🟢 Active",
        age if age <= 365 => "🟡 In Development",
        _ => "🔵 Experimental",
    }
}
