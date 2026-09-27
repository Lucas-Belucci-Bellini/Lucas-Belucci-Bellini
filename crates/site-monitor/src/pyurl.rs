//! `urllib.parse` do CPython 3.12.14 — só o que a verificação de sites usa.
//!
//! Port função por função de `Lib/urllib/parse.py`, com os mesmos nomes. O
//! `final_url` que o relatório grava é o texto que estas funções produzem (e
//! não a URL normalizada de um cliente HTTP): `https://example.org` continua
//! sem a barra final, `/ção` num `Location` vira `/%E7%E3o`, `..` além da raiz
//! é ignorado. Cada caso está no fixture `tests/fixtures/parity/site_monitor.json`.
//!
//! **Fora do port, de propósito:** a checagem NFKC de `_checknetloc`. Ela só
//! age em netloc não-ASCII, e aqui todo netloc chega em latin-1 (o `Location`
//! é decodificado como ISO-8859-1 pelo `http.client`, e um host fora do
//! latin-1 é recusado antes da rede — ver [`crate::pyrequest`]). Nenhum
//! caractere de U+0080 a U+00FF normaliza para `/?#@:` — conferido no
//! CPython 3.12.14 ao escrever este módulo.

use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::LazyLock;

use ecosystem_domain::text::py_strip;
use regex::Regex;

/// `ValueError` levantado pelas funções do `urllib.parse`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ValueError(pub String);

/// `SplitResult`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SplitResult {
    /// Esquema, em minúsculas.
    pub scheme: String,
    /// Autoridade (`user@host:port`).
    pub netloc: String,
    /// Caminho.
    pub path: String,
    /// Consulta, sem o `?`.
    pub query: String,
    /// Fragmento, sem o `#`.
    pub fragment: String,
}

/// `ParseResult`: o `SplitResult` com os parâmetros (`;p`) separados.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParseResult {
    /// Esquema, em minúsculas.
    pub scheme: String,
    /// Autoridade.
    pub netloc: String,
    /// Caminho.
    pub path: String,
    /// Parâmetros do último segmento.
    pub params: String,
    /// Consulta.
    pub query: String,
    /// Fragmento.
    pub fragment: String,
}

const USES_RELATIVE: &[&str] = &[
    "", "ftp", "http", "gopher", "nntp", "imap", "wais", "file", "https", "shttp", "mms", "prospero", "rtsp", "rtsps",
    "rtspu", "sftp", "svn", "svn+ssh", "ws", "wss",
];
const USES_NETLOC: &[&str] = &[
    "",
    "ftp",
    "http",
    "gopher",
    "nntp",
    "telnet",
    "imap",
    "wais",
    "file",
    "mms",
    "https",
    "shttp",
    "snews",
    "prospero",
    "rtsp",
    "rtsps",
    "rtspu",
    "rsync",
    "svn",
    "svn+ssh",
    "sftp",
    "nfs",
    "git",
    "git+ssh",
    "ws",
    "wss",
    "itms-services",
];
const USES_PARAMS: &[&str] = &[
    "", "ftp", "hdl", "prospero", "http", "imap", "https", "shttp", "rtsp", "rtsps", "rtspu", "sip", "sips", "mms",
    "sftp", "tel",
];

fn is_scheme_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')
}

/// `_WHATWG_C0_CONTROL_OR_SPACE`.
fn is_c0_or_space(c: char) -> bool {
    c <= ' '
}

