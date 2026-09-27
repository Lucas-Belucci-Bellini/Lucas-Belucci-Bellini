//! A verificação de um site — `check_website()` e `check_websites()` de
//! `scripts/project_catalog.py` — sobre um transporte HTTP de verdade.
//!
//! Regras reproduzidas (versão `py-check@1`):
//!
//! * URL que não passa em [`looks_like_http_url`] é `invalid`, `final_url` vazio,
//!   sem rede.
//! * `retries + 1` tentativas, espera de `0,5 × n` s entre elas; um 404 também
//!   é tentado de novo.
//! * 2xx (depois de seguir redirects) é `verified`; o `final_url` é o
//!   `full_url` do último pedido, no texto do `urllib`.
//! * Qualquer outra resposta vira o `HTTPError` do `urllib`, e conta como
//!   `verified` se o código estiver em [`LIVE_STATUSES`] — o que acontece com
//!   **laço de redirect, 302 sem `Location` e redirect para `mailto:`** (achado
//!   A21 da auditoria; reproduzido de propósito, D-006).
//! * Falha de rede, de TLS ou de validação é `unreachable`, HTTP 0 e
//!   `final_url` = a URL **original**, mesmo que a falha tenha sido num salto.
//!
//! Além do que o Python relata, cada checagem registra tentativas, número de
//! redirects, tempo e o tipo do erro — o que vai para `ecosystem.website_checks`.

use std::collections::HashMap;
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use ecosystem_domain::looks_like_http_url;
use reqwest::header::{self, HeaderMap, HeaderValue};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::pyrequest::{PreflightError, PyRequest, Target, preflight};
use crate::redirect::{RedirectGuard, RedirectStep, redirect_step};

/// O mesmo `User-Agent` do Python.
pub const USER_AGENT: &str = "profile-readme-website-check/2.0";

/// Códigos que o Python considera "no ar" quando chegam como `HTTPError`.
pub const LIVE_STATUSES: [u16; 5] = [200, 301, 302, 307, 308];

const REDIRECT_CODES: [u16; 5] = [301, 302, 303, 307, 308];

/// Resultado da verificação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Respondeu.
    Verified,
    /// Não respondeu (ou respondeu com erro).
    Unreachable,
    /// URL malformada; recusada antes da rede.
    Invalid,
}

impl Status {
    /// Texto do relatório.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unreachable => "unreachable",
            Self::Invalid => "invalid",
        }
    }
}

/// Classificação do erro — os valores da `CHECK` de `website_checks.error_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Tempo esgotado ao conectar ou ler.
    Timeout,
    /// Nome não resolvido.
    Dns,
    /// Conexão recusada ou interrompida.
    Connect,
    /// Certificado ou handshake.
    Tls,
    /// O servidor respondeu com um código que não conta como no ar.
    HttpStatus,
    /// URL recusada antes da rede.
    InvalidUrl,
    /// O resto.
    Other,
}

impl ErrorKind {
    /// Texto gravado no banco.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Dns => "dns",
            Self::Connect => "connect",
            Self::Tls => "tls",
            Self::HttpStatus => "http_status",
            Self::InvalidUrl => "invalid_url",
            Self::Other => "other",
        }
    }
}

/// Uma verificação: os campos do `WebsiteCheck` do Python e os extras do banco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebsiteCheck {
    /// URL verificada, como veio.
    pub url: String,
    /// Resultado.
    pub status: Status,
    /// Código HTTP; 0 sem resposta.
    pub http_status: u16,
    /// Destino, no texto do `urllib`.
    pub final_url: String,
    /// Início da checagem, `YYYY-MM-DDTHH:MM:SS+00:00`.
    pub checked_at: String,
    /// Tentativas feitas (0 para `invalid`).
    pub attempts: u32,
    /// Redirects seguidos na última tentativa.
    pub redirect_count: u32,
    /// Tempo da última tentativa até a resposta final.
    pub response_time_ms: Option<u64>,
    /// Tipo do erro; `None` quando `verified`.
    pub error_kind: Option<ErrorKind>,
    /// Mensagem do erro.
    pub error_message: Option<String>,
    /// Exceção que teria derrubado o `check_websites.py` inteiro (achado A22).
    pub python_crash: Option<&'static str>,
}

/// Parâmetros da verificação (os mesmos do `check_websites.py`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// Timeout de cada conexão e de cada leitura (o `timeout` do socket).
    pub timeout: Duration,
    /// Tentativas extras.
    pub retries: u32,
    /// Verificações simultâneas.
    pub max_workers: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self { timeout: Duration::from_secs(15), retries: 1, max_workers: 6 }
    }
}

