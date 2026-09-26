//! O inventário do GitHub no banco: donos, repositórios, linguagens e o
//! projeto 1:1 de cada repositório.
//!
//! Regras (docs/database/DATA-MODEL.md, D-005, D-019):
//!
//! * a identidade é o `github_id` — renomear atualiza a linha, não cria outra;
//! * nada é apagado: o repositório que some do inventário ganha `gone_at` (e
//!   perde, se voltar) — e isso **só** com inventário completo, senão um
//!   inventário só de públicos "sumiria" com os privados;
//! * repositório excluído pelo manifesto não chega aqui (quem chama filtra);
//! * o mapa de linguagens é estado atual e é trocado quando muda; uma consulta
//!   de linguagens que **falhou** não apaga o mapa anterior;
//! * cada repositório novo ganha um projeto com o rótulo da heurística
//!   (`classifier_version`); rótulo editorial nunca é sobrescrito.

use std::collections::BTreeMap;

use sqlx::Connection;
use sqlx::postgres::PgConnection;

use crate::{Database, RunContext, StoreError, close_run, open_run, record_failed_run};

/// Dono de um repositório.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerRecord {
    /// `owner.id`.
    pub github_id: i64,
    /// `owner.login`.
    pub login: String,
    /// `User` | `Organization`.
    pub kind: String,
}

/// O recorte de um repositório que o banco guarda.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRecord {
    /// `id`.
    pub github_id: i64,
    /// `node_id`.
    pub node_id: Option<String>,
    /// Dono.
    pub owner: OwnerRecord,
    /// Nome curto.
    pub name: String,
    /// `owner/nome`.
    pub full_name: String,
    /// Descrição.
    pub description: Option<String>,
    /// `public` | `private` | `internal`.
    pub visibility: String,
    /// É fork.
    pub fork: bool,
    /// Está arquivado.
    pub archived: bool,
    /// É template.
    pub template: bool,
    /// Branch padrão.
    pub default_branch: Option<String>,
    /// Linguagem principal.
    pub primary_language: Option<String>,
    /// `homepage` (vazia vira ausente).
    pub homepage: Option<String>,
    /// A `homepage` quando ela é um site anunciável (URL http(s) válida, sem
    /// espaços nas pontas) e o repositório é público — a descoberta do
    /// `project_catalog.py`. Vira um site `github_homepage` do projeto.
    pub homepage_site: Option<String>,
    /// Tópicos.
    pub topics: Vec<String>,
    /// Tamanho em KB.
    pub size_kb: Option<i32>,
    /// Criação no GitHub (ISO 8601).
    pub created_at: Option<String>,
    /// Última atualização no GitHub.
    pub updated_at: Option<String>,
    /// Último push.
    pub pushed_at: Option<String>,
    /// Bytes por linguagem; `None` quando a consulta falhou (o mapa anterior fica).
    pub languages: Option<Vec<(String, i64)>>,
    /// Rótulo da heurística (`classify`), texto exibido.
    pub label: String,
    /// Versão da heurística que produziu o rótulo.
    pub classifier_version: String,
    /// Slugs para um projeto novo, em ordem de preferência (o primeiro livre
    /// vence; se nenhum estiver livre, `repo-<github_id>`).
    pub slug_candidates: Vec<String>,
}

/// Uma sincronização do inventário.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryRecord {
    /// Repositórios (já sem os excluídos).
    pub repos: Vec<RepoRecord>,
    /// `github_id` de tudo que o GitHub listou, excluídos inclusive: o que não
    /// está aqui sumiu.
    pub seen: Vec<i64>,
    /// O inventário inclui os privados (token de inventário ou arquivo). Só
    /// então quem não aparece é marcado como sumido.
    pub complete: bool,
}

