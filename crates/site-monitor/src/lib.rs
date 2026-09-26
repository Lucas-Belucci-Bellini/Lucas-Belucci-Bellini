//! Monitor de sites do ecossistema: recebe URLs, devolve resultados.
//!
//! Não conhece README, catálogo nem GitHub (docs/architecture/COMPONENTS.md,
//! Website Monitor) — quem decide *quais* URLs verificar é o chamador.
//!
//! # Paridade com o Python
//!
//! O relatório do `scripts/check_websites.py` depende de detalhes do `urllib`
//! do CPython 3.12.14 que um cliente HTTP moderno faria diferente: o texto do
//! `final_url` (sem normalizar), a resolução de `Location` relativo, o limite
//! de laço (4 visitas à mesma URL, 10 URLs distintas) e o que conta como "no
//! ar" quando o redirecionamento falha. Esses pedaços são portados do código
//! da biblioteca padrão ([`pyurl`], [`pyrequest`], [`redirect`]) e conferidos
//! contra `tests/fixtures/parity/site_monitor.json`; o transporte é o
//! `reqwest` (HTTP/1.1, rustls, certificados do sistema).
//!
//! As diferenças que sobram estão em docs/migration/PYTHON-TO-RUST.md
//! ("Armadilhas de paridade — monitor de sites").

pub mod check;
pub mod pyrequest;
pub mod pyurl;
pub mod redirect;

pub use check::{Checker, ErrorKind, LIVE_STATUSES, Options, Status, USER_AGENT, WebsiteCheck};