/// Falha ao montar o cliente HTTP.
#[derive(Debug, thiserror::Error)]
#[error("não foi possível montar o cliente HTTP: {0}")]
pub struct BuildError(#[source] reqwest::Error);

/// Verificador de sites. Barato de clonar.
#[derive(Debug, Clone)]
pub struct Checker {
    client: reqwest::Client,
    options: Options,
}

enum Attempt {
    Success { status: u16, final_url: String },
    HttpError { status: u16, url: String },
    Failure { kind: ErrorKind, message: String, crash: Option<&'static str> },
}

struct Response {
    status: u16,
    location: Option<String>,
    uri: Option<String>,
    body_error: Option<(ErrorKind, String)>,
}

impl Checker {
    /// Cliente HTTP/1.1 sem redirecionamento automático, com os cabeçalhos
    /// do `urllib` (`Accept-Encoding: identity`, `Connection: close`) e a
    /// confiança de certificados do sistema.
    pub fn new(options: Options) -> Result<Self, BuildError> {
        static PROVIDER: Once = Once::new();
        PROVIDER.call_once(|| {
            // Já instalado por outro crate também serve.
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT_ENCODING, HeaderValue::from_static("identity"));
        headers.insert(header::CONNECTION, HeaderValue::from_static("close"));
        let client = reqwest::Client::builder()
            .tls_backend_rustls()
            .http1_only()
            .redirect(reqwest::redirect::Policy::none())
            .referer(false)
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .connect_timeout(options.timeout)
            .read_timeout(options.timeout)
            .pool_max_idle_per_host(0)
            .build()
            .map_err(BuildError)?;
        Ok(Self { client, options })
    }

    /// `check_website(url)`.
    pub async fn check_website(&self, url: &str) -> WebsiteCheck {
        let checked_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S+00:00").to_string();
        let mut check = WebsiteCheck {
            url: url.to_string(),
            status: Status::Invalid,
            http_status: 0,
            final_url: String::new(),
            checked_at,
            attempts: 0,
            redirect_count: 0,
            response_time_ms: None,
            error_kind: Some(ErrorKind::InvalidUrl),
            error_message: Some("não é uma URL http(s) válida".into()),
            python_crash: None,
        };
        if !looks_like_http_url(url) {
            return check;
        }

        for attempt in 0..=self.options.retries {
            check.attempts = attempt + 1;
            let started = Instant::now();
            let mut redirects = 0;
            let outcome = self.attempt(url, &mut redirects).await;
            check.redirect_count = redirects;
            check.response_time_ms = Some(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX));
            match outcome {
                Attempt::Success { status, final_url } => {
                    return verified(check, status, final_url);
                }
                Attempt::HttpError { status, url: error_url } => {
                    let final_url = if error_url.is_empty() { url.to_string() } else { error_url };
                    if LIVE_STATUSES.contains(&status) {
                        return verified(check, status, final_url);
                    }
                    check.status = Status::Unreachable;
                    check.http_status = status;
                    check.final_url = final_url;
                    check.error_kind = Some(ErrorKind::HttpStatus);
                    check.error_message = Some(format!("HTTP {status}"));
                    check.python_crash = None;
                }
                Attempt::Failure { kind, message, crash } => {
                    check.status = Status::Unreachable;
                    check.http_status = 0;
                    check.final_url = url.to_string();
                    check.error_kind = Some(kind);
                    check.error_message = Some(message);
                    check.python_crash = crash;
                }
            }
            if attempt < self.options.retries {
                tokio::time::sleep(Duration::from_millis(500 * u64::from(attempt + 1))).await;
            }
        }
        check
    }

    /// `check_websites(urls)`: cada URL não vazia uma vez só, no máximo
    /// `max_workers` ao mesmo tempo.
    pub async fn check_websites<I, S>(&self, urls: I) -> HashMap<String, WebsiteCheck>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut unique: Vec<String> = Vec::new();
        for url in urls.into_iter().map(Into::into) {
            if !url.is_empty() && !unique.contains(&url) {
                unique.push(url);
            }
        }
        if unique.is_empty() {
            return HashMap::new();
        }
        let workers = self.options.max_workers.clamp(1, unique.len());
        let semaphore = Arc::new(Semaphore::new(workers));
        let mut tasks = JoinSet::new();
        for url in unique {
            let (checker, semaphore) = (self.clone(), Arc::clone(&semaphore));
            tasks.spawn(async move {
                let _permit = semaphore.acquire_owned().await.expect("semáforo nunca fecha");
                checker.check_website(&url).await
            });
        }
        let mut results = HashMap::new();
        while let Some(joined) = tasks.join_next().await {
            let check = joined.expect("uma verificação não entra em pânico");
            results.insert(check.url.clone(), check);
        }
        results
    }

    /// Uma tentativa: segue redirects como o `urllib` até uma resposta final.
    async fn attempt(&self, url: &str, redirects: &mut u32) -> Attempt {
        let mut request = match PyRequest::new(url) {
            Ok(request) => request,
            Err(error) => return failure_from(error),
        };
        let mut guard = RedirectGuard::default();
        loop {
            if request.scheme == "ftp" {
                return Attempt::Failure {
                    kind: ErrorKind::Other,
                    message: format!("redirecionado para {} — FTP não é verificado", request.full_url),
                    crash: None,
                };
            }
            let target = match preflight(&request) {
                Ok(target) => target,
                Err(error) => return failure_from(error),
            };
            let (following, response) = match self.fetch(&target).await {
                Ok(response) => (REDIRECT_CODES.contains(&response.status), response),
                Err((kind, message)) => return Attempt::Failure { kind, message, crash: None },
            };
            let status = response.status;
            if (200..300).contains(&status) {
                return Attempt::Success { status, final_url: request.full_url };
            }
            if !following {
                return Attempt::HttpError { status, url: request.full_url };
            }
            match redirect_step(&request.full_url, response.location.as_deref(), response.uri.as_deref()) {
                RedirectStep::NoLocation => return Attempt::HttpError { status, url: request.full_url },
                RedirectStep::NotAllowed(raw) => return Attempt::HttpError { status, url: raw },
                RedirectStep::Failed(error) => return failure_from(error),
                RedirectStep::Follow { visited_key, request: next } => {
                    if !guard.admit(&visited_key) {
                        return Attempt::HttpError { status, url: request.full_url };
                    }
                    // O Python lê o corpo do redirect antes de seguir.
                    if let Some((kind, message)) = response.body_error {
                        return Attempt::Failure { kind, message, crash: None };
                    }
                    *redirects += 1;
                    request = next;
                }
            }
        }
    }

    /// Um GET, sem seguir redirect. O corpo só é lido quando a resposta é um
    /// redirect (como o `fp.read()` do `http_error_302`).
    async fn fetch(&self, target: &Target) -> Result<Response, (ErrorKind, String)> {
        let host = if target.connect_host.contains(':') {
            format!("[{}]", target.connect_host)
        } else {
            target.connect_host.clone()
        };
        let raw = format!("{}://{}:{}{}", target.scheme, host, target.port, target.request_target);
        let url = reqwest::Url::parse(&raw).map_err(|error| (ErrorKind::InvalidUrl, format!("{error}: {raw}")))?;
        let host_header: Vec<u8> =
            target.host_header.chars().map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?')).collect();
        let host_value =
            HeaderValue::from_bytes(&host_header).map_err(|error| (ErrorKind::InvalidUrl, error.to_string()))?;
        let response = self.client.get(url).header(header::HOST, host_value).send().await.map_err(|e| classify(&e))?;
        let status = response.status().as_u16();
        let latin1 = |name: &str| {
            response
                .headers()
                .get(name)
                .map(|value| value.as_bytes().iter().map(|b| char::from(*b)).collect::<String>())
        };
        let (location, uri) = (latin1("location"), latin1("uri"));
        let body_error = if REDIRECT_CODES.contains(&status) {
            response.bytes().await.err().map(|error| classify(&error))
        } else {
            None
        };
        Ok(Response { status, location, uri, body_error })
    }
}

fn verified(mut check: WebsiteCheck, status: u16, final_url: String) -> WebsiteCheck {
    check.status = Status::Verified;
    check.http_status = status;
    check.final_url = final_url;
    check.error_kind = None;
    check.error_message = None;
    check.python_crash = None;
    check
}

fn failure_from(error: PreflightError) -> Attempt {
    let crash = match &error {
        PreflightError::Crash { kind, .. } => Some(*kind),
        PreflightError::Caught { .. } => None,
    };
    // `UnicodeError` é o codec idna do getaddrinfo: o nome não resolve.
    let kind = if error.kind() == "UnicodeError" { ErrorKind::Dns } else { ErrorKind::InvalidUrl };
    Attempt::Failure { kind, message: error.to_string(), crash }
}

/// Classifica um erro do `reqwest` pela cadeia de causas: primeiro pelo tipo
/// (`rustls::Error`, `io::ErrorKind`), depois pelo texto do resolvedor.
fn classify(error: &reqwest::Error) -> (ErrorKind, String) {
    let mut message = error.to_string();
    let (mut tls, mut timed_out, mut refused) = (false, error.is_timeout(), false);
    let mut inspect = |cause: &(dyn std::error::Error + 'static)| {
        tls |= cause.downcast_ref::<rustls::Error>().is_some();
        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
            timed_out |= io.kind() == std::io::ErrorKind::TimedOut;
            refused |= matches!(
                io.kind(),
                std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
            );
        }
    };
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        // O `source()` de um io::Error pula o erro que ele embrulha; o
        // rustls::Error do handshake fica dois io::Error para dentro.
        let mut inner: Option<&(dyn std::error::Error + 'static)> = Some(cause);
        while let Some(current) = inner {
            inspect(current);
            inner = current
                .downcast_ref::<std::io::Error>()
                .and_then(|io| io.get_ref())
                .map(|wrapped| wrapped as &(dyn std::error::Error + 'static));
        }
        source = cause.source();
    }
    let lower = message.to_lowercase();
    let kind = if timed_out {
        ErrorKind::Timeout
    } else if tls {
        ErrorKind::Tls
    } else if lower.contains("dns error") || lower.contains("failed to lookup address") {
        ErrorKind::Dns
    } else if refused || error.is_connect() {
        ErrorKind::Connect
    } else {
        ErrorKind::Other
    };
    (kind, message)
}
