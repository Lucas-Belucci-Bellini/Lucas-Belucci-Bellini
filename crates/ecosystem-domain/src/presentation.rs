//! Como um projeto aparece na vitrine — `resolve_presentation()` e
//! `catalog_entry()` de `scripts/project_catalog.py`.
//!
//! A regra central do perfil mora aqui (e na view `ecosystem.public_projects`):
//!
//! ```text
//! projeto COM site verificado  →  CTA primário = site,   secundário = código
//! projeto SEM site             →  CTA primário = código, sem secundário
//! ```
//!
//! Repositório privado nunca anuncia site, mesmo com homepage verificada.

use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::priority::marketing_priority;
use crate::repo::RepoFacts;
use crate::slug::slug;
use crate::taxonomy::canonical_category;
use crate::text::{py_split_join, py_strip};

/// Resultado de uma verificação HTTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Respondeu (2xx, ou redirecionamento cujo destino respondeu).
    Verified,
    /// 404, 5xx, timeout, DNS, conexão recusada.
    Unreachable,
    /// URL malformada; recusada antes da rede.
    Invalid,
}

impl CheckStatus {
    /// Texto gravado no catálogo (`website_status`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unreachable => "unreachable",
            Self::Invalid => "invalid",
        }
    }
}

/// Uma verificação de site, como o catálogo a usa (sem o carimbo de hora).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WebsiteCheck {
    /// URL verificada.
    pub url: String,
    /// Resultado.
    pub status: CheckStatus,
    /// Código HTTP; 0 quando não houve resposta.
    pub http_status: i64,
    /// Destino após redirecionamentos.
    pub final_url: String,
}

/// O que a vitrine precisa saber para renderizar um projeto.
#[derive(Debug, Clone, PartialEq)]
pub struct Presentation {
    /// `owner/nome`.
    pub repository: String,
    /// Nome curto.
    pub name: String,
    /// Descrição pública.
    pub description: String,
    /// Categoria canônica.
    pub category: &'static str,
    /// Rótulo exibido no README.
    pub category_label: String,
    /// Link publicado — só quando verificado e público.
    pub website: Option<String>,
    /// URL descoberta, no ar ou não (nunca para privado).
    pub website_declared: Option<String>,
    /// `verified` | `unreachable` | `invalid` | `none`.
    pub website_status: &'static str,
    /// Código HTTP da verificação; 0 sem verificação.
    pub website_http_status: i64,
    /// `github_homepage` | `manifest` | `none`.
    pub website_source: String,
    /// Destino após redirecionamento.
    pub website_final_url: String,
    /// URL do repositório.
    pub github: String,
    /// `website` | `github`.
    pub primary_cta: &'static str,
    /// `github` quando há site.
    pub secondary_cta: Option<&'static str>,
    /// Status exibido.
    pub status: String,
    /// Visibilidade privada.
    pub private: bool,
    /// Está na curadoria.
    pub featured: bool,
    /// Prioridade de exibição, 0–100.
    pub marketing_priority: i64,
}

impl Presentation {
    /// Identidade estável para rota.
    pub fn slug(&self) -> String {
        slug(&self.name)
    }

    /// Há site verificado para publicar.
    pub fn has_live_website(&self) -> bool {
        self.website.is_some() && self.website_status == "verified"
    }
}

/// `describe()` de `scripts/update_profile.py`: descrição pública em uma linha.
pub fn describe(description: Option<&str>) -> String {
    let text = match description {
        Some(value) if !value.is_empty() => value,
        _ => "Descrição pública não informada.",
    };
    py_split_join(text)
}

/// O `ProjectPresentationResolver`: decide como o projeto aparece.
#[allow(clippy::too_many_arguments)]
pub fn resolve_presentation(
    repo: &RepoFacts,
    check: Option<&WebsiteCheck>,
    website: Option<&str>,
    website_source: &str,
    category: &str,
    status: &str,
    description: &str,
    featured_order: Option<i64>,
    featured_priority: Option<i64>,
) -> Presentation {
    let private = repo.private;
    let website = if private { None } else { website };
    let live =
        !private && check.is_some_and(|c| c.status == CheckStatus::Verified) && website.is_some_and(|w| !w.is_empty());

    let stripped = py_strip(description);
    let description_ok = stripped.chars().count() >= 25 && stripped != "—";

    Presentation {
        repository: repo.full_name.clone(),
        name: repo.name.clone(),
        description: description.to_string(),
        category: canonical_category(category),
        category_label: category.to_string(),
        website: if live { website.map(str::to_string) } else { None },
        website_declared: website.map(str::to_string),
        website_status: check.map_or("none", |c| c.status.as_str()),
        website_http_status: check.map_or(0, |c| c.http_status),
        website_source: website_source.to_string(),
        website_final_url: check.map_or_else(String::new, |c| c.final_url.clone()),
        github: format!("https://github.com/{}", repo.full_name),
        primary_cta: if live { "website" } else { "github" },
        secondary_cta: live.then_some("github"),
        status: status.to_string(),
        private,
        featured: featured_order.is_some(),
        marketing_priority: marketing_priority(
            repo,
            check.map(|c| c.status),
            featured_order,
            featured_priority,
            description_ok,
        ),
    }
}

/// Uma linha de `docs/project-catalog.json`, com as chaves na mesma ordem do Python.
pub fn catalog_entry(p: &Presentation) -> Value {
    let mut entry = Map::new();
    entry.insert("repository".into(), json!(p.repository));
    entry.insert("name".into(), json!(p.name));
    entry.insert("slug".into(), json!(p.slug()));
    entry.insert("description".into(), json!(p.description));
    entry.insert("category".into(), json!(p.category));
    entry.insert("category_label".into(), json!(p.category_label));
    entry.insert("status".into(), json!(p.status));
    entry.insert("github".into(), json!(p.github));
    entry.insert("github_visible".into(), json!(true));
    entry.insert("github_role".into(), json!(if p.website.is_some() { "source" } else { "primary" }));
    entry.insert("primary_cta".into(), json!(p.primary_cta));
    entry.insert("secondary_cta".into(), json!(p.secondary_cta));
    entry.insert("featured".into(), json!(p.featured));
    entry.insert("marketing_priority".into(), json!(p.marketing_priority));
    entry.insert("private".into(), json!(p.private));
    if let Some(declared) = p.website_declared.as_deref().filter(|w| !w.is_empty()) {
        entry.insert("website_declared".into(), json!(declared));
        entry.insert("website_status".into(), json!(p.website_status));
        entry.insert("website_http_status".into(), json!(p.website_http_status));
        entry.insert("website_source".into(), json!(p.website_source));
    }
    if let Some(website) = &p.website {
        entry.insert("website".into(), json!(website));
        if !p.website_final_url.is_empty() && &p.website_final_url != website {
            entry.insert("website_final_url".into(), json!(p.website_final_url));
        }
    }
    Value::Object(entry)
}
