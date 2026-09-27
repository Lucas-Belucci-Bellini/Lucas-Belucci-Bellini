//! `HTTPRedirectHandler.http_error_302` do CPython 3.12.14: como um
//! `Location` vira o próximo pedido.
//!
//! ```text
//! Location → urlparse → esquema permitido? → caminho vazio vira "/"
//!          → urlunparse → quote(latin-1, safe=pontuação) → urljoin(atual, …)
//!          → Request(novo)
//! ```
//!
//! A contagem de laço (no máximo 4 visitas à mesma URL e 10 URLs distintas)
//! fica em [`RedirectGuard`], com a mesma ordem de checagem do Python.

use std::collections::HashMap;

use crate::pyrequest::{PreflightError, PyRequest};
use crate::pyurl::{quote_redirect, urljoin, urlparse, urlunparse};

/// Esquemas para os quais o `urllib` aceita redirecionar.
const ALLOWED_SCHEMES: [&str; 4] = ["http", "https", "ftp", ""];

/// O que um 301/302/303/307/308 produz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectStep {
    /// Seguir para `request`; `visited_key` é a chave da contagem de laço.
    Follow {
        /// URL depois do `urljoin` (a chave de `redirect_dict`).
        visited_key: String,
        /// O próximo pedido.
        request: PyRequest,
    },
    /// Esquema proibido (`mailto:`, `javascript:`, `file:`…): `HTTPError`
    /// com o código do redirect e a URL crua do cabeçalho.
    NotAllowed(String),
    /// Sem `Location` nem `URI`: o redirect vira `HTTPError` com o próprio código.
    NoLocation,
    /// `ValueError` ao montar o próximo pedido.
    Failed(PreflightError),
}

/// Aplica a regra do `http_error_302` a um redirecionamento recebido em
/// `current_full_url`. `location`/`uri` já decodificados como latin-1.
pub fn redirect_step(current_full_url: &str, location: Option<&str>, uri: Option<&str>) -> RedirectStep {
    let Some(newurl) = location.or(uri) else {
        return RedirectStep::NoLocation;
    };
    let value_error = |message: String| RedirectStep::Failed(PreflightError::Caught { kind: "ValueError", message });
    let mut parts = match urlparse(newurl, "", true) {
        Ok(parts) => parts,
        Err(error) => return value_error(error.0),
    };
    if !ALLOWED_SCHEMES.contains(&parts.scheme.as_str()) {
        return RedirectStep::NotAllowed(newurl.to_string());
    }
    if parts.path.is_empty() && !parts.netloc.is_empty() {
        parts.path = "/".into();
    }
    let quoted = match quote_redirect(&urlunparse(&parts)) {
        Ok(quoted) => quoted,
        Err(error) => {
            return RedirectStep::Failed(PreflightError::Caught { kind: "UnicodeEncodeError", message: error.0 });
        }
    };
    let joined = match urljoin(current_full_url, &quoted) {
        Ok(joined) => joined,
        Err(error) => return value_error(error.0),
    };
    // redirect_request(): "Be conciliant with URIs containing a space".
    match PyRequest::new(&joined.replace(' ', "%20")) {
        Ok(request) => RedirectStep::Follow { visited_key: joined, request },
        Err(error) => RedirectStep::Failed(error),
    }
}

/// `redirect_dict` do `HTTPRedirectHandler`.
#[derive(Debug, Default)]
pub struct RedirectGuard {
    visited: Option<HashMap<String, u32>>,
}

impl RedirectGuard {
    /// `max_repeats`.
    pub const MAX_REPEATS: u32 = 4;
    /// `max_redirections`.
    pub const MAX_REDIRECTIONS: usize = 10;

    /// Registra o próximo salto. `false` quando o Python levantaria o
    /// `HTTPError` de laço (com o código do redirect e a URL **atual**).
    ///
    /// O primeiro redirect nunca é checado — o dicionário ainda não existe.
    pub fn admit(&mut self, visited_key: &str) -> bool {
        match &mut self.visited {
            None => {
                self.visited = Some(HashMap::from([(visited_key.to_string(), 1)]));
                true
            }
            Some(visited) => {
                if visited.get(visited_key).copied().unwrap_or(0) >= Self::MAX_REPEATS
                    || visited.len() >= Self::MAX_REDIRECTIONS
                {
                    return false;
                }
                *visited.entry(visited_key.to_string()).or_insert(0) += 1;
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laco_na_mesma_url_para_na_quinta() {
        let mut guard = RedirectGuard::default();
        let admitted = (0..10).take_while(|_| guard.admit("https://x.example.org/loop")).count();
        assert_eq!(4, admitted, "4 seguidas; a 5ª levanta HTTPError");
    }

    #[test]
    fn dez_urls_distintas_no_maximo() {
        let mut guard = RedirectGuard::default();
        let admitted = (0..20).take_while(|i| guard.admit(&format!("https://x.example.org/{i}"))).count();
        assert_eq!(10, admitted);
    }

    #[test]
    fn esquema_proibido_devolve_a_url_crua() {
        assert_eq!(
            RedirectStep::NotAllowed("mailto:a@b".into()),
            redirect_step("https://example.org/", Some("mailto:a@b"), None)
        );
    }
}
