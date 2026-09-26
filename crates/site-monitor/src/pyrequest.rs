//! O que o `urllib.request` e o `http.client` do CPython 3.12.14 fazem com uma
//! URL **antes** de abrir o socket: `Request.full_url`, `_splithost`,
//! `HTTPConnection._get_hostport`, `_validate_host`, `putrequest` e
//! `putheader("Host", …)`.
//!
//! Isso decide três coisas do relatório: o `final_url` quando não há
//! redirecionamento (`https://example.org` fica sem barra), quais URLs falham
//! sem tocar a rede (caminho não-ASCII, host fora do latin-1) e quais **derrubam
//! o `check_websites.py` inteiro** — `http.client.InvalidURL` não é
//! `ValueError` e escapa do `except` de `check_website` (porta não numérica,
//! senha na URL, caractere de controle). Nesses casos o Rust não reproduz a
//! queda: devolve [`PreflightError::Crash`] e a checagem vira "fora do ar",
//! com a divergência registrada (docs/migration/PYTHON-TO-RUST.md).

use std::sync::LazyLock;

use regex::Regex;

use crate::pyurl::{splithost, splittag, splittype, unquote, unwrap};

/// Um `urllib.request.Request`, só os campos que a verificação usa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyRequest {
    /// `full_url`: o texto que vira `final_url` no relatório.
    pub full_url: String,
    /// `type`: esquema em minúsculas.
    pub scheme: String,
    /// `host` (com `user@` e `:porta`, se houver), já sem `%XX`.
    pub host: Option<String>,
    /// `selector`: caminho + consulta (+ `#` interno, se houver mais de um).
    pub selector: String,
}

/// Falha antes da rede, com o nome da exceção do Python.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PreflightError {
    /// Exceção que o `check_website` do Python captura: vira "unreachable".
    #[error("{kind}: {message}")]
    Caught {
        /// Classe da exceção (`URLError`, `ValueError`, `UnicodeEncodeError`…).
        kind: &'static str,
        /// Mensagem.
        message: String,
    },
    /// Exceção que escapa do `check_website` e derruba o `check_websites.py`.
    #[error("{kind} (derruba o Python): {message}")]
    Crash {
        /// Classe da exceção (`InvalidURL`).
        kind: &'static str,
        /// Mensagem.
        message: String,
    },
}

impl PreflightError {
    fn caught(kind: &'static str, message: impl Into<String>) -> Self {
        Self::Caught { kind, message: message.into() }
    }

    fn crash(message: impl Into<String>) -> Self {
        Self::Crash { kind: "InvalidURL", message: message.into() }
    }

    /// Nome da exceção do Python.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Caught { kind, .. } | Self::Crash { kind, .. } => kind,
        }
    }
}

impl PyRequest {
    /// `Request(url)`: `unwrap`, `_splittag`, `_splittype`, `_splithost`,
    /// `unquote(host)`.
    pub fn new(url: &str) -> Result<Self, PreflightError> {
        let unwrapped = unwrap(url);
        let (without_tag, fragment) = splittag(&unwrapped);
        let full_url = match fragment {
            Some(tag) if !tag.is_empty() => format!("{without_tag}#{tag}"),
            _ => without_tag.clone(),
        };
        let (scheme, rest) = splittype(&without_tag);
        let Some(scheme) = scheme else {
            return Err(PreflightError::caught("ValueError", format!("unknown url type: {full_url:?}")));
        };
        let (host, selector) = splithost(&rest);
        let host = host.filter(|h| !h.is_empty()).map(|h| unquote(&h));
        Ok(Self { full_url, scheme, host, selector })
    }
}

/// Para onde o pedido vai, do jeito que o `http.client` montaria.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// `full_url` do pedido.
    pub full_url: String,
    /// `http` ou `https`.
    pub scheme: String,
    /// Host da conexão (sem colchetes de IPv6, sem porta).
    pub connect_host: String,
    /// Porta da conexão.
    pub port: i64,
    /// Alvo da linha de pedido (`/caminho?consulta`).
    pub request_target: String,
    /// Valor do cabeçalho `Host` (o `req.host` cru, com porta explícita).
    pub host_header: String,
}

static DISALLOWED_URL_PCHAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\x00-\x20\x7f]").expect("expressão válida"));

