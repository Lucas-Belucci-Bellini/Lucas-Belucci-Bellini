//! `assets/profile-snapshot.svg` e o carimbo `generated_at`.

use chrono::{Datelike, NaiveDateTime, Timelike};
use ecosystem_domain::classify::classify_py_v1;
use ecosystem_domain::lifecycle::status_py_v1;
use ecosystem_domain::monitor::is_profile_repository;
use ecosystem_domain::repo::RepoFacts;
use ecosystem_domain::timestamps::{PyLocalDatetime, py_fromisoformat_local};

use crate::pytext::html_escape;
use crate::{OWNER, Profile, RenderError};

/// `source_timestamp(repos, now).strftime("%Y-%m-%d %H:%M UTC")`.
///
/// O push mais recente do inventário, **exceto o do próprio perfil** (o
/// monitor horário empurra snapshots para ele; A7). Sem nenhuma data
/// legível, o relógio da execução (`now_local`, no fuso em que foi dado).
///
/// Como no Python: a hora impressa é a do fuso da data escolhida (um
/// `+02:00` sai com a hora de lá, e o texto diz "UTC" mesmo assim); o empate
/// fica com a primeira; e datas com e sem fuso no mesmo inventário não se
/// comparam — `TypeError`.
pub fn generated_at(repos: &[RepoFacts], now_local: NaiveDateTime) -> Result<String, RenderError> {
    let profile = format!("{OWNER}/{OWNER}");
    let mut latest: Option<PyLocalDatetime> = None;
    for repo in repos.iter().filter(|repo| !is_profile_repository(&repo.full_name, &profile)) {
        let Some(value) = repo.activity_timestamp() else { continue };
        let parsed = py_fromisoformat_local(&value.replace('Z', "+00:00"));
        latest = match (latest, parsed) {
            (_, PyLocalDatetime::Invalid) => latest,
            (None, _) => Some(parsed),
            (Some(PyLocalDatetime::Aware { instant: best, .. }), PyLocalDatetime::Aware { instant, .. }) => {
                if instant > best { Some(parsed) } else { latest }
            }
            (Some(PyLocalDatetime::Naive(best)), PyLocalDatetime::Naive(local)) => {
                if local > best {
                    Some(parsed)
                } else {
                    latest
                }
            }
            _ => {
                return Err(RenderError::PythonCrash {
                    kind: "TypeError",
                    detail: "can't compare offset-naive and offset-aware datetimes".into(),
                });
            }
        };
    }
    let local = match latest {
        Some(PyLocalDatetime::Aware { local, .. } | PyLocalDatetime::Naive(local)) => local,
        _ => now_local,
    };
    // `%Y` do glibc não completa com zeros (o ano 5 sai "5").
    Ok(format!(
        "{}-{:02}-{:02} {:02}:{:02} UTC",
        local.year(),
        local.month(),
        local.day(),
        local.hour(),
        local.minute()
    ))
}

