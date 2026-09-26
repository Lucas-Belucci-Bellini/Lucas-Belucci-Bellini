//! `profile-core sync commits` — port de `.github/scripts/ecosystem_watch.py`.
//!
//! Lê o estado anterior (`docs/ECOSYSTEM-COMMIT-STATE.json`), varre os
//! repositórios públicos do dono (sem forks, sem o próprio perfil), consulta o
//! último commit do branch padrão de cada um e, se algo mudou, publica o
//! estado e o relatório (`docs/ECOSYSTEM-COMMIT-MONITOR.md`) com os mesmos
//! bytes do Python, exceto a hora da varredura. Sem mudança semântica, nada é
//! reescrito — e o contador do monitor não sobe.
//!
//! Enquanto o Python publica (modo B), o JSON versionado continua sendo o
//! estado de referência dos dois: o Rust lê o mesmo arquivo e, com banco,
//! grava as transições em `ecosystem.commit_observations` e os contadores em
//! `ecosystem.metric_samples` (D-020).
//!
//! A paridade é provada por `tests/fixtures/parity/monitor.json` (o `main()`
//! do Python sobre respostas fixas) e pelo teste de ponta a ponta contra um
//! GitHub simulado.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use ecosystem_domain::discovery::{json_truthy, py_str};
use ecosystem_domain::monitor::{
    LEGACY_BASELINE, PyException, STATE_SCHEMA, ahead_by, is_profile_repository, latest_from_commits, py_prefix,
    py_type_name, sha_text,
};
use ecosystem_domain::pyjson::py_int;
use github_client::{ApiError, Client, Retry, py_quote};
use serde_json::{Map, Value, json};

/// Estado, relativo à raiz.
pub const STATE_FILE: &str = "docs/ECOSYSTEM-COMMIT-STATE.json";
/// Relatório, relativo à raiz.
pub const REPORT_FILE: &str = "docs/ECOSYSTEM-COMMIT-MONITOR.md";
/// `User-Agent` do `ecosystem_watch.py`.
pub const USER_AGENT: &str = "profile-ecosystem-watch";

/// De onde vêm as respostas da API — o GitHub, ou respostas fixas nos testes.
pub trait Source {
    /// GET de um caminho REST.
    fn get(&self, path: &str) -> impl std::future::Future<Output = Result<Value, ApiError>>;
}

/// O GitHub de verdade, com a política de novas tentativas do monitor.
pub struct GitHub(pub Client);

impl Source for GitHub {
    async fn get(&self, path: &str) -> Result<Value, ApiError> {
        self.0.get_json(path, Retry::Watch).await
    }
}

/// Onde o Python quebraria com traceback (código de saída 1, nada gravado).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("o ecosystem_watch.py quebraria com {kind}: {message}")]
pub struct Crash {
    /// Exceção do Python.
    pub kind: &'static str,
    /// O que aconteceu.
    pub message: String,
}

impl From<PyException> for Crash {
    fn from(error: PyException) -> Self {
        Self { kind: error.kind, message: error.message }
    }
}

impl From<ApiError> for Crash {
    fn from(error: ApiError) -> Self {
        let kind = match &error {
            ApiError::Http { .. } => "HTTPError",
            ApiError::Url(_) => "URLError",
            ApiError::Timeout => "TimeoutError",
            ApiError::Transport(_) => "RemoteDisconnected",
            ApiError::Json(_) => "JSONDecodeError",
        };
        Self { kind, message: error.to_string() }
    }
}

/// Quem é o dono e qual repositório é o perfil (`GH_USER`, `PROFILE_REPOSITORY_NAME`).
#[derive(Debug, Clone)]
pub struct Identity {
    /// Dono dos repositórios varridos.
    pub user: String,
    /// Nome do repositório do perfil, que não é projeto.
    pub profile: String,
}

impl Identity {
    /// Do ambiente, com os padrões do Python.
    pub fn from_env() -> Self {
        let read = |name: &str| std::env::var(name).unwrap_or_else(|_| "Lucas-Belucci-Bellini".into());
        Self { user: read("GH_USER"), profile: read("PROFILE_REPOSITORY_NAME") }
    }
}

