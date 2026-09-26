//! O README do perfil e o `assets/profile-snapshot.svg` — o `update_profile.py`
//! da leitura dos dados em diante.
//!
//! ```text
//! inventário + linguagens + apresentações + manifestos ─▶ 13 blocos ─▶ README.md
//!                                                     └─▶ profile-snapshot.svg
//! ```
//!
//! Crate puro, como o `ecosystem-domain`: não lê disco, não fala com a rede.
//! Quem chama (`profile-core render readme`) monta as entradas pelo mesmo
//! caminho do `catalog build` e grava as saídas. Cada bloco é a função
//! Python de mesmo papel, com os mesmos bytes — inclusive os defeitos já
//! registrados (D-006): a pilha de linguagens dos privados no PROJECT-MAP
//! (A20) continua lá até a correção entrar como versão nova.
//!
//! A prova é `tests/golden.rs`, sobre a mesma raiz sintética do teste golden
//! do Python (`tests/fixtures/profile`), e o teste de ponta a ponta
//! `tests/e2e/github_parity.py`, contra um GitHub simulado.
//!
//! Onde o Python quebraria com traceback (manifesto com formato errado,
//! marcador ausente no README), aqui sai um [`RenderError`] — e o chamador
//! sai com o mesmo código 1.

pub mod badges;
pub mod blocks;
pub mod languages;
pub mod pytext;
pub mod snapshot;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use ecosystem_domain::presentation::Presentation;
use ecosystem_domain::repo::RepoFacts;
use serde_json::{Map, Value};

pub use languages::{LanguageMap, LanguageRow, format_bytes, language_rows};

/// Dono do perfil (`OWNER`).
pub const OWNER: &str = "Lucas-Belucci-Bellini";

/// Onde o Python quebraria.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RenderError {
    /// `replace_block`: o README não tem o par de marcadores.
    #[error("ValueError: README marker not found: {0}")]
    MarkerNotFound(String),
    /// Manifesto com formato que o Python não percorre, ou datas que ele não compara.
    #[error("{kind}: {detail}")]
    PythonCrash {
        /// Exceção que o Python levantaria.
        kind: &'static str,
        /// O que estava errado.
        detail: String,
    },
}

/// O que os blocos leem.
#[derive(Debug, Clone, Copy)]
pub struct Inputs<'a> {
    /// Inventário depois das exclusões, na ordem da API (repetidos inclusive).
    pub repos: &'a [RepoFacts],
    /// Linguagens por `full_name`.
    pub languages: &'a LanguageMap,
    /// `build_presentations()`: uma por `full_name`.
    pub presentations: &'a [Presentation],
    /// Tamanho do conjunto de exclusões editoriais.
    pub excluded: usize,
    /// Relógio da execução.
    pub now: DateTime<Utc>,
    /// `docs/README_FEATURED.json`.
    pub featured: &'a Map<String, Value>,
    /// `docs/README_STACK.json`.
    pub stack: &'a Map<String, Value>,
    /// Carimbo exibido (`source_timestamp(...).strftime(...)`, ver [`snapshot::generated_at`]).
    pub generated_at: &'a str,
}

/// As entradas com os índices que os blocos consultam.
#[derive(Debug)]
pub struct Profile<'a> {
    /// Entradas.
    pub inputs: Inputs<'a>,
    /// `language_rows(repos, languages, public_only=True)`.
    pub rows: Vec<LanguageRow>,
    by_repository: HashMap<&'a str, &'a Presentation>,
}

impl<'a> Profile<'a> {
    /// Indexa as apresentações e calcula a matriz de linguagens.
    pub fn new(inputs: Inputs<'a>) -> Self {
        let by_repository = inputs.presentations.iter().map(|p| (p.repository.as_str(), p)).collect();
        Self { rows: language_rows(inputs.repos, inputs.languages), inputs, by_repository }
    }

    /// `presentations[repo["full_name"]]`: toda linha do inventário tem uma.
    pub fn presentation(&self, repo: &RepoFacts) -> &'a Presentation {
        self.by_repository
            .get(repo.full_name.as_str())
            .copied()
            .unwrap_or_else(|| panic!("sem apresentação para {}: entradas inconsistentes", repo.full_name))
    }

    /// Apresentações dos públicos, na ordem do inventário (repetidos inclusive).
    pub fn public(&self) -> Vec<&'a Presentation> {
        self.inputs.repos.iter().filter(|repo| !repo.private).map(|repo| self.presentation(repo)).collect()
    }

    /// `live_site_map(presentations)`: quantos projetos têm site no ar.
    pub fn verified_sites(&self) -> usize {
        self.inputs.presentations.iter().filter(|p| p.has_live_website()).count()
    }

    /// O README: os 13 blocos, na ordem do Python, sobre o texto atual.
    pub fn readme(&self, template: &str) -> Result<String, RenderError> {
        let mut text = template.to_string();
        for marker in blocks::MARKERS {
            let body = blocks::render(self, marker)?;
            text = blocks::replace_block(&text, marker, &body)?;
        }
        Ok(text)
    }

    /// `assets/profile-snapshot.svg`.
    pub fn snapshot_svg(&self) -> String {
        snapshot::svg(self)
    }
}
