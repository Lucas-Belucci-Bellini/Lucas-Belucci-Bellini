//! Tipos e regras puras do catálogo do ecossistema do perfil.
//!
//! Este crate não faz I/O: nada de rede, banco ou disco (mesma disciplina do
//! `src/engine/` do Project Vanguard). É o que permite testar toda regra de
//! negócio sem GitHub e sem PostgreSQL — e comparar, caso a caso, com o
//! Python que ele substitui.
//!
//! # Paridade com o Python
//!
//! Cada função com sufixo `py_v1` reproduz **byte a byte** a função Python de
//! mesmo papel, inclusive os defeitos conhecidos (D-006 em
//! `docs/DECISION-LOG.md`): correção entra depois, como versão nova. A prova
//! é `tests/parity.rs`, que roda os casos de `tests/fixtures/parity/domain.json`
//! — o mesmo arquivo que `tests/test_parity_domain.py` gera a partir do Python.
//!
//! As armadilhas de semântica do Python estão isoladas em [`text`],
//! [`url`], [`timestamps`] e [`priority::py_round`].

pub mod classify;
pub mod discovery;
pub mod lifecycle;
pub mod monitor;
pub mod presentation;
pub mod priority;
pub mod pyjson;
pub mod repo;
pub mod slug;
pub mod taxonomy;
pub mod text;
pub mod timestamps;
pub mod url;

pub use classify::classify_py_v1;
pub use discovery::{WebsiteSource, discover_project_website, normalize_site_overrides};
pub use lifecycle::status_py_v1;
pub use presentation::{CheckStatus, Presentation, WebsiteCheck, catalog_entry, describe, resolve_presentation};
pub use priority::{featured_score_py_v1, marketing_priority};
pub use repo::RepoFacts;
pub use slug::slug;
pub use taxonomy::canonical_category;
pub use url::looks_like_http_url;