/// Resultado.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InventoryReport {
    /// Execução criada.
    pub sync_run_id: i64,
    /// Repositórios novos.
    pub inserted: usize,
    /// Repositórios cujos dados mudaram.
    pub updated: usize,
    /// Repositórios marcados como sumidos.
    pub gone: usize,
    /// Mapas de linguagem trocados.
    pub languages_changed: usize,
    /// Projetos criados.
    pub projects_created: usize,
    /// Sites de homepage aposentados (a homepage mudou ou saiu).
    pub sites_retired: usize,
}

impl Database {
    /// Grava o inventário numa transação só; falha registra a execução como `failed`.
    pub async fn record_inventory(
        &self,
        run: &RunContext,
        inventory: &InventoryRecord,
    ) -> Result<InventoryReport, StoreError> {
        let mut conn = self.connect(false).await?;
        match record_on(&mut conn, run, inventory).await {
            Ok(report) => Ok(report),
            Err(error) => {
                record_failed_run(&mut conn, "github_inventory", run, inventory.repos.len(), &error).await;
                Err(error)
            }
        }
    }
}

/// O que decide se um repositório "mudou" (para `items_changed`).
#[derive(Debug, PartialEq, sqlx::FromRow)]
struct Snapshot {
    name: String,
    full_name: String,
    description: Option<String>,
    visibility: String,
    is_fork: bool,
    is_archived: bool,
    is_template: bool,
    default_branch: Option<String>,
    primary_language: Option<String>,
    homepage: Option<String>,
    topics: Vec<String>,
    size_kb: Option<i32>,
    pushed_at: Option<String>,
    gone: bool,
}

