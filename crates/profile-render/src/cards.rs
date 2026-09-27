//! Os cards do `.github/scripts/profile_cards.py`: `profile-stats.svg`,
//! `profile-streak.svg`, `profile-trophies.svg` e `profile-projects.svg`.
//!
//! Os números vêm do GraphQL (a coleta fica no `profile-core`); aqui entra o
//! dicionário `data` do Python — os valores como vieram, porque o SVG mostra
//! `str(valor)` — e saem os mesmos bytes.

use ecosystem_domain::discovery::py_str;
use serde_json::{Map, Value};

use crate::pytext::html_escape;

const SURFACE: &str = "#1d1729";
const GOLD: &str = "#d4a24e";
const GOLD_LIGHT: &str = "#e8c07a";
const PARCHMENT: &str = "#f4ecdd";
const MUTED: &str = "#a89a80";
const DIM: &str = "#77694f";
const GREEN: &str = "#3ddc84";
const RED: &str = "#e07a5f";
const PURPLE: &str = "#a68dad";

/// Os arquivos, na ordem em que o Python os grava.
pub const FILES: [&str; 4] =
    ["profile-stats.svg", "profile-streak.svg", "profile-trophies.svg", "profile-projects.svg"];

/// `esc(value)`: `html.escape(str(value), quote=True)`.
fn esc(value: &Value) -> String {
    html_escape(&py_str(value))
}

fn text(value: &str) -> Value {
    Value::String(value.to_string())
}

/// `data[key]`; ausente vale `None` (o Python já teria quebrado antes).
fn field<'a>(data: &'a Map<String, Value>, key: &str) -> &'a Value {
    data.get(key).unwrap_or(&Value::Null)
}

/// `frame(width, height, title, subtitle)`.
fn frame(width: i64, height: i64, title: &str, subtitle: &str) -> Vec<String> {
    vec![
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" role="img" aria-label="{}">"#,
            esc(&text(title))
        ),
        r##"<defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#0e0c16"/><stop offset="1" stop-color="#0b0910"/></linearGradient><linearGradient id="scan" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#d4a24e" stop-opacity="0"/><stop offset="0.5" stop-color="#e8c07a" stop-opacity="0.9"/><stop offset="1" stop-color="#d4a24e" stop-opacity="0"/></linearGradient></defs>"##.to_string(),
        r#"<rect width="100%" height="100%" rx="14" fill="url(#bg)"/>"#.to_string(),
        format!(
            r#"<rect x="5" y="5" width="{}" height="{}" rx="11" fill="none" stroke="{GOLD}" stroke-opacity="0.35"/>"#,
            width - 10,
            height - 10
        ),
        format!(
            r#"<text x="28" y="35" fill="{GOLD_LIGHT}" font-family="DejaVu Sans Mono,monospace" font-size="14" font-weight="700" letter-spacing="2">&gt;&gt; {}</text>"#,
            esc(&text(title))
        ),
        format!(
            r#"<text x="28" y="54" fill="{DIM}" font-family="DejaVu Sans Mono,monospace" font-size="10" letter-spacing="1.2">{}</text>"#,
            esc(&text(subtitle))
        ),
        format!(r#"<line x1="24" y1="66" x2="{}" y2="66" stroke="{GOLD}" stroke-opacity="0.25"/>"#, width - 24),
        r#"<rect x="24" y="67" width="180" height="2" fill="url(#scan)" opacity="0.85"/>"#.to_string(),
    ]
}

/// `metric(x, y, w, label, value, color)`.
fn metric(x: i64, y: i64, w: i64, label: &str, value: &Value, color: &str) -> String {
    format!(
        r#"<rect x="{x}" y="{y}" width="{w}" height="76" rx="8" fill="{SURFACE}" stroke="{GOLD}" stroke-opacity="0.22"/><text x="{}" y="{}" fill="{color}" font-family="DejaVu Sans Mono,monospace" font-size="25" font-weight="700">{}</text><text x="{}" y="{}" fill="{MUTED}" font-family="DejaVu Sans Mono,monospace" font-size="10" letter-spacing="1">{}</text>"#,
        x + 16,
        y + 31,
        esc(value),
        x + 16,
        y + 55,
        esc(&text(label))
    )
}