/// `render_snapshot_svg(...)`: oito cartões de contagem, 1100×300.
pub fn svg(profile: &Profile) -> String {
    let repos = profile.inputs.repos;
    let now = profile.inputs.now;
    let public = repos.iter().filter(|repo| !repo.private).count();
    let active = repos.iter().filter(|repo| status_py_v1(repo, classify_py_v1(repo), now) == "🟢 Active").count();
    let academic = repos.iter().filter(|repo| classify_py_v1(repo) == "Academia").count();
    let values = [
        ("REPOSITORIES", repos.len(), "inventory"),
        ("PUBLIC", public, "visible"),
        ("PRIVATE", repos.len() - public, "metadata"),
        ("DEPLOYMENTS", profile.verified_sites(), "HTTP 200"),
        ("ACTIVE", active, "status"),
        ("ACADEMIC", academic, "portfolio"),
        ("LANGUAGES", profile.rows.len(), "public"),
        ("EXCLUDED", profile.inputs.excluded, "editorial"),
    ];
    let (width, height) = (1100, 300);
    let (background, surface, border) = ("#0e0c16", "#1d1729", "#4b3a5c");
    let (light, muted, gold, green) = ("#f4ecdd", "#a89f91", "#d4a24e", "#3ddc84");
    let mut svg = vec![
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
        ),
        format!(r#"<rect width="{width}" height="{height}" rx="18" fill="{background}"/>"#),
        format!(
            r#"<rect x="18" y="18" width="{}" height="{}" rx="14" fill="none" stroke="{border}" stroke-width="2"/>"#,
            width - 36,
            height - 36
        ),
        format!(
            r#"<text x="48" y="58" fill="{gold}" font-family="sans-serif" font-size="22" font-weight="700">&gt;&gt; GITHUB SNAPSHOT // FIELD REPORT &lt;&lt;</text>"#
        ),
        format!(
            r#"<text x="48" y="84" fill="{muted}" font-family="sans-serif" font-size="13">authenticated inventory | public-safe metrics | generated {}</text>"#,
            html_escape(profile.inputs.generated_at)
        ),
    ];
    let (card_w, card_h, gap) = (245, 78, 16);
    let (start_x, start_y) = (48, 105);
    for (index, (label, value, note)) in values.iter().enumerate() {
        let (row, col) = (index / 4, index % 4);
        let (x, y) = (start_x + col * (card_w + gap), start_y + row * (card_h + gap));
        let value_color = if matches!(*label, "DEPLOYMENTS" | "ACTIVE") { green } else { gold };
        svg.extend([
            format!(
                r#"<rect x="{x}" y="{y}" width="{card_w}" height="{card_h}" rx="8" fill="{surface}" stroke="{border}"/>"#
            ),
            format!(
                r#"<text x="{}" y="{}" fill="{value_color}" font-family="sans-serif" font-size="25" font-weight="700">{}</text>"#,
                x + 16,
                y + 29,
                html_escape(&value.to_string())
            ),
            format!(
                r#"<text x="{}" y="{}" fill="{light}" font-family="sans-serif" font-size="12" font-weight="700">{}</text>"#,
                x + 16,
                y + 50,
                html_escape(label)
            ),
            format!(
                r#"<text x="{}" y="{}" fill="{muted}" font-family="sans-serif" font-size="10">{}</text>"#,
                x + 16,
                y + 66,
                html_escape(note)
            ),
        ]);
    }
    svg.push("</svg>".into());
    let mut text = svg.join("\n");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(full_name: &str, pushed_at: &str) -> RepoFacts {
        RepoFacts {
            name: full_name.rsplit('/').next().unwrap().into(),
            full_name: full_name.into(),
            pushed_at: Some(pushed_at.into()),
            ..RepoFacts::default()
        }
    }

    fn now() -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 9, 25).unwrap().and_hms_opt(12, 0, 0).unwrap()
    }

    #[test]
    fn carimbo_ignora_o_perfil_e_usa_o_fuso_da_data() {
        let repos = [
            repo("o/a", "2026-09-20T10:00:00Z"),
            repo("o/b", "2026-09-21T01:30:00+05:00"),
            repo("lucas-belucci-bellini/LUCAS-BELUCCI-BELLINI", "2026-09-25T11:17:00Z"),
        ];
        assert_eq!("2026-09-21 01:30 UTC", generated_at(&repos, now()).unwrap());
    }

    #[test]
    fn sem_data_legivel_vale_o_relogio() {
        assert_eq!("2026-09-25 12:00 UTC", generated_at(&[repo("o/a", "ontem")], now()).unwrap());
    }

    #[test]
    fn datas_com_e_sem_fuso_nao_se_comparam() {
        let repos = [repo("o/a", "2026-09-20T10:00:00Z"), repo("o/b", "2026-09-21T10:00:00")];
        assert!(matches!(generated_at(&repos, now()), Err(RenderError::PythonCrash { kind: "TypeError", .. })));
        // Uma só data sem fuso não compara com nada.
        assert_eq!("2026-09-21 10:00 UTC", generated_at(&repos[1..], now()).unwrap());
    }

    #[test]
    fn ano_sem_zeros_a_esquerda() {
        assert_eq!("5-01-02 03:04 UTC", generated_at(&[repo("o/a", "0005-01-02T03:04:00Z")], now()).unwrap());
    }
}
