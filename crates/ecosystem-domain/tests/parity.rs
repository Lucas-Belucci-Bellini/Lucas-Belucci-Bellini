//! Paridade com o Python: cada caso de `tests/fixtures/parity/domain.json`
//! (gerado por `tests/test_parity_domain.py` a partir das funções Python
//! reais) tem de dar exatamente a mesma saída aqui.
//!
//! ```text
//! OLD PYTHON ──▶ tests/fixtures/parity/domain.json ◀── NEW RUST
//! ```
//!
//! Um grupo de casos que o Python passe a gerar e este arquivo não conheça
//! reprova o teste: paridade parcial não conta como paridade.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use ecosystem_domain::lifecycle::status_py_v1;
use ecosystem_domain::text::py_is_space;
use ecosystem_domain::timestamps::{PyIsoDatetime, parse_github_timestamp, py_fromisoformat};
use ecosystem_domain::{
    CheckStatus, RepoFacts, WebsiteCheck, canonical_category, catalog_entry, classify_py_v1, describe,
    discover_project_website, featured_score_py_v1, looks_like_http_url, marketing_priority, normalize_site_overrides,
    resolve_presentation, slug,
};

#[derive(Deserialize)]
struct Fixture {
    schema: String,
    now: String,
    cases: BTreeMap<String, Vec<Value>>,
}

#[derive(Deserialize)]
struct Case<I, E> {
    input: I,
    expected: E,
}

#[derive(Deserialize)]
struct CheckInput {
    status: CheckStatus,
    http_status: i64,
    final_url: Option<String>,
}

#[derive(Deserialize)]
struct StatusInput {
    repo: RepoFacts,
    category: String,
}

#[derive(Deserialize)]
struct MarketingInput {
    repo: RepoFacts,
    check: Option<CheckInput>,
    featured_order: Option<i64>,
    featured_priority: Option<i64>,
    description_ok: bool,
}

#[derive(Deserialize)]
struct PresentationInput {
    repo: RepoFacts,
    check: Option<CheckInput>,
    website: Option<String>,
    website_source: String,
    category: String,
    status: String,
    description: String,
    featured_order: Option<i64>,
    featured_priority: Option<i64>,
}

/// `py_timestamp()` de `tests/test_parity_domain.py`.
#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Timestamp {
    Aware { micros_before_now: i64 },
    Naive,
    Invalid,
}

#[derive(Deserialize)]
struct DiscoveryInput {
    repo: RepoFacts,
    overrides: Value,
}

fn fixture() -> Fixture {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/parity/domain.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_str(&text).expect("fixture de paridade é JSON válido")
}

/// O `WebsiteCheck` que `tests/test_parity_domain.py::_check_obj` monta.
fn website_check(input: Option<CheckInput>, url: Option<&str>) -> Option<WebsiteCheck> {
    input.map(|check| WebsiteCheck {
        url: url.unwrap_or_default().to_string(),
        status: check.status,
        http_status: check.http_status,
        final_url: check.final_url.filter(|u| !u.is_empty()).unwrap_or_else(|| url.unwrap_or_default().to_string()),
    })
}

fn timestamp(value: &str, now: DateTime<Utc>) -> Timestamp {
    match py_fromisoformat(value) {
        PyIsoDatetime::Aware(instant) => {
            Timestamp::Aware { micros_before_now: (now - instant).num_microseconds().expect("dentro da faixa de i64") }
        }
        PyIsoDatetime::Naive => Timestamp::Naive,
        PyIsoDatetime::Invalid => Timestamp::Invalid,
    }
}

/// Roda `actual` em cada caso do grupo e devolve as divergências.
fn compare<I, E>(cases: &[Value], actual: impl Fn(I) -> E) -> Vec<String>
where
    I: for<'de> Deserialize<'de>,
    E: for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
{
    cases
        .iter()
        .filter_map(|raw| {
            let case: Case<I, E> = serde_json::from_value(raw.clone()).expect("caso no formato do grupo");
            let got = actual(case.input);
            (got != case.expected)
                .then(|| format!("  entrada {}\n    Python: {:?}\n    Rust:   {got:?}", raw["input"], case.expected))
        })
        .collect()
}

