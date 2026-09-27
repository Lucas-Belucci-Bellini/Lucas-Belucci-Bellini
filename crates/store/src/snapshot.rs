//! O que o gerador do perfil lê do banco (`render … --from-db`, Fase 5).
//!
//! Só o que é **coletado**: o inventário (repositórios que não sumiram, na
//! ordem da última listagem do GitHub), os mapas de linguagem (na ordem da
//! API) e a checagem mais recente de cada URL. O que é **editorial** —
//! exclusões, curadoria, arsenal, sites manuais — continua vindo dos
//! manifestos em `docs/`, que são a fonte (D-032): o banco é espelho deles.
//!
//! Os carimbos saem no formato do GitHub (`2026-09-20T10:00:00Z`), que é o
//! que o gerador lê de um inventário vindo da API.

use std::collections::HashMap;

use crate::{Database, StoreError};

/// Um repositório como a API o descreveria (o recorte que o gerador usa).
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct StoredRepo {
    /// `id` interno (junta com as linguagens).
    pub id: i64,
    /// Nome curto.
    pub name: String,
    /// `owner/nome`.
    pub full_name: String,
    /// Descrição.
    pub description: Option<String>,
    /// `public` | `private` | `internal`.
    pub visibility: String,
    /// É fork.
    pub is_fork: bool,
    /// Está arquivado.
    pub is_archived: bool,
    /// `homepage`, como veio.
    pub homepage: Option<String>,
    /// Último push, `YYYY-MM-DDTHH:MM:SSZ`.
    pub pushed_at: Option<String>,
    /// Última atualização, `YYYY-MM-DDTHH:MM:SSZ`.
    pub updated_at: Option<String>,
    /// Tamanho em KB.
    pub size_kb: Option<i32>,
    /// Posição na última listagem (`None`: sincronizado antes da 0008).
    pub inventory_position: Option<i32>,
}

/// A checagem mais recente de uma URL.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct StoredCheck {
    /// URL do site.
    pub url: String,
    /// `verified` | `unreachable` | `invalid`.
    pub outcome: String,
    /// Código HTTP (`None`: não houve resposta).
    pub http_status: Option<i16>,
    /// Destino depois dos redirecionamentos.
    pub final_url: Option<String>,
}

/// O inventário coletado.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollectedSnapshot {
    /// Repositórios ativos, na ordem da listagem; sem posição, por último,
    /// em ordem de nome.
    pub repos: Vec<StoredRepo>,
    /// Linguagens por `id` do repositório, na ordem da API.
    pub languages: HashMap<i64, Vec<(String, i64)>>,
    /// Checagem mais recente por URL de site ativo.
    pub checks: Vec<StoredCheck>,
}

impl Database {
    /// Lê o inventário, as linguagens e as checagens (só leitura).
    pub async fn collected_snapshot(&self) -> Result<CollectedSnapshot, StoreError> {
        let mut conn = self.connect(false).await?;
        let repos: Vec<StoredRepo> = sqlx::query_as(
            "SELECT id, name, full_name, description, visibility, is_fork, is_archived, homepage, \
                    to_char(github_pushed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS pushed_at, \
                    to_char(github_updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at, \
                    size_kb, inventory_position \
             FROM ecosystem.repositories WHERE gone_at IS NULL \
             ORDER BY inventory_position NULLS LAST, full_name_key, id",
        )
        .fetch_all(&mut conn)
        .await?;
        let rows: Vec<(i64, String, i64)> = sqlx::query_as(
            "SELECT repository_id, language, bytes FROM ecosystem.repository_languages \
             ORDER BY repository_id, position NULLS LAST, bytes DESC, language",
        )
        .fetch_all(&mut conn)
        .await?;
        let mut languages: HashMap<i64, Vec<(String, i64)>> = HashMap::new();
        for (repository_id, language, bytes) in rows {
            languages.entry(repository_id).or_default().push((language, bytes));
        }
        let checks: Vec<StoredCheck> = sqlx::query_as(
            "SELECT DISTINCT ON (url) url, outcome, http_status, final_url \
             FROM ecosystem.website_status_current WHERE check_id IS NOT NULL \
             ORDER BY url, last_checked_at DESC, check_id DESC",
        )
        .fetch_all(&mut conn)
        .await?;
        Ok(CollectedSnapshot { repos, languages, checks })
    }
}