/// O estado anterior, como o Python o lê.
#[derive(Debug, Clone)]
pub struct Previous {
    state: Value,
    /// `project_commits` anterior (ou o baseline).
    pub project_commits: i64,
    /// `monitor_commits` anterior (ou 0).
    pub monitor_commits: i64,
}

impl Previous {
    /// `json.loads(STATE.read_text())` — ausente ou ilegível vira `{}` (e o
    /// contador volta ao baseline: achado A23); o que não é objeto derruba.
    pub fn read(root: &Path) -> Result<Self, Crash> {
        let state = std::fs::read(root.join(STATE_FILE))
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .unwrap_or_else(|| Value::Object(Map::new()));
        let crash = |value: &Value| Crash {
            kind: "AttributeError",
            message: format!("'{}' object has no attribute 'get'", py_type_name(value)),
        };
        let Value::Object(object) = &state else { return Err(crash(&state)) };
        let empty = Value::Object(Map::new());
        let metrics = object.get("metrics").unwrap_or(&empty);
        let Value::Object(metrics) = metrics else { return Err(crash(metrics)) };
        let counter = |key: &str, default: i64| -> Result<i64, Crash> {
            match metrics.get(key) {
                None => Ok(default),
                Some(value) => py_int(value).map_err(|error| Crash {
                    kind: error.python_name(),
                    message: format!("invalid literal for int(): {value}"),
                }),
            }
        };
        let project_commits = counter("project_commits", LEGACY_BASELINE)?;
        let monitor_commits = counter("monitor_commits", 0)?;
        Ok(Self { state, project_commits, monitor_commits })
    }

    /// `previous_state.get("repositories", {})`.
    fn repositories(&self) -> Value {
        self.state.get("repositories").cloned().unwrap_or_else(|| Value::Object(Map::new()))
    }

    /// `previous.get(name, {}).get("sha")`, avaliado onde o Python avalia.
    fn old_sha(&self, name: &str) -> Result<Value, Crash> {
        let repositories = self.repositories();
        let Value::Object(repositories) = &repositories else {
            return Err(PyException { kind: "AttributeError", message: attribute_get(&repositories) }.into());
        };
        match repositories.get(name) {
            None => Ok(Value::Null),
            Some(Value::Object(entry)) => Ok(entry.get("sha").cloned().unwrap_or(Value::Null)),
            Some(other) => Err(PyException { kind: "AttributeError", message: attribute_get(other) }.into()),
        }
    }
}

fn attribute_get(value: &Value) -> String {
    format!("'{}' object has no attribute 'get'", py_type_name(value))
}

/// Uma mudança detectada (`changes` do Python).
#[derive(Debug, Clone, PartialEq)]
pub struct Change {
    /// Repositório.
    pub name: String,
    /// Commits novos; `None` = não determinado.
    pub count: Option<i64>,
    /// `{sha, date, message, url}`.
    pub latest: Map<String, Value>,
}

/// O resultado de uma varredura.
#[derive(Debug, Clone, PartialEq)]
pub struct Scan {
    /// Estado atual de cada repositório, na ordem da listagem.
    pub current: Map<String, Value>,
    /// Mudanças desde a varredura anterior.
    pub changes: Vec<Change>,
    /// Erros de consulta (`{name, stage, error}`).
    pub errors: Vec<Value>,
    /// Soma dos commits novos determinados.
    pub detected: i64,
    /// O estado mudou (senão, nada é publicado).
    pub changed: bool,
    /// Contadores depois da varredura.
    pub project_commits: i64,
    /// Snapshots publicados pelo monitor.
    pub monitor_commits: i64,
}

impl Scan {
    /// `project_commits + monitor_commits`.
    pub fn tracked_commits(&self) -> i64 {
        self.project_commits + self.monitor_commits
    }
}

/// `repos()`: todas as páginas da listagem pública do dono, sem forks.
async fn repos<S: Source>(source: &S, identity: &Identity) -> Result<Vec<Value>, Crash> {
    let mut out = Vec::new();
    let mut page = 1;
    loop {
        let data = source
            .get(&format!("/users/{}/repos?type=owner&per_page=100&page={page}", py_quote(&identity.user)))
            .await?;
        if !json_truthy(&data) {
            return Ok(out);
        }
        let Value::Array(items) = &data else {
            // Iterar um dicionário dá as chaves (texto), e `r.get` quebra.
            return Err(
                PyException { kind: "AttributeError", message: "'str' object has no attribute 'get'".into() }.into()
            );
        };
        for item in items {
            let Value::Object(repo) = item else {
                return Err(PyException { kind: "AttributeError", message: attribute_get(item) }.into());
            };
            if !repo.get("fork").is_some_and(json_truthy) {
                out.push(item.clone());
            }
        }
        if items.len() < 100 {
            return Ok(out);
        }
        page += 1;
    }
}

