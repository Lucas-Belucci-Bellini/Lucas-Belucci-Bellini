//! O inventário do GitHub como o `update_profile.py` o lê.
//!
//! - Com token (`PROFILE_GITHUB_TOKEN`): `/user/repos` com
//!   `affiliation=owner,collaborator,organization_member` — privados inclusive.
//! - Sem token: só os públicos do dono (`/users/{dono}/repos?type=owner`).
//! - Linguagens: `/repos/{full_name}/languages`; qualquer falha vira mapa
//!   vazio, sem derrubar a execução.
//!
//! Uma tentativa por chamada ([`Retry::Never`]), como o `api_get` do Python.
//! Os arquivos locais (`--input-repos`, `--languages-dir`) têm o mesmo formato
//! do Python: um array de repositórios e um JSON `{linguagem: bytes}` por
//! repositório, com `/` trocado por `__` no nome.

use std::path::Path;

use ecosystem_domain::pyjson::py_int;
use ecosystem_domain::repo::RepoFacts;
use github_client::{ApiError, Client, Retry};
use serde_json::{Map, Value};

use crate::{CatalogError, OWNER};

/// `User-Agent` do `update_profile.py`.
pub const USER_AGENT: &str = "profile-readme-refresh";

/// `fetch_repositories(token)`: o JSON cru de cada repositório, na ordem da API.
pub async fn fetch_repositories(client: &Client) -> Result<Vec<Value>, ApiError> {
    if client.authenticated() {
        client
            .get_pages(
                |page| {
                    format!("/user/repos?affiliation=owner,collaborator,organization_member&per_page=100&page={page}")
                },
                Retry::Never,
            )
            .await
    } else {
        client.get_pages(|page| format!("/users/{OWNER}/repos?type=owner&per_page=100&page={page}"), Retry::Never).await
    }
}

/// `fetch_languages(full_name)`: bytes por linguagem, na ordem da API; falha
/// (HTTP, rede, JSON, valor que não é inteiro) vira mapa vazio.
pub async fn fetch_languages(client: &Client, full_name: &str) -> Vec<(String, i64)> {
    match client.get_json(&format!("/repos/{full_name}/languages"), Retry::Never).await {
        Ok(Value::Object(map)) => language_map(&map).unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// `{str(k): int(v) for k, v in data.items()}` — um valor ruim anula o mapa inteiro.
fn language_map(map: &Map<String, Value>) -> Option<Vec<(String, i64)>> {
    map.iter().map(|(language, bytes)| py_int(bytes).ok().map(|bytes| (language.clone(), bytes))).collect()
}

/// `load_local_repositories(path)`: o arquivo precisa ser um array JSON.
pub fn load_local_repositories(path: &Path) -> Result<Vec<Value>, CatalogError> {
    let text =
        std::fs::read_to_string(path).map_err(|error| CatalogError::Input(format!("{}: {error}", path.display())))?;
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Array(repos)) => Ok(repos),
        Ok(_) => Err(CatalogError::Input("repository input must be a JSON array".into())),
        Err(error) => Err(CatalogError::Input(error.to_string())),
    }
}

/// `load_local_languages(directory, repos)`: arquivo ausente ou inválido é mapa vazio.
pub fn load_local_languages(directory: &Path, full_name: &str) -> Vec<(String, i64)> {
    let path = directory.join(format!("{}.json", full_name.replace('/', "__")));
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value.as_object().and_then(language_map))
        .unwrap_or_default()
}

/// Os fatos que as regras usam, de cada repositório cru.
pub fn facts(repos: &[Value]) -> Result<Vec<RepoFacts>, CatalogError> {
    repos
        .iter()
        .map(|repo| {
            serde_json::from_value::<RepoFacts>(repo.clone())
                .map_err(|error| CatalogError::Input(format!("repositório com campo de tipo inesperado: {error}")))
        })
        .collect()
}