/// `urlsplit(url, scheme, allow_fragments)`.
pub fn urlsplit(url: &str, scheme: &str, allow_fragments: bool) -> Result<SplitResult, ValueError> {
    let mut url = url.trim_start_matches(is_c0_or_space).to_string();
    let mut scheme = scheme.trim_matches(is_c0_or_space).to_string();
    for unsafe_byte in ['\t', '\r', '\n'] {
        url = url.replace(unsafe_byte, "");
        scheme = scheme.replace(unsafe_byte, "");
    }
    let mut result = SplitResult::default();
    if let Some(i) = url.find(':') {
        let first = url.chars().next().expect("há ':' na posição i");
        if i > 0 && first.is_ascii_alphabetic() && url[..i].chars().all(is_scheme_char) {
            scheme = url[..i].to_ascii_lowercase();
            url = url[i + 1..].to_string();
        }
    }
    if url.starts_with("//") {
        let (netloc, rest) = splitnetloc(&url, 2);
        if netloc.contains('[') != netloc.contains(']') {
            return Err(ValueError("Invalid IPv6 URL".into()));
        }
        if netloc.contains('[') && netloc.contains(']') {
            check_bracketed_netloc(&netloc)?;
        }
        result.netloc = netloc;
        url = rest;
    }
    if allow_fragments && let Some((before, fragment)) = url.split_once('#') {
        result.fragment = fragment.to_string();
        url = before.to_string();
    }
    if let Some((before, query)) = url.split_once('?') {
        result.query = query.to_string();
        url = before.to_string();
    }
    result.scheme = scheme;
    result.path = url;
    Ok(result)
}

/// `_splitnetloc(url, start)`.
fn splitnetloc(url: &str, start: usize) -> (String, String) {
    let delim =
        ['/', '?', '#'].iter().filter_map(|c| url[start..].find(*c).map(|i| i + start)).min().unwrap_or(url.len());
    (url[start..delim].to_string(), url[delim..].to_string())
}

/// `_check_bracketed_netloc` + `_check_bracketed_host`.
fn check_bracketed_netloc(netloc: &str) -> Result<(), ValueError> {
    let invalid = || ValueError("Invalid IPv6 URL".into());
    let host_and_port = netloc.rsplit_once('@').map_or(netloc, |(_, after)| after);
    let hostname = match host_and_port.split_once('[') {
        Some((before, bracketed)) => {
            if !before.is_empty() {
                return Err(invalid());
            }
            let (hostname, port) = bracketed.split_once(']').unwrap_or((bracketed, ""));
            if !port.is_empty() && !port.starts_with(':') {
                return Err(invalid());
            }
            hostname
        }
        None => host_and_port.split_once(':').map_or(host_and_port, |(host, _)| host),
    };
    static IPV_FUTURE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\Av[a-fA-F0-9]+\..+\z").expect("expressão válida"));
    if hostname.starts_with('v') {
        return if IPV_FUTURE.is_match(hostname) {
            Ok(())
        } else {
            Err(ValueError("IPvFuture address is invalid".into()))
        };
    }
    // ipaddress.ip_address(): IPv4 primeiro (e IPv4 entre colchetes é erro),
    // depois IPv6, com zona opcional (`fe80::1%eth0`).
    if hostname.parse::<Ipv4Addr>().is_ok() {
        return Err(ValueError("An IPv4 address cannot be in brackets".into()));
    }
    let address = match hostname.split_once('%') {
        Some((_, scope)) if scope.is_empty() || scope.contains('%') => return Err(invalid()),
        Some((address, _)) => address,
        None => hostname,
    };
    address
        .parse::<Ipv6Addr>()
        .map(|_| ())
        .map_err(|_| ValueError(format!("'{hostname}' does not appear to be an IPv4 or IPv6 address")))
}

/// `urlparse(url, scheme, allow_fragments)`.
pub fn urlparse(url: &str, scheme: &str, allow_fragments: bool) -> Result<ParseResult, ValueError> {
    let split = urlsplit(url, scheme, allow_fragments)?;
    let (path, params) = if USES_PARAMS.contains(&split.scheme.as_str()) && split.path.contains(';') {
        splitparams(&split.path)
    } else {
        (split.path, String::new())
    };
    Ok(ParseResult {
        scheme: split.scheme,
        netloc: split.netloc,
        path,
        params,
        query: split.query,
        fragment: split.fragment,
    })
}

/// `_splitparams(url)`.
fn splitparams(url: &str) -> (String, String) {
    let i = match url.rfind('/') {
        Some(slash) => match url[slash..].find(';') {
            Some(offset) => slash + offset,
            None => return (url.to_string(), String::new()),
        },
        None => url.find(';').expect("chamado só com ';' presente"),
    };
    (url[..i].to_string(), url[i + 1..].to_string())
}

