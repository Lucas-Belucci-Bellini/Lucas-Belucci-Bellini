//! `profile-core sync contributions` — port de `scripts/update_contribution_timeline.py`.
//!
//! Uma consulta GraphQL (`contributionsCollection`) por janela mensal dos
//! últimos 365 dias; o resultado vai para `docs/assets/contributions-timeline-data.json`
//! e para a página `contributions-timeline.html` (o mesmo template do Python,
//! em `templates/`), e — com banco — para `ecosystem.metric_samples`
//! (`profile.contributions.*`, uma série por janela).
//!
//! Paridade: `tests/fixtures/parity/contributions.json` (o `main()` do Python
//! contra um GraphQL simulado: pedidos, respostas, arquivos e erros).

use std::future::Future;
use std::path::Path;

use chrono::{DateTime, Datelike, Days, NaiveDate, Utc};
use ecosystem_domain::discovery::py_str;
use ecosystem_domain::monitor::{PyException, py_get, py_index_str, py_type_name};
use github_client::{ApiError, Client, Settings};
use serde_json::{Map, Value, json};

/// Dado bruto, relativo à raiz.
pub const DATA_FILE: &str = "docs/assets/contributions-timeline-data.json";
/// Página, relativa à raiz.
pub const HTML_FILE: &str = "docs/assets/contributions-timeline.html";
/// `User-Agent` do script Python.
pub const USER_AGENT: &str = "Lucas-Belucci-Bellini-contributions-timeline";
/// A consulta, com o mesmo texto do Python.
pub const QUERY: &str = "
query($login:String!, $from:DateTime!, $to:DateTime!) {
  user(login:$login) {
    contributionsCollection(from:$from, to:$to) {
      contributionCalendar { totalContributions }
      totalCommitContributions
      totalIssueContributions
      totalPullRequestContributions
      totalPullRequestReviewContributions
      totalRepositoryContributions
      restrictedContributionsCount
    }
  }
}
";
const TEMPLATE: &str = include_str!("../templates/contributions-timeline.html");

/// As sete métricas de cada janela: chave no JSON → chave em `metric_definitions`.
pub const METRICS: [(&str, &str); 7] = [
    ("total", "profile.contributions.total"),
    ("commits", "profile.contributions.commits"),
    ("pull_requests", "profile.contributions.pull_requests"),
    ("issues", "profile.contributions.issues"),
    ("reviews", "profile.contributions.reviews"),
    ("repositories", "profile.contributions.repositories"),
    ("restricted", "profile.contributions.restricted"),
];

/// Quem responde o GraphQL — o GitHub, ou respostas fixas nos testes.
pub trait GraphQl {
    /// Uma consulta com as variáveis dadas; devolve o corpo inteiro.
    fn query(&self, variables: Value) -> impl Future<Output = Result<Value, ApiError>>;
}

/// O GitHub de verdade.
pub struct GitHub(pub Client);

impl GitHub {
    /// Cliente com o `User-Agent` do script e o token dado.
    pub fn new(token: String) -> Result<Self, github_client::BuildError> {
        Ok(Self(Client::new(Settings::new(USER_AGENT))?.with_token(Some(token))))
    }
}

impl GraphQl for GitHub {
    async fn query(&self, variables: Value) -> Result<Value, ApiError> {
        self.0.graphql(QUERY, variables).await
    }
}