/// Tudo o que `urlopen` faz com o pedido antes do socket.
pub fn preflight(request: &PyRequest) -> Result<Target, PreflightError> {
    if request.scheme != "http" && request.scheme != "https" {
        return Err(PreflightError::caught("URLError", format!("unknown url type: {}", request.scheme)));
    }
    let Some(host) = request.host.clone() else {
        return Err(PreflightError::caught("URLError", "no host given"));
    };
    let default_port = if request.scheme == "https" { 443 } else { 80 };
    let (connect_host, port) = get_hostport(&host, default_port)?;
    if DISALLOWED_URL_PCHAR.is_match(&connect_host) {
        return Err(PreflightError::crash(format!("URL can't contain control characters. {connect_host:?}")));
    }
    let request_target = if request.selector.is_empty() { "/".to_string() } else { request.selector.clone() };
    if DISALLOWED_URL_PCHAR.is_match(&request_target) {
        return Err(PreflightError::crash(format!("URL can't contain control characters. {request_target:?}")));
    }
    if !request_target.is_ascii() {
        return Err(PreflightError::caught("UnicodeEncodeError", "'ascii' codec can't encode the request line"));
    }
    if host.chars().any(|c| u32::from(c) > 0xFF) {
        return Err(PreflightError::caught("UnicodeEncodeError", "'latin-1' codec can't encode the Host header"));
    }
    if !connect_host.is_ascii() && idna::domain_to_ascii(&connect_host).is_err() {
        return Err(PreflightError::caught("UnicodeError", "encoding with 'idna' codec failed"));
    }
    Ok(Target {
        full_url: request.full_url.clone(),
        scheme: request.scheme.clone(),
        connect_host,
        port,
        request_target,
        host_header: host,
    })
}

/// `HTTPConnection._get_hostport(host, None)`.
fn get_hostport(host: &str, default_port: i64) -> Result<(String, i64), PreflightError> {
    let colon = host.rfind(':');
    let bracket = host.rfind(']');
    let (mut hostname, port) = match colon {
        Some(i) if bracket.is_none_or(|j| i > j) => {
            let digits = &host[i + 1..];
            let port = match py_int(digits) {
                Some(port) => port,
                None if digits.is_empty() => default_port,
                None => return Err(PreflightError::crash(format!("nonnumeric port: '{digits}'"))),
            };
            (host[..i].to_string(), port)
        }
        _ => (host.to_string(), default_port),
    };
    if hostname.len() >= 2 && hostname.starts_with('[') && hostname.ends_with(']') {
        hostname = hostname[1..hostname.len() - 1].to_string();
    }
    Ok((hostname, port))
}

/// `int(text)` do Python para dígitos ASCII: espaços nas pontas, sinal e `_`
/// entre dígitos são aceitos. **Divergência conhecida:** o Python também
/// aceita dígitos de outros sistemas numéricos (`٨٠`); aqui eles contam como
/// porta não numérica.
fn py_int(text: &str) -> Option<i64> {
    let trimmed = ecosystem_domain::text::py_strip(text);
    let (negative, digits) = match trimmed.as_bytes().first() {
        Some(b'-') => (true, &trimmed[1..]),
        Some(b'+') => (false, &trimmed[1..]),
        _ => (false, trimmed),
    };
    if digits.is_empty() || digits.starts_with('_') || digits.ends_with('_') || digits.contains("__") {
        return None;
    }
    if !digits.chars().all(|c| c.is_ascii_digit() || c == '_') {
        return None;
    }
    let value: i64 = digits.replace('_', "").parse().ok()?;
    Some(if negative { -value } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_url_preserva_o_texto() {
        assert_eq!("https://example.org", PyRequest::new("https://example.org").unwrap().full_url);
        assert_eq!("https://example.org/", PyRequest::new("https://example.org/#").unwrap().full_url);
        assert_eq!("https://example.org", PyRequest::new(" https://example.org\n").unwrap().full_url);
    }

    #[test]
    fn porta_como_o_int_do_python() {
        assert_eq!(Some(443), py_int("+443"));
        assert_eq!(Some(443), py_int("4_43"));
        assert_eq!(None, py_int("4__43"));
        assert_eq!(None, py_int("abc"));
    }
}
