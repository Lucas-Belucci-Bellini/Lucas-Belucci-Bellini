//! Cliente da API do GitHub com a semântica dos coletores Python.
//!
//! Hoje há um cliente por script, cada um com a sua política:
//!
//! | script | tentativas | o que tenta de novo |
//! |:---|:---|:---|
//! | `update_profile.api_get` | 1 | nada: erro propaga |
//! | `ecosystem_watch.api` | até 4 | 429, 5xx, 403 com `X-RateLimit-Remaining: 0` (esperando `Retry-After` só com dígitos, senão 2ⁿ s, no máximo 30); falha de conexão e timeout (2ⁿ s) |
//! | `update_contribution_timeline.query_period` | 1 | nada (POST GraphQL) |
//!
//! O cliente é um só; a política é argumento ([`Retry`]). Os erros sabem o
//! texto que o Python gravaria (`str(exc)`, pelo `Display` de [`ApiError`]):
//! o monitor guarda esse texto no estado, e ele entra na comparação
//! semântica entre varreduras.

use std::sync::Once;
use std::time::Duration;

use reqwest::header::{self, HeaderMap, HeaderValue};
use serde_json::Value;

/// Base da API REST pública.
pub const DEFAULT_API_URL: &str = "https://api.github.com";
/// Endpoint GraphQL público.
pub const DEFAULT_GRAPHQL_URL: &str = "https://api.github.com/graphql";
/// `urlopen(..., timeout=30)` dos três scripts.
pub const TIMEOUT: Duration = Duration::from_secs(30);
/// `MAX_RETRIES` do `ecosystem_watch.py`.
pub const WATCH_ATTEMPTS: u32 = 4;
/// Teto de espera entre tentativas (`min(delay, 30)`).
pub const MAX_DELAY_UNITS: u64 = 30;

/// Onde está a API. O Actions define `GITHUB_API_URL` e `GITHUB_GRAPHQL_URL`
/// com os valores públicos; fora dele, as variáveis apontam para um GitHub
/// simulado (os testes de ponta a ponta) — o mesmo contrato dos scripts Python.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    /// Base REST, sem barra final.
    pub api_url: String,
    /// URL do GraphQL.
    pub graphql_url: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self { api_url: DEFAULT_API_URL.to_string(), graphql_url: DEFAULT_GRAPHQL_URL.to_string() }
    }
}

impl Endpoints {
    /// Lê `GITHUB_API_URL` (sem a barra final, como o `rstrip("/")` do Python)
    /// e `GITHUB_GRAPHQL_URL`; o que faltar fica no padrão público.
    pub fn from_env() -> Self {
        let read = |name: &str| std::env::var(name).ok();
        Self {
            api_url: read("GITHUB_API_URL")
                .map_or_else(|| DEFAULT_API_URL.to_string(), |value| value.trim_end_matches('/').to_string()),
            graphql_url: read("GITHUB_GRAPHQL_URL").unwrap_or_else(|| DEFAULT_GRAPHQL_URL.to_string()),
        }
    }
}

/// Política de novas tentativas, por chamador.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retry {
    /// Uma tentativa (`update_profile.py`, timeline).
    Never,
    /// `ecosystem_watch.api()`.
    Watch,
}

/// Configuração do cliente.
#[derive(Debug, Clone)]
pub struct Settings {
    /// `User-Agent` do script que este cliente substitui.
    pub user_agent: &'static str,
    /// Envia `X-GitHub-Api-Version: 2022-11-28` (só o monitor envia).
    pub api_version_header: bool,
    /// Endpoints.
    pub endpoints: Endpoints,
    /// Timeout de conexão e de cada leitura.
    pub timeout: Duration,
    /// Duração de "1 segundo" nas esperas entre tentativas. Os testes encurtam.
    pub delay_unit: Duration,
}

impl Settings {
    /// Configuração de produção para o `User-Agent` dado.
    pub fn new(user_agent: &'static str) -> Self {
        Self {
            user_agent,
            api_version_header: false,
            endpoints: Endpoints::from_env(),
            timeout: TIMEOUT,
            delay_unit: Duration::from_secs(1),
        }
    }
}