/// `period_ranges(start, end)`: janelas do dia 1 ao último dia de cada mês,
/// cortadas nas pontas. Com `start` depois de `end` no mesmo mês, sai uma
/// janela invertida — como no Python.
pub fn period_ranges(start: NaiveDate, end: NaiveDate) -> Vec<(NaiveDate, NaiveDate)> {
    let mut ranges = Vec::new();
    let mut cursor = start.with_day(1).expect("dia 1 existe");
    while cursor <= end {
        let next_month = if cursor.month() == 12 {
            NaiveDate::from_ymd_opt(cursor.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(cursor.year(), cursor.month() + 1, 1)
        }
        .expect("mês seguinte existe");
        let last = next_month.pred_opt().expect("véspera existe");
        ranges.push((cursor.max(start), last.min(end)));
        cursor = next_month;
    }
    ranges
}

fn exception(error: PyException) -> String {
    error.message
}

/// `query_period()`: uma janela. O erro é o texto que o Python mostraria.
async fn query_period<G: GraphQl>(
    graphql: &G,
    login: &str,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Map<String, Value>, String> {
    let variables = json!({
        "login": login,
        "from": format!("{start}T00:00:00Z"),
        "to": format!("{end}T23:59:59Z"),
    });
    let result = match graphql.query(variables).await {
        Ok(result) => result,
        Err(ApiError::Http { status, .. }) => return Err(format!("GitHub GraphQL HTTP {status}")),
        Err(other) => return Err(other.to_string()),
    };
    let errors = py_get(&result, "errors").map_err(exception)?.cloned().unwrap_or(Value::Null);
    if ecosystem_domain::discovery::json_truthy(&errors) {
        let Value::Array(errors) = &errors else {
            return Err("'str' object has no attribute 'get'".into());
        };
        let messages: Result<Vec<String>, String> = errors
            .iter()
            .map(|error| {
                Ok(py_get(error, "message").map_err(exception)?.map_or_else(|| "GraphQL error".to_string(), py_str))
            })
            .collect();
        return Err(messages?.join("; "));
    }
    let collection = py_index_str(&result, "data")
        .and_then(|data| py_index_str(data, "user"))
        .and_then(|user| py_index_str(user, "contributionsCollection"))
        .map_err(exception)?;
    let total = py_index_str(collection, "contributionCalendar")
        .and_then(|calendar| py_index_str(calendar, "totalContributions"))
        .map_err(exception)?;
    let mut values = Map::new();
    values.insert("total".into(), total.clone());
    for (key, field) in [
        ("commits", "totalCommitContributions"),
        ("pull_requests", "totalPullRequestContributions"),
        ("issues", "totalIssueContributions"),
        ("reviews", "totalPullRequestReviewContributions"),
        ("repositories", "totalRepositoryContributions"),
        ("restricted", "restrictedContributionsCount"),
    ] {
        values.insert(key.into(), py_index_str(collection, field).map_err(exception)?.clone());
    }
    Ok(values)
}

/// `collect()`: a janela móvel de 365 dias até a data (UTC) de `now`. Uma
/// janela que falha derruba a coleta inteira, sem gravar nada.
pub async fn collect<G: GraphQl>(graphql: &G, login: &str, now: DateTime<Utc>) -> Result<Value, String> {
    let today = now.date_naive();
    let start = today.checked_sub_days(Days::new(365)).expect("data válida");
    let mut rows = Vec::new();
    for (period_start, period_end) in period_ranges(start, today) {
        let mut values = query_period(graphql, login, period_start, period_end).await?;
        values.insert("period_start".into(), Value::String(period_start.to_string()));
        values.insert("period_end".into(), Value::String(period_end.to_string()));
        rows.push(Value::Object(values));
    }
    Ok(json!({
        "login": login,
        "window_start": start.to_string(),
        "window_end": today.to_string(),
        "generated_at": now.format("%Y-%m-%dT%H:%M:%S+00:00").to_string(),
        "source": "GitHub GraphQL API / contributionsCollection",
        "rows": rows,
    }))
}

/// `json.dumps(payload, ensure_ascii=False, indent=2) + "\n"`.
pub fn render_data(payload: &Value) -> String {
    let mut text = serde_json::to_string_pretty(payload).expect("Value sempre serializa");
    text.push('\n');
    text
}

/// `html.escape(text, quote=True)`.
pub fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            other => out.push(other),
        }
    }
    out
}

/// `render_html(payload)`: o template do Python com o JSON compacto embutido.
pub fn render_html(payload: &Value) -> String {
    let data_json = serde_json::to_string(payload).expect("Value sempre serializa");
    let (head, rest) = TEMPLATE.split_once("{{LOGIN}}").expect("marcador de login");
    let (middle, rest) = rest.split_once("{{GENERATED}}").expect("marcador de carimbo");
    let (before_data, tail) = rest.split_once("{{DATA_JSON}}").expect("marcador de dados");
    format!(
        "{head}{}{middle}{}{before_data}{data_json}{tail}",
        html_escape(&py_str(&payload["login"])),
        html_escape(&py_str(&payload["generated_at"])),
    )
}

/// Grava o JSON e a página, como o Python.
pub fn write(root: &Path, payload: &Value) -> std::io::Result<()> {
    let data = root.join(DATA_FILE);
    if let Some(parent) = data.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&data, render_data(payload))?;
    std::fs::write(root.join(HTML_FILE), render_html(payload))
}

/// As amostras para o banco: (métrica, início, fim, valor). Valor que não é
/// número (o Python grava o que a API mandou) fica de fora, com o motivo.
pub fn samples(payload: &Value) -> (Vec<store::metrics::Sample>, Vec<String>) {
    let mut samples = Vec::new();
    let mut skipped = Vec::new();
    for row in payload["rows"].as_array().into_iter().flatten() {
        let start = format!("{}T00:00:00Z", py_str(&row["period_start"]));
        let end = format!("{}T23:59:59Z", py_str(&row["period_end"]));
        for (field, key) in METRICS {
            let value = match &row[field] {
                Value::Number(number) => number.to_string(),
                Value::String(text) if text.trim().parse::<f64>().is_ok_and(f64::is_finite) => text.trim().to_string(),
                other => {
                    skipped.push(format!("{key} {start}: {} não é número", py_type_name(other)));
                    continue;
                }
            };
            samples.push(store::metrics::Sample {
                metric_key: key.to_string(),
                value,
                window_start: Some(start.clone()),
                window_end: Some(end.clone()),
            });
        }
    }
    (samples, skipped)
}