/// `compare_count()`: 0 sem base ou sem mudança; `None` se a comparação falhar.
async fn compare_count<S: Source>(source: &S, owner: &str, name: &str, base: &Value, head: &Value) -> Option<i64> {
    if !json_truthy(base) || base == head {
        return Some(0);
    }
    let path = format!("/repos/{owner}/{}/compare/{}...{}", py_quote(name), sha_text(base), sha_text(head));
    ahead_by(&source.get(&path).await.ok()?)
}

/// `latest_commit()`: último commit do branch, repositório vazio (`None`) ou
/// o `str(exc)` do que deu errado.
async fn latest_commit<S: Source>(
    source: &S,
    owner: &str,
    name: &str,
    branch: &Value,
) -> Result<Option<Map<String, Value>>, String> {
    let Value::String(branch) = branch else {
        // `quote()` de algo que não é texto.
        return Err("quote_from_bytes() expected bytes".into());
    };
    let path = format!("/repos/{owner}/{}/commits?sha={}&per_page=1", py_quote(name), py_quote(branch));
    let commits = source.get(&path).await.map_err(|error| error.to_string())?;
    latest_from_commits(&commits).map_err(|error| error.message)
}

/// Uma varredura completa, sem gravar nada.
pub async fn scan<S: Source>(source: &S, identity: &Identity, previous: &Previous) -> Result<Scan, Crash> {
    let mut current = Map::new();
    let mut changes = Vec::new();
    let mut errors = Vec::new();
    let mut detected = 0_i64;

    for repo in repos(source, identity).await? {
        if repo.get("private").is_some_and(json_truthy) {
            continue;
        }
        let name = match repo.get("name") {
            Some(Value::String(name)) => name.clone(),
            Some(other) => {
                return Err(PyException {
                    kind: "AttributeError",
                    message: format!("'{}' object has no attribute 'casefold'", py_type_name(other)),
                }
                .into());
            }
            None => return Err(PyException { kind: "KeyError", message: "'name'".into() }.into()),
        };
        if is_profile_repository(&name, &identity.profile) {
            continue;
        }
        let branch = repo
            .get("default_branch")
            .filter(|value| json_truthy(value))
            .cloned()
            .unwrap_or_else(|| Value::String("main".into()));

        let latest = match latest_commit(source, &identity.user, &name, &branch).await {
            Ok(Some(latest)) => latest,
            Ok(None) => {
                current.insert(name, json!({"branch": branch, "empty": true}));
                continue;
            }
            Err(message) => {
                let message = py_prefix(&message, 180).to_string();
                current.insert(name.clone(), json!({"branch": branch, "error": message}));
                errors.push(json!({"name": name, "stage": "latest_commit", "error": message}));
                continue;
            }
        };

        let old = previous.old_sha(&name)?;
        let head = latest["sha"].clone();
        let count =
            if json_truthy(&old) { compare_count(source, &identity.user, &name, &old, &head).await } else { None };
        let mut entry = Map::new();
        entry.insert("branch".into(), branch);
        entry.extend(latest.clone());
        current.insert(name.clone(), Value::Object(entry));
        if json_truthy(&old) && old != head {
            if let Some(count) = count {
                detected += count;
            }
            changes.push(Change { name, count, latest });
        }
    }

    let changed = previous.repositories() != Value::Object(current.clone());
    let (project_commits, monitor_commits) = if changed {
        (previous.project_commits + detected, previous.monitor_commits + 1)
    } else {
        (previous.project_commits, previous.monitor_commits)
    };
    Ok(Scan { current, changes, errors, detected, changed, project_commits, monitor_commits })
}