fn finish(mut lines: Vec<String>) -> String {
    lines.push("</svg>".to_string());
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// `stats_svg(data)`.
pub fn stats_svg(data: &Map<String, Value>) -> String {
    let subtitle = "GraphQL | rolling 365 days | total = commits + PRs + issues + reviews + repos";
    let mut lines = frame(880, 260, "FIELD REPORT // GITHUB SNAPSHOT", subtitle);
    lines.push(metric(24, 86, 198, "TOTAL CONTRIBUTIONS", field(data, "contributions"), GOLD_LIGHT));
    lines.push(metric(238, 86, 198, "DIRECT COMMITS", field(data, "commits"), GOLD));
    lines.push(metric(452, 86, 198, "PULL REQUESTS", field(data, "pull_requests"), GREEN));
    lines.push(metric(666, 86, 190, "ISSUES", field(data, "issues"), PURPLE));
    lines.push(metric(24, 174, 198, "REVIEWS", field(data, "reviews"), GOLD_LIGHT));
    lines.push(metric(238, 174, 198, "REPOS CREATED", field(data, "repository_contributions"), GOLD));
    lines.push(metric(452, 174, 198, "RESTRICTED", field(data, "restricted"), MUTED));
    lines.push(metric(666, 174, 190, "STATUS", &text("ONLINE"), GREEN));
    lines.push(format!(
        r#"<text x="24" y="246" fill="{DIM}" font-family="DejaVu Sans Mono,monospace" font-size="9">updated automatically | {}</text>"#,
        esc(field(data, "generated"))
    ));
    finish(lines)
}

/// `streak_svg(data)`.
pub fn streak_svg(data: &Map<String, Value>) -> String {
    let mut lines =
        frame(495, 195, "CONTINUITY // FIELD STREAK", "total contributions vs. direct commits | rolling 365 days");
    lines.push(metric(24, 84, 142, "TOTAL", field(data, "contributions"), GOLD_LIGHT));
    lines.push(metric(176, 84, 142, "DIRECT COMMITS", field(data, "commits"), GOLD));
    lines.push(metric(328, 84, 143, "PRs", field(data, "pull_requests"), GREEN));
    lines.push(format!(
        r#"<text x="24" y="180" fill="{DIM}" font-family="DejaVu Sans Mono,monospace" font-size="9">source: GitHub GraphQL | {}</text>"#,
        esc(field(data, "generated"))
    ));
    finish(lines)
}

/// `projects_svg()`: os quatro projetos fixos do painel.
pub fn projects_svg() -> String {
    let mut lines =
        frame(880, 236, "ARSENAL // PROJETOS EM DESTAQUE", "cards locais · links diretos preservados no README");
    let projects = [
        ("Projeto Baluarte", "J.A.R.V.I.S. · Vite · Electron", GOLD_LIGHT),
        ("Digital Logic Sim CE", "CPUs de 8 → 64 bits · Unity", GREEN),
        ("Stock Analyzer", "IA · RSI · MACD · alertas", PURPLE),
        ("Baluarte Obra Segura", "web · Electron · segurança", RED),
    ];
    let positions = [(24, 84), (452, 84), (24, 158), (452, 158)];
    for ((name, description, color), (x, y)) in projects.into_iter().zip(positions) {
        lines.push(format!(
            r#"<rect x="{x}" y="{y}" width="404" height="58" rx="8" fill="{SURFACE}" stroke="{GOLD}" stroke-opacity="0.22"/>"#
        ));
        lines.push(format!(
            r#"<circle cx="{}" cy="{}" r="7" fill="{color}"/><text x="{}" y="{}" fill="{PARCHMENT}" font-family="DejaVu Sans Mono,monospace" font-size="13" font-weight="700">{}</text>"#,
            x + 22,
            y + 29,
            x + 42,
            y + 25,
            esc(&text(name))
        ));
        lines.push(format!(
            r#"<text x="{}" y="{}" fill="{MUTED}" font-family="DejaVu Sans Mono,monospace" font-size="10">{}</text>"#,
            x + 42,
            y + 43,
            esc(&text(description))
        ));
    }
    finish(lines)
}

/// `trophies_svg(data)`.
pub fn trophies_svg(data: &Map<String, Value>) -> String {
    let mut lines = frame(
        880,
        190,
        "TROFÉUS // MISSÃO EM CAMPO",
        "marcos derivados da atividade oficial do GitHub · sem serviço externo",
    );
    let labels = [
        ("COMMIT ENGINE", "commits", GOLD_LIGHT),
        ("PR COMMANDER", "pull_requests", GREEN),
        ("ISSUE SCOUT", "issues", PURPLE),
        ("REVIEW SENTINEL", "reviews", GOLD),
        ("REPO ARCHITECT", "repositories", RED),
    ];
    let mut x = 24;
    for (label, key, color) in labels {
        lines.push(metric(x, 84, 158, label, field(data, key), color));
        x += 158 + 16;
    }
    finish(lines)
}

/// Os quatro cards, na ordem de [`FILES`].
pub fn all(data: &Map<String, Value>) -> [String; 4] {
    [stats_svg(data), streak_svg(data), trophies_svg(data), projects_svg()]
}