/// Falha ao montar o cliente HTTP.
#[derive(Debug, thiserror::Error)]
#[error("cliente HTTP: {0}")]
pub struct BuildError(#[from] reqwest::Error);

/// Um erro de chamada, com o `Display` igual ao `str(exc)` do Python.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ApiError {
    /// `urllib.error.HTTPError`: resposta com código fora de 2xx.
    #[error("HTTP Error {status}: {reason}")]
    Http {
        /// Código HTTP.
        status: u16,
        /// Frase de status (a canônica do código).
        reason: String,
        /// Cabeçalho `Retry-After`.
        retry_after: Option<String>,
        /// Cabeçalho `X-RateLimit-Remaining`.
        ratelimit_remaining: Option<String>,
    },
    /// `urllib.error.URLError`: falha antes da resposta (conexão, DNS, TLS,
    /// timeout de conexão). O texto é o `reason` que o Python mostraria.
    #[error("<urlopen error {0}>")]
    Url(String),
    /// `TimeoutError` cru: a resposta não chegou (ou parou) a tempo.
    #[error("timed out")]
    Timeout,
    /// Outra falha de transporte (conexão fechada sem resposta, status
    /// ilegível): no Python ela escapa sem nova tentativa.
    #[error("{0}")]
    Transport(String),
    /// Corpo que não é JSON.
    #[error("{0}")]
    Json(String),
}