/// O `ECOSYSTEM-COMMIT-STATE.json` publicado.
pub fn render_state(scan: &Scan, scanned_at: &str) -> String {
    let state = json!({
        "schema": STATE_SCHEMA,
        "scanned_at": scanned_at,
        "scan_interval": "hourly",
        "repositories": scan.current,
        "metrics": {
            "tracked_commits": scan.tracked_commits(),
            "project_commits": scan.project_commits,
            "monitor_commits": scan.monitor_commits,
            "detected_project_commits_this_scan": scan.detected,
            "monitor_commit_this_scan": 1,
        },
        "summary": {
            "repositories_scanned": scan.current.len(),
            "repositories_changed": scan.changes.len(),
            "errors": scan.errors.len(),
        },
        "errors": scan.errors,
    });
    let mut text = serde_json::to_string_pretty(&state).expect("Value sempre serializa");
    text.push('\n');
    text
}

/// O `ECOSYSTEM-COMMIT-MONITOR.md`. Um SHA que não é texto quebraria o
/// Python aqui, depois de o estado já ter sido gravado.
pub fn render_report(scan: &Scan, scanned_at: &str) -> Result<String, Crash> {
    let mut lines: Vec<String> = [
        "# Ecosystem Commit Monitor",
        "",
        "> Snapshot horário do ecossistema público. O perfil acompanha o último commit de cada repositório e agrega mudanças; ele não espelha o histórico inteiro dos projetos.",
        "",
    ]
    .map(String::from)
    .to_vec();
    lines.push(format!("**Última varredura:** `{scanned_at}`  "));
    lines.push("**Intervalo configurado:** `1 hora`  ".into());
    lines.push(format!("**Repositórios acompanhados:** `{}`  ", scan.current.len()));
    lines.push(format!("**Repositórios com mudanças desde a última varredura:** `{}`  ", scan.changes.len()));
    lines.push(format!("**Falhas de consulta:** `{}`", scan.errors.len()));
    lines.extend(["", "## Contadores", ""].map(String::from));
    lines.push(format!("- **Commits rastreados pelo ecossistema:** `{}`", scan.tracked_commits()));
    lines.push(format!("- **Commits dos projetos:** `{}`", scan.project_commits));
    lines.push(format!("- **Commits do próprio monitor:** `{}`", scan.monitor_commits));
    lines.push(format!("- **Commits de projetos detectados nesta hora:** `{}`", scan.detected));
    lines.extend(
        [
            "",
            "> O contador acima é uma métrica própria do monitor. Ele não é o mesmo que **GitHub Contributions**. O contador do monitor cresce somente quando há mudança semântica e o snapshot é publicado; varreduras sem mudança são no-op.",
            "",
            "## Mudanças detectadas",
            "",
        ]
        .map(String::from),
    );
    if scan.changes.is_empty() {
        lines.push("- Nenhuma mudança desde a última varredura.".into());
    } else {
        let mut ordered: Vec<&Change> = scan.changes.iter().collect();
        ordered.sort_by_key(|change| change.name.to_lowercase());
        for change in ordered {
            let count = match change.count {
                None => "quantidade não determinada".to_string(),
                Some(count) => format!("{count} commit(s)"),
            };
            let Value::String(sha) = &change.latest["sha"] else {
                return Err(Crash {
                    kind: "TypeError",
                    message: format!("'{}' object is not subscriptable", py_type_name(&change.latest["sha"])),
                });
            };
            lines.push(format!(
                "- **{}** — {count} — [{}]({}) — {}",
                change.name,
                py_prefix(sha, 12),
                py_str(&change.latest["url"]),
                py_str(&change.latest["message"]),
            ));
        }
    }
    if !scan.errors.is_empty() {
        lines.extend(["", "## Erros de consulta", ""].map(String::from));
        for error in &scan.errors {
            lines.push(format!(
                "- **{}** — `{}` — {}",
                py_str(&error["name"]),
                py_str(&error["stage"]),
                py_str(&error["error"])
            ));
        }
    }
    lines.extend(ARCHITECTURE.iter().map(|line| line.to_string()));
    Ok(lines.join("\n"))
}