/// `urlunsplit((scheme, netloc, path, query, fragment))` — a versão do 3.12.14,
/// que preserva `//` no caminho quando não há netloc.
pub fn urlunsplit(scheme: &str, netloc: &str, path: &str, query: &str, fragment: &str) -> String {
    let mut url = path.to_string();
    if !netloc.is_empty() {
        if !url.is_empty() && !url.starts_with('/') {
            url.insert(0, '/');
        }
        url = format!("//{netloc}{url}");
    } else if url.starts_with("//")
        || (!scheme.is_empty() && USES_NETLOC.contains(&scheme) && (url.is_empty() || url.starts_with('/')))
    {
        // Os dois `elif` do Python, que fazem a mesma coisa.
        url = format!("//{url}");
    }
    if !scheme.is_empty() {
        url = format!("{scheme}:{url}");
    }
    if !query.is_empty() {
        url = format!("{url}?{query}");
    }
    if !fragment.is_empty() {
        url = format!("{url}#{fragment}");
    }
    url
}

/// `urlunparse(parts)`.
pub fn urlunparse(parts: &ParseResult) -> String {
    let path = if parts.params.is_empty() { parts.path.clone() } else { format!("{};{}", parts.path, parts.params) };
    urlunsplit(&parts.scheme, &parts.netloc, &path, &parts.query, &parts.fragment)
}

/// `urljoin(base, url)`.
pub fn urljoin(base: &str, url: &str) -> Result<String, ValueError> {
    if base.is_empty() {
        return Ok(url.to_string());
    }
    if url.is_empty() {
        return Ok(base.to_string());
    }
    let b = urlparse(base, "", true)?;
    let mut u = urlparse(url, &b.scheme, true)?;

    if u.scheme != b.scheme || !USES_RELATIVE.contains(&u.scheme.as_str()) {
        return Ok(url.to_string());
    }
    if USES_NETLOC.contains(&u.scheme.as_str()) {
        if !u.netloc.is_empty() {
            return Ok(urlunparse(&u));
        }
        u.netloc = b.netloc.clone();
    }
    if u.path.is_empty() && u.params.is_empty() {
        u.path = b.path;
        u.params = b.params;
        if u.query.is_empty() {
            u.query = b.query;
        }
        return Ok(urlunparse(&u));
    }

    let mut base_parts: Vec<&str> = b.path.split('/').collect();
    if base_parts.last() != Some(&"") {
        base_parts.pop();
    }
    let segments: Vec<&str> = if u.path.starts_with('/') {
        u.path.split('/').collect()
    } else {
        let mut all: Vec<&str> = base_parts;
        all.extend(u.path.split('/'));
        // segments[1:-1] = filter(None, segments[1:-1])
        if all.len() > 2 {
            let last = all.len() - 1;
            let middle: Vec<&str> = all[1..last].iter().copied().filter(|s| !s.is_empty()).collect();
            let mut rebuilt = vec![all[0]];
            rebuilt.extend(middle);
            rebuilt.push(all[last]);
            all = rebuilt;
        }
        all
    };

    let mut resolved: Vec<&str> = Vec::new();
    for segment in &segments {
        match *segment {
            ".." => {
                resolved.pop();
            }
            "." => {}
            other => resolved.push(other),
        }
    }
    if matches!(segments.last(), Some(&".") | Some(&"..")) {
        resolved.push("");
    }
    let joined = resolved.join("/");
    u.path = if joined.is_empty() { "/".into() } else { joined };
    Ok(urlunparse(&u))
}