impl ApiError {
    /// Código HTTP, quando houve resposta.
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Http { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Espera antes da próxima tentativa do `ecosystem_watch.api()`, em
    /// segundos, ou `None` quando o erro propaga na hora.
    fn watch_delay(&self, attempt: u32) -> Option<u64> {
        let backoff = 2_u64.pow(attempt);
        match self {
            Self::Http { status, retry_after, ratelimit_remaining, .. } => {
                let mut retryable = *status == 429 || *status >= 500;
                if *status == 403 {
                    retryable = ratelimit_remaining.as_deref() == Some("0");
                }
                if !retryable {
                    return None;
                }
                // `retry_after.isdigit()` — só dígitos ASCII aqui (o Python
                // também aceitaria dígitos de outros alfabetos).
                let delay = retry_after
                    .as_deref()
                    .filter(|value| !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(backoff);
                Some(delay.min(MAX_DELAY_UNITS))
            }
            Self::Url(_) | Self::Timeout => Some(backoff.min(MAX_DELAY_UNITS)),
            Self::Transport(_) | Self::Json(_) => None,
        }
    }
}

/// Cliente REST + GraphQL. Clonar é barato (o pool HTTP é compartilhado).
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    settings: Settings,
    token: Option<String>,
}

impl Client {
    /// Monta o cliente (sem token; ver [`Client::with_token`]).
    pub fn new(settings: Settings) -> Result<Self, BuildError> {
        static PROVIDER: Once = Once::new();
        PROVIDER.call_once(|| {
            // Já instalado por outro crate também serve.
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT, HeaderValue::from_static("application/vnd.github+json"));
        if settings.api_version_header {
            headers.insert("X-GitHub-Api-Version", HeaderValue::from_static("2022-11-28"));
        }
        let http = reqwest::Client::builder()
            .tls_backend_rustls()
            .http1_only()
            .user_agent(settings.user_agent)
            .default_headers(headers)
            .connect_timeout(settings.timeout)
            .read_timeout(settings.timeout)
            .build()?;
        Ok(Self { http, settings, token: None })
    }

    /// O mesmo cliente com outro token (`None` = anônimo).
    pub fn with_token(&self, token: Option<String>) -> Self {
        Self { token: token.filter(|t| !t.is_empty()), ..self.clone() }
    }

    /// Há token.
    pub fn authenticated(&self) -> bool {
        self.token.is_some()
    }

    /// Endpoints em uso.
    pub fn endpoints(&self) -> &Endpoints {
        &self.settings.endpoints
    }

    /// URL absoluta de um caminho REST (`/users/x/repos?...`).
    pub fn rest_url(&self, path: &str) -> String {
        format!("{}{path}", self.settings.endpoints.api_url)
    }

    /// GET de um caminho REST que devolve JSON, com a política dada.
    pub async fn get_json(&self, path: &str, retry: Retry) -> Result<Value, ApiError> {
        let url = self.rest_url(path);
        let attempts = match retry {
            Retry::Never => 1,
            Retry::Watch => WATCH_ATTEMPTS,
        };
        let mut attempt = 0;
        loop {
            match self.send(self.authorize(self.http.get(&url))).await {
                Ok(value) => return Ok(value),
                Err(error) => {
                    let delay = error.watch_delay(attempt).filter(|_| attempt + 1 < attempts);
                    let Some(delay) = delay else { return Err(error) };
                    tokio::time::sleep(self.settings.delay_unit * u32::try_from(delay).unwrap_or(u32::MAX)).await;
                    attempt += 1;
                }
            }
        }
    }

    /// Todas as páginas de uma listagem: `page=1, 2, …` até uma página vazia
    /// ou com menos de 100 itens — o laço dos dois coletores Python. Uma
    /// página que não é lista é erro (o Python seguiria com lixo).
    pub async fn get_pages(&self, path_for_page: impl Fn(u32) -> String, retry: Retry) -> Result<Vec<Value>, ApiError> {
        let mut items = Vec::new();
        let mut page = 1;
        loop {
            let batch = match self.get_json(&path_for_page(page), retry).await? {
                Value::Array(batch) => batch,
                other if !json_truthy(&other) => return Ok(items),
                _ => return Err(ApiError::Json("listagem do GitHub não é uma lista".into())),
            };
            if batch.is_empty() {
                return Ok(items);
            }
            let full = batch.len() >= 100;
            items.extend(batch);
            if !full {
                return Ok(items);
            }
            page += 1;
        }
    }

    /// POST GraphQL. Devolve o corpo inteiro (com `errors`, se houver): quem
    /// chama decide o que é erro, como no Python.
    pub async fn graphql(&self, query: &str, variables: Value) -> Result<Value, ApiError> {
        let body = serde_json::json!({ "query": query, "variables": variables }).to_string();
        let request = self
            .http
            .post(&self.settings.endpoints.graphql_url)
            .header(header::CONTENT_TYPE, "application/json")
            .body(body);
        self.send(self.authorize(request)).await
    }

    fn authorize(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    async fn send(&self, request: reqwest::RequestBuilder) -> Result<Value, ApiError> {
        let response = request.send().await.map_err(|error| transport_error(&error, false))?;
        let status = response.status();
        if !status.is_success() {
            let header =
                |name: &str| response.headers().get(name).and_then(|value| value.to_str().ok()).map(str::to_string);
            return Err(ApiError::Http {
                status: status.as_u16(),
                reason: status.canonical_reason().unwrap_or("").to_string(),
                retry_after: header("Retry-After"),
                ratelimit_remaining: header("X-RateLimit-Remaining"),
            });
        }
        let bytes = response.bytes().await.map_err(|error| transport_error(&error, true))?;
        serde_json::from_slice(&bytes).map_err(|error| ApiError::Json(error.to_string()))
    }
}

/// Verdade do Python para um valor JSON (`if not batch`).
fn json_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Traduz um erro do `reqwest` para o que o `urllib` levantaria.
///
/// Antes da resposta, o `urllib` embrulha a falha em `URLError` (conexão,
/// DNS, TLS e timeout de conexão); esperando ou lendo a resposta, o timeout
/// escapa cru (`TimeoutError: timed out`) e o resto também (conexão fechada
/// sem resposta, status ilegível).
fn transport_error(error: &reqwest::Error, reading_body: bool) -> ApiError {
    let chain = error_chain(error);
    let lower = chain.to_lowercase();
    if error.is_timeout() {
        return if error.is_connect() { ApiError::Url("timed out".into()) } else { ApiError::Timeout };
    }
    if error.is_connect() && !reading_body {
        let reason = if has_io_kind(error, std::io::ErrorKind::ConnectionRefused) {
            "[Errno 111] Connection refused".to_string()
        } else if lower.contains("dns error") || lower.contains("failed to lookup address") {
            "[Errno -2] Name or service not known".to_string()
        } else {
            chain
        };
        return ApiError::Url(reason);
    }
    if lower.contains("connection closed before message completed") {
        return ApiError::Transport("Remote end closed connection without response".into());
    }
    ApiError::Transport(chain)
}

fn error_chain(error: &reqwest::Error) -> String {
    let mut message = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

fn has_io_kind(error: &reqwest::Error, kind: std::io::ErrorKind) -> bool {
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        // O `source()` de um io::Error pula o erro que ele embrulha.
        let mut inner: Option<&(dyn std::error::Error + 'static)> = Some(cause);
        while let Some(current) = inner {
            if current.downcast_ref::<std::io::Error>().is_some_and(|io| io.kind() == kind) {
                return true;
            }
            inner = current
                .downcast_ref::<std::io::Error>()
                .and_then(|io| io.get_ref())
                .map(|wrapped| wrapped as &(dyn std::error::Error + 'static));
        }
        source = cause.source();
    }
    false
}

/// `urllib.parse.quote(value)` com `safe='/'` e UTF-8 — como o monitor monta
/// `/repos/{dono}/{quote(nome)}/commits?sha={quote(branch)}`.
pub fn py_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"_.-~/".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn http(status: u16, retry_after: Option<&str>, remaining: Option<&str>) -> ApiError {
        ApiError::Http {
            status,
            reason: String::new(),
            retry_after: retry_after.map(str::to_string),
            ratelimit_remaining: remaining.map(str::to_string),
        }
    }

    #[test]
    fn politica_do_monitor() {
        assert_eq!(None, http(404, None, None).watch_delay(0));
        assert_eq!(None, http(409, None, None).watch_delay(0));
        assert_eq!(Some(1), http(500, None, None).watch_delay(0));
        assert_eq!(Some(4), http(502, None, None).watch_delay(2));
        assert_eq!(Some(7), http(429, Some("7"), None).watch_delay(0));
        assert_eq!(Some(30), http(429, Some("120"), None).watch_delay(0), "teto de 30");
        assert_eq!(Some(2), http(429, Some("1.5"), None).watch_delay(1), "Retry-After não inteiro: 2ⁿ");
        assert_eq!(Some(0), http(503, Some("0"), None).watch_delay(3));
        assert_eq!(None, http(403, None, Some("12")).watch_delay(0), "403 comum não tenta de novo");
        assert_eq!(Some(1), http(403, None, Some("0")).watch_delay(0), "403 de rate limit tenta");
        assert_eq!(Some(8), ApiError::Timeout.watch_delay(3));
        assert_eq!(Some(2), ApiError::Url("x".into()).watch_delay(1));
        assert_eq!(None, ApiError::Transport("x".into()).watch_delay(0));
        assert_eq!(None, ApiError::Json("x".into()).watch_delay(0));
    }

    #[test]
    fn texto_do_python() {
        let error =
            ApiError::Http { status: 409, reason: "Conflict".into(), retry_after: None, ratelimit_remaining: None };
        assert_eq!("HTTP Error 409: Conflict", error.to_string());
        assert_eq!(
            "<urlopen error [Errno 111] Connection refused>",
            ApiError::Url("[Errno 111] Connection refused".into()).to_string()
        );
        assert_eq!("timed out", ApiError::Timeout.to_string());
    }

    #[test]
    fn quote_como_o_urllib() {
        assert_eq!("feature/x", py_quote("feature/x"));
        assert_eq!("a%20b%23c%2Bd", py_quote("a b#c+d"));
        assert_eq!("%C3%A7%C3%A3o~_.-", py_quote("ção~_.-"));
    }
}