async fn record_on(
    conn: &mut PgConnection,
    run: &RunContext,
    inventory: &InventoryRecord,
) -> Result<InventoryReport, StoreError> {
    let mut tx = conn.begin().await?;
    let sync_run_id = open_run(&mut tx, "github_inventory", run).await?;
    let mut report = InventoryReport { sync_run_id, ..InventoryReport::default() };

    if inventory.complete {
        report.gone = sqlx::query(
            "UPDATE ecosystem.repositories SET gone_at = now() \
             WHERE gone_at IS NULL AND NOT (github_id = ANY($1))",
        )
        .bind(&inventory.seen)
        .execute(&mut *tx)
        .await?
        .rows_affected() as usize;
    }

    for repo in &inventory.repos {
        let owner_id: i64 = sqlx::query_scalar(
            "INSERT INTO ecosystem.github_owners (github_id, login, kind) VALUES ($1, $2, $3) \
             ON CONFLICT (github_id) DO UPDATE SET login = EXCLUDED.login, kind = EXCLUDED.kind RETURNING id",
        )
        .bind(repo.owner.github_id)
        .bind(&repo.owner.login)
        .bind(&repo.owner.kind)
        .fetch_one(&mut *tx)
        .await?;

        let mut languages: Vec<&str> = repo.primary_language.iter().map(String::as_str).collect();
        languages.extend(repo.languages.iter().flatten().map(|(language, _)| language.as_str()));
        for language in languages {
            sqlx::query(
                "INSERT INTO ecosystem.languages (name, display_name) VALUES ($1, $1) ON CONFLICT (name) DO NOTHING",
            )
            .bind(language)
            .execute(&mut *tx)
            .await?;
        }

        let before: Option<Snapshot> = sqlx::query_as(
            "SELECT name, full_name, description, visibility, is_fork, is_archived, is_template, default_branch, \
                    primary_language, homepage, topics, size_kb, to_char(github_pushed_at AT TIME ZONE 'UTC', \
                    'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS pushed_at, gone_at IS NOT NULL AS gone \
             FROM ecosystem.repositories WHERE github_id = $1",
        )
        .bind(repo.github_id)
        .fetch_optional(&mut *tx)
        .await?;

        let repository_id: i64 = sqlx::query_scalar(
            "INSERT INTO ecosystem.repositories \
               (github_id, github_node_id, owner_id, name, full_name, description, visibility, is_fork, is_archived, \
                is_template, default_branch, primary_language, homepage, topics, size_kb, github_created_at, \
                github_updated_at, github_pushed_at, last_synced_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16::timestamptz, \
                     $17::timestamptz, $18::timestamptz, now()) \
             ON CONFLICT (github_id) DO UPDATE SET \
               github_node_id = EXCLUDED.github_node_id, owner_id = EXCLUDED.owner_id, name = EXCLUDED.name, \
               full_name = EXCLUDED.full_name, description = EXCLUDED.description, visibility = EXCLUDED.visibility, \
               is_fork = EXCLUDED.is_fork, is_archived = EXCLUDED.is_archived, is_template = EXCLUDED.is_template, \
               default_branch = EXCLUDED.default_branch, primary_language = EXCLUDED.primary_language, \
               homepage = EXCLUDED.homepage, topics = EXCLUDED.topics, size_kb = EXCLUDED.size_kb, \
               github_created_at = EXCLUDED.github_created_at, github_updated_at = EXCLUDED.github_updated_at, \
               github_pushed_at = EXCLUDED.github_pushed_at, last_synced_at = now(), gone_at = NULL \
             RETURNING id",
        )
        .bind(repo.github_id)
        .bind(&repo.node_id)
        .bind(owner_id)
        .bind(&repo.name)
        .bind(&repo.full_name)
        .bind(&repo.description)
        .bind(&repo.visibility)
        .bind(repo.fork)
        .bind(repo.archived)
        .bind(repo.template)
        .bind(&repo.default_branch)
        .bind(&repo.primary_language)
        .bind(&repo.homepage)
        .bind(&repo.topics)
        .bind(repo.size_kb)
        .bind(&repo.created_at)
        .bind(&repo.updated_at)
        .bind(&repo.pushed_at)
        .fetch_one(&mut *tx)
        .await?;

        match before {
            None => report.inserted += 1,
            Some(before) => {
                let now = Snapshot {
                    name: repo.name.clone(),
                    full_name: repo.full_name.clone(),
                    description: repo.description.clone(),
                    visibility: repo.visibility.clone(),
                    is_fork: repo.fork,
                    is_archived: repo.archived,
                    is_template: repo.template,
                    default_branch: repo.default_branch.clone(),
                    primary_language: repo.primary_language.clone(),
                    homepage: repo.homepage.clone(),
                    topics: repo.topics.clone(),
                    size_kb: repo.size_kb,
                    pushed_at: repo.pushed_at.as_deref().map(normalize_timestamp),
                    gone: false,
                };
                if before != now {
                    report.updated += 1;
                }
            }
        }

        if let Some(languages) = &repo.languages {
            let current: Vec<(String, i64)> =
                sqlx::query_as("SELECT language, bytes FROM ecosystem.repository_languages WHERE repository_id = $1")
                    .bind(repository_id)
                    .fetch_all(&mut *tx)
                    .await?;
            let current: BTreeMap<String, i64> = current.into_iter().collect();
            let new: BTreeMap<String, i64> = languages.iter().cloned().collect();
            if current != new {
                sqlx::query("DELETE FROM ecosystem.repository_languages WHERE repository_id = $1")
                    .bind(repository_id)
                    .execute(&mut *tx)
                    .await?;
                for (language, bytes) in &new {
                    sqlx::query(
                        "INSERT INTO ecosystem.repository_languages (repository_id, language, bytes, sync_run_id) \
                         VALUES ($1, $2, $3, $4)",
                    )
                    .bind(repository_id)
                    .bind(language)
                    .bind(bytes)
                    .bind(sync_run_id)
                    .execute(&mut *tx)
                    .await?;
                }
                report.languages_changed += 1;
            }
        }

        let (project_id, created) = ensure_project(&mut tx, repository_id, repo).await?;
        if created {
            report.projects_created += 1;
        }
        if let Some(url) = &repo.homepage_site {
            sqlx::query(
                "INSERT INTO ecosystem.websites (project_id, url, source, is_primary) \
                 VALUES ($1, $2, 'github_homepage', false) \
                 ON CONFLICT (project_id, url) DO UPDATE SET retired_at = NULL",
            )
            .bind(project_id)
            .bind(url)
            .execute(&mut *tx)
            .await?;
        }
        // A homepage mudou ou saiu: o site antigo é aposentado (as checagens ficam).
        report.sites_retired += sqlx::query(
            "UPDATE ecosystem.websites SET retired_at = now(), is_primary = false \
             WHERE project_id = $1 AND source = 'github_homepage' AND retired_at IS NULL \
               AND url IS DISTINCT FROM $2",
        )
        .bind(project_id)
        .bind(&repo.homepage_site)
        .execute(&mut *tx)
        .await?
        .rows_affected() as usize;
        crate::refresh_primary(&mut tx, project_id).await?;
    }

    let changed = report.inserted + report.updated + report.gone;
    close_run(&mut tx, sync_run_id, inventory.repos.len(), changed).await?;
    tx.commit().await?;
    Ok(report)
}