/// `quote(string, safe=string.punctuation, encoding="iso-8859-1")` — a forma
/// que `HTTPRedirectHandler.http_error_302` usa. Só espaço, controles, DEL e
/// bytes ≥ 0x80 são codificados. Caractere acima de U+00FF não cabe em
/// latin-1: `UnicodeEncodeError` (um `ValueError`).
pub fn quote_redirect(value: &str) -> Result<String, ValueError> {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        let byte = u8::try_from(u32::from(c))
            .map_err(|_| ValueError(format!("'latin-1' codec can't encode character {c:?}")))?;
        if byte.is_ascii_graphic() {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    Ok(out)
}

/// `unwrap(url)`: tira espaços (no sentido do Python) e o invólucro `<URL:…>`.
pub fn unwrap(url: &str) -> String {
    let mut url = py_strip(url).to_string();
    if url.starts_with('<') && url.ends_with('>') && url.len() >= 2 {
        url = py_strip(&url[1..url.len() - 1]).to_string();
    }
    if let Some(rest) = url.strip_prefix("URL:") {
        url = py_strip(rest).to_string();
    }
    url
}

/// `_splittag(url)`: separa no **último** `#`. Fragmento vazio vira `None`
/// só no `full_url` (quem chama decide).
pub fn splittag(url: &str) -> (String, Option<String>) {
    match url.rsplit_once('#') {
        Some((path, tag)) => (path.to_string(), Some(tag.to_string())),
        None => (url.to_string(), None),
    }
}

/// `_splittype(url)`: `([^/:]+):(.*)` no começo; esquema em minúsculas.
pub fn splittype(url: &str) -> (Option<String>, String) {
    match url.find(':') {
        Some(i) if i > 0 && !url[..i].contains('/') => (Some(url[..i].to_lowercase()), url[i + 1..].to_string()),
        _ => (None, url.to_string()),
    }
}

/// `_splithost(url)`: `//([^/#?]*)(.*)`; caminho sem `/` inicial ganha um.
pub fn splithost(url: &str) -> (Option<String>, String) {
    let Some(rest) = url.strip_prefix("//") else {
        return (None, url.to_string());
    };
    let end = rest.find(['/', '#', '?']).unwrap_or(rest.len());
    let (host, path) = rest.split_at(end);
    let path = if !path.is_empty() && !path.starts_with('/') { format!("/{path}") } else { path.to_string() };
    (Some(host.to_string()), path)
}

/// `unquote(string)` (UTF-8, `errors="replace"`): cada trecho ASCII tem os
/// `%XX` decodificados; trechos não-ASCII passam intactos.
pub fn unquote(value: &str) -> String {
    if !value.contains('%') {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len());
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        if !run.is_empty() {
            out.push_str(&String::from_utf8_lossy(&unquote_to_bytes(run)));
            run.clear();
        }
    };
    for c in value.chars() {
        if c.is_ascii() {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// `unquote_to_bytes`: `%XX` com dois dígitos hexadecimais vira o byte; o
/// resto fica literal.
fn unquote_to_bytes(ascii: &str) -> Vec<u8> {
    let bytes = ascii.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let (Some(high), Some(low)) = (hex(bytes.get(i + 1)), hex(bytes.get(i + 2)))
        {
            out.push(high * 16 + low);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

fn hex(byte: Option<&u8>) -> Option<u8> {
    char::from(*byte?).to_digit(16).map(|d| d as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urljoin_basico() {
        let base = "https://example.org/dir/page?x=1#top";
        assert_eq!("https://example.org/dir/rel", urljoin(base, "rel").unwrap());
        assert_eq!("https://example.org/up", urljoin(base, "../up").unwrap());
        assert_eq!("https://example.org/dir/page?only=query", urljoin(base, "?only=query").unwrap());
        assert_eq!(base, urljoin(base, "").unwrap());
    }

    #[test]
    fn quote_so_codifica_o_que_nao_e_pontuacao() {
        assert_eq!("/a%20b%E7?x=%41", quote_redirect("/a b\u{e7}?x=%41").unwrap());
        assert!(quote_redirect("日").is_err());
    }

    #[test]
    fn unquote_por_trecho_ascii() {
        assert_eq!("example.org", unquote("ex%61mple.org"));
        assert_eq!("\u{fffd}é\u{fffd}", unquote("%C3é%A7"));
        assert_eq!("100%", unquote("100%"));
    }

    #[test]
    fn ipv6_entre_colchetes() {
        assert!(urlsplit("http://[::1]:8080/x", "", true).is_ok());
        assert!(urlsplit("http://[::1/", "", true).is_err());
        assert!(urlsplit("http://[1.2.3.4]/", "", true).is_err());
        assert!(urlsplit("http://[fe80::1%eth0]/", "", true).is_ok());
    }
}