#[test]
fn rust_reproduz_o_python_em_todos_os_casos() {
    let fixture = fixture();
    assert_eq!("lucas-belucci-bellini/parity-domain@1", fixture.schema);
    let now: DateTime<Utc> = parse_github_timestamp(&fixture.now).expect("`now` do fixture é válido");

    let mut failures: Vec<String> = Vec::new();
    let mut total = 0;
    for (group, cases) in &fixture.cases {
        assert!(!cases.is_empty(), "grupo {group} vazio no fixture");
        total += cases.len();
        let diverged = match group.as_str() {
            "looks_like_http_url" => compare(cases, |url: String| looks_like_http_url(&url)),
            "slug" => compare(cases, |name: String| slug(&name)),
            "describe" => compare(cases, |text: Option<String>| describe(text.as_deref())),
            "canonical_category" => compare(cases, |label: String| canonical_category(&label).to_string()),
            "classify" => compare(cases, |repo: RepoFacts| classify_py_v1(&repo).to_string()),
            "status_for" => compare(cases, |i: StatusInput| status_py_v1(&i.repo, &i.category, now).to_string()),
            "github_timestamp" => compare(cases, |value: String| timestamp(&value.replace('Z', "+00:00"), now)),
            "fromisoformat" => compare(cases, |value: String| timestamp(&value, now)),
            // Mesmos bits em f64, não "quase igual".
            "featured_score" => compare(cases, |repo: RepoFacts| featured_score_py_v1(&repo, now)),
            "marketing_priority" => compare(cases, |i: MarketingInput| {
                marketing_priority(
                    &i.repo,
                    i.check.map(|c| c.status),
                    i.featured_order,
                    i.featured_priority,
                    i.description_ok,
                )
            }),
            "resolve_presentation" => compare(cases, |i: PresentationInput| {
                let check = website_check(i.check, i.website.as_deref());
                catalog_entry(&resolve_presentation(
                    &i.repo,
                    check.as_ref(),
                    i.website.as_deref(),
                    &i.website_source,
                    &i.category,
                    &i.status,
                    &i.description,
                    i.featured_order,
                    i.featured_priority,
                ))
            }),
            "normalize_site_overrides" => compare(cases, |raw: Value| normalize_site_overrides(&raw)),
            "discover_project_website" => compare(cases, |i: DiscoveryInput| {
                let (url, source) = discover_project_website(&i.repo, &normalize_site_overrides(&i.overrides));
                (url, source.as_str().to_string())
            }),
            "python_whitespace" => {
                let python: HashSet<u32> = cases.iter().map(|c| c.as_u64().expect("code point") as u32).collect();
                (0..=0x10FFFF_u32)
                    .filter_map(char::from_u32)
                    .filter(|c| py_is_space(*c) != python.contains(&u32::from(*c)))
                    .map(|c| format!("  U+{:04X}: Python isspace={}", u32::from(c), python.contains(&u32::from(c))))
                    .collect()
            }
            other => vec![format!("  grupo `{other}` gerado pelo Python e sem conferência no Rust")],
        };
        if !diverged.is_empty() {
            failures.push(format!("{group}: {} de {} divergem\n{}", diverged.len(), cases.len(), diverged.join("\n")));
        }
    }

    assert!(total > 400, "fixture encolheu para {total} casos; a paridade deixa de provar o que promete");
    assert!(failures.is_empty(), "Rust diverge do Python:\n{}", failures.join("\n"));
}

/// Os casos-armadilha precisam continuar no fixture: sem eles, um port
/// ingênuo (`f64::round`, `trim()`, `$` do Rust) passaria.
#[test]
fn armadilhas_continuam_no_fixture() {
    let fixture = fixture();
    let group = |name: &str| fixture.cases.get(name).unwrap_or_else(|| panic!("grupo {name} sumiu"));

    let tie = group("marketing_priority")
        .iter()
        .find(|c| c["input"]["featured_priority"] == 50)
        .expect("caso de empate priority=50");
    assert_eq!(87, tie["expected"], "round(12.5) == 12 no Python");

    assert!(
        group("looks_like_http_url").iter().any(|c| c["input"] == "https://example.org\n" && c["expected"] == true)
    );
    assert!(group("describe").iter().any(|c| c["input"].as_str().is_some_and(|d| d.contains('\u{1f}'))));
    assert!(group("status_for").iter().any(|c| c["input"]["repo"]["pushed_at"] == "2026-09-20T10:00:00"));
}