/// `2026-09-20T10:00:00+00:00` e `2026-09-20T10:00:00Z` são o mesmo instante;
/// a comparação com o banco usa a forma com `Z`.
fn normalize_timestamp(value: &str) -> String {
    value.strip_suffix("+00:00").map_or_else(|| value.to_string(), |base| format!("{base}Z"))
}

/// Projeto 1:1 do repositório: cria se não existe; atualiza o rótulo da
/// heurística (nunca o editorial). Devolve o projeto e se ele foi criado.
async fn ensure_project(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    repository_id: i64,
    repo: &RepoRecord,
) -> Result<(i64, bool), StoreError> {
    let label_slug: Option<String> =
        sqlx::query_scalar("SELECT slug FROM ecosystem.classification_labels WHERE label = $1")
            .bind(&repo.label)
            .fetch_optional(&mut **tx)
            .await?;
    let existing: Option<i64> =
        sqlx::query_scalar("SELECT project_id FROM ecosystem.project_repositories WHERE repository_id = $1")
            .bind(repository_id)
            .fetch_optional(&mut **tx)
            .await?;
    if let Some(project_id) = existing {
        sqlx::query(
            "UPDATE ecosystem.projects SET label_slug = $2, classifier_version = $3 \
             WHERE id = $1 AND label_source = 'heuristic' \
               AND (label_slug IS DISTINCT FROM $2 OR classifier_version IS DISTINCT FROM $3)",
        )
        .bind(project_id)
        .bind(&label_slug)
        .bind(&repo.classifier_version)
        .execute(&mut **tx)
        .await?;
        return Ok((project_id, false));
    }

    let mut slug = None;
    for candidate in repo.slug_candidates.iter().cloned().chain([format!("repo-{}", repo.github_id)]) {
        if candidate.is_empty() {
            continue;
        }
        let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ecosystem.projects WHERE slug = $1)")
            .bind(&candidate)
            .fetch_one(&mut **tx)
            .await?;
        if !taken {
            slug = Some(candidate);
            break;
        }
    }
    let slug = slug.unwrap_or_else(|| format!("repo-{}", repo.github_id));
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO ecosystem.projects (slug, name, label_slug, label_source, classifier_version) \
         VALUES ($1, $2, $3, 'heuristic', $4) RETURNING id",
    )
    .bind(&slug)
    .bind(&repo.name)
    .bind(&label_slug)
    .bind(&repo.classifier_version)
    .fetch_one(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO ecosystem.project_repositories (project_id, repository_id, role) VALUES ($1, $2, 'primary')",
    )
    .bind(project_id)
    .bind(repository_id)
    .execute(&mut **tx)
    .await?;
    Ok((project_id, true))
}