const ARCHITECTURE: [&str; 30] = [
    "",
    "## Arquitetura",
    "",
    "```text",
    "projetos individuais",
    "       │",
    "       │ latest SHA + comparação",
    "       ▼",
    "ecosystem_watch.py",
    "       │",
    "       ├── estado dos projetos",
    "       ├── commits dos projetos",
    "       ├── + 1 commit do monitor",
    "       └── contador acumulado",
    "       │",
    "       ▼",
    "snapshot agregado a cada hora",
    "```",
    "",
    "### Regras de estabilidade",
    "",
    "1. O perfil faz uma varredura programada por hora.",
    "2. Cada snapshot publicado acrescenta exatamente 1 ao contador de commits do monitor; varreduras sem mudança semântica não geram commit.",
    "3. As mudanças dos projetos são agregadas: um snapshot pode registrar quantos commits cada repositório recebeu desde a varredura anterior, sem copiar esses commits para o perfil.",
    "4. Retries e backoff protegem contra falhas transitórias da API.",
    "5. Repositórios novos do usuário são descobertos automaticamente; forks são ignorados.",
    "6. O contador próprio do ecossistema não tenta reproduzir a métrica oficial de GitHub Contributions.",
    "",
    "A varredura continua horária para detectar mudanças, mas o histórico só recebe commits quando há alteração semântica; o scheduler do GitHub pode atrasar a execução real.",
    "",
];

/// Hora da varredura no formato do Python (`%Y-%m-%dT%H:%M:%SZ`).
pub fn scanned_at(now: DateTime<Utc>) -> String {
    now.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// O que foi publicado numa varredura com mudança.
#[derive(Debug)]
pub struct Published {
    /// Texto do estado.
    pub state: String,
    /// Texto do relatório (ou o que derrubaria o Python ao renderizá-lo).
    pub report: Result<String, Crash>,
}

/// Grava estado e relatório na ordem do Python: o estado primeiro.
pub fn write(root: &Path, published: &Published) -> Result<(), std::io::Error> {
    let state = root.join(STATE_FILE);
    if let Some(parent) = state.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&state, &published.state)?;
    if let Ok(report) = &published.report {
        std::fs::write(root.join(REPORT_FILE), report)?;
    }
    Ok(())
}

/// Estado e relatório de uma varredura com mudança.
pub fn publish(scan: &Scan, now: DateTime<Utc>) -> Published {
    let at = scanned_at(now);
    Published { state: render_state(scan, &at), report: render_report(scan, &at) }
}

/// Caminho do estado numa raiz.
pub fn state_path(root: &Path) -> PathBuf {
    root.join(STATE_FILE)
}

/// As transições de uma varredura em relação ao estado anterior, no formato
/// do banco — só os repositórios cujo estado mudou (D-020). Devolve também os
/// que não cabem no schema (SHA fora do formato de 40 dígitos hexadecimais).
pub fn transitions(scan: &Scan, previous: &Previous) -> (Vec<store::activity::HeadObservation>, Vec<String>) {
    let before = previous.repositories();
    let mut observations = Vec::new();
    let mut skipped = Vec::new();
    for (name, entry) in &scan.current {
        if before.get(name) == Some(entry) {
            continue;
        }
        let count = scan.changes.iter().find(|change| &change.name == name).and_then(|c| c.count);
        match observation(name, entry, count) {
            Some(observation) => observations.push(observation),
            None => skipped.push(name.clone()),
        }
    }
    (observations, skipped)
}

/// Uma entrada de `repositories` do estado (`{branch, sha, …}`, `{branch,
/// empty}` ou `{branch, error}`) como observação do banco; `None` quando o
/// SHA não tem o formato que o schema exige.
pub fn observation(name: &str, entry: &Value, count: Option<i64>) -> Option<store::activity::HeadObservation> {
    use store::activity::{HeadObservation, HeadState};
    let text = |key: &str| entry.get(key).and_then(Value::as_str).map(str::to_string);
    let state = if let Some(message) = text("error") {
        HeadState::Error { stage: "latest_commit".into(), message }
    } else if entry.get("empty").is_some_and(json_truthy) {
        HeadState::Empty
    } else {
        let sha = text("sha")
            .filter(|sha| sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))?;
        HeadState::Commit {
            sha,
            committed_at: text("date")
                .and_then(|date| ecosystem_domain::timestamps::parse_github_timestamp(&date))
                .map(|date| date.to_rfc3339()),
            message: text("message").unwrap_or_default(),
            url: text("url"),
        }
    };
    Some(HeadObservation {
        name: name.to_string(),
        branch: py_str(&entry["branch"]),
        state,
        commits_since_previous: count,
    })
}
