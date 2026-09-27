//! Manifestos editoriais no banco (D-011: editorial entra por importação).
//!
//! `docs/README_EXCLUDED.json`, `README_FEATURED.json`, `README_STACK.json` e
//! `README_SITES.json` continuam sendo a fonte — editados à mão, versionados
//! no Git. A importação deixa o banco igual a eles: entrada nova entra,
//! entrada alterada muda, entrada que saiu do manifesto sai do banco. A
//! exceção são os sites: um site que sai do manifesto é **aposentado**
//! (`retired_at`), porque as checagens dele são histórico.
//!
//! Tudo numa transação; um manifesto que não casa com o banco (projeto que o
//! `sync github` ainda não trouxe, categoria que não existe) recusa a
//! importação inteira, com a lista do que falta.

use std::collections::BTreeSet;

use sqlx::postgres::PgConnection;
use sqlx::{Connection, Postgres, Transaction};

use crate::{Database, RunContext, StoreError, close_run, open_run, record_failed_run, refresh_primary};

/// Arquivo de origem gravado em `repository_exclusions.source`.
pub const EXCLUDED_SOURCE: &str = "docs/README_EXCLUDED.json";

/// Uma entrada da curadoria (`README_FEATURED.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeaturedEntry {
    /// `owner/nome` do repositório primário do projeto.
    pub repository: String,
    /// `order`.
    pub position: i16,
    /// `priority` (0–100).
    pub priority: Option<i16>,
    /// `label`.
    pub label: String,
    /// `focus`.
    pub focus: Option<String>,
    /// `reason`.
    pub reason: Option<String>,
    /// `website_required`.
    pub website_required: bool,
}

/// Uma ferramenta do arsenal (`README_STACK.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackTool {
    /// Nome.
    pub name: String,
    /// Nome da categoria (`stack_categories.name`).
    pub category: String,
    /// Papel.
    pub family: String,
    /// Evidência pública.
    pub evidence: String,
}

/// Os quatro manifestos, já lidos e validados no formato.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifests {
    /// Nomes excluídos (`nome` ou `owner/nome`).
    pub exclusions: Vec<String>,
    /// Motivo geral das exclusões.
    pub exclusion_reason: String,
    /// Curadoria.
    pub featured: Vec<FeaturedEntry>,
    /// Arsenal, na ordem do manifesto.
    pub tools: Vec<StackTool>,
    /// Sites manuais: (`owner/nome`, URL já validada).
    pub sites: Vec<(String, String)>,
}

/// Resultado.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManifestsReport {
    /// Execução criada.
    pub sync_run_id: i64,
    /// Exclusões (gravadas, removidas).
    pub exclusions: (usize, usize),
    /// Curadoria (gravadas, removidas).
    pub featured: (usize, usize),
    /// Arsenal (gravadas, removidas).
    pub tools: (usize, usize),
    /// Sites manuais (ativos, aposentados).
    pub sites: (usize, usize),
    /// Sites de repositório ausente ou privado, ignorados como no Python.
    pub skipped_sites: Vec<String>,
}

impl Database {
    /// Importa os manifestos numa transação só.
    pub async fn import_manifests(
        &self,
        run: &RunContext,
        manifests: &Manifests,
    ) -> Result<ManifestsReport, StoreError> {
        let mut conn = self.connect(false).await?;
        let seen =
            manifests.exclusions.len() + manifests.featured.len() + manifests.tools.len() + manifests.sites.len();
        match import_on(&mut conn, run, manifests).await {
            Ok(report) => Ok(report),
            Err(error) => {
                record_failed_run(&mut conn, "manifests", run, seen, &error).await;
                Err(error)
            }
        }
    }
}

/// O projeto do repositório ativo `owner/nome`, e se ele é privado.
pub(crate) async fn project_of(
    tx: &mut Transaction<'_, Postgres>,
    full_name: &str,
) -> Result<Option<(i64, bool)>, StoreError> {
    Ok(sqlx::query_as(
        "SELECT pr.project_id, r.visibility <> 'public' FROM ecosystem.repositories r \
         JOIN ecosystem.project_repositories pr ON pr.repository_id = r.id \
         WHERE r.full_name_key = lower($1) AND r.gone_at IS NULL",
    )
    .bind(full_name)
    .fetch_optional(&mut **tx)
    .await?)
}

async fn import_on(
    conn: &mut PgConnection,
    run: &RunContext,
    manifests: &Manifests,
) -> Result<ManifestsReport, StoreError> {
    let mut tx = conn.begin().await?;
    let sync_run_id = open_run(&mut tx, "manifests", run).await?;
    let mut report = ManifestsReport { sync_run_id, ..ManifestsReport::default() };

    // Exclusões: a unicidade é sem caixa.
    for name in &manifests.exclusions {
        sqlx::query(
            "INSERT INTO ecosystem.repository_exclusions (match_name, reason, source) VALUES ($1, $2, $3) \
             ON CONFLICT ((lower(match_name))) DO UPDATE SET match_name = EXCLUDED.match_name, reason = EXCLUDED.reason, \
               source = EXCLUDED.source",
        )
        .bind(name)
        .bind(&manifests.exclusion_reason)
        .bind(EXCLUDED_SOURCE)
        .execute(&mut *tx)
        .await?;
    }
    let keep: Vec<String> = manifests.exclusions.iter().map(|name| name.to_lowercase()).collect();
    let removed = sqlx::query(
        "DELETE FROM ecosystem.repository_exclusions WHERE source = $1 AND NOT (lower(match_name) = ANY($2))",
    )
    .bind(EXCLUDED_SOURCE)
    .bind(&keep)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    report.exclusions = (manifests.exclusions.len(), removed as usize);

    // Curadoria: todo projeto precisa existir.
    let mut missing = Vec::new();
    let mut featured_projects = Vec::new();
    for entry in &manifests.featured {
        match project_of(&mut tx, &entry.repository).await? {
            Some((project_id, _)) => featured_projects.push((project_id, entry)),
            None => missing.push(format!("curadoria: {} não está no banco (rode sync github)", entry.repository)),
        }
    }
    let mut categories = BTreeSet::new();
    for tool in &manifests.tools {
        let known: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM ecosystem.stack_categories WHERE name = $1)")
                .bind(&tool.category)
                .fetch_one(&mut *tx)
                .await?;
        if !known && categories.insert(tool.category.clone()) {
            missing.push(format!("arsenal: categoria {:?} não existe (entra por migration)", tool.category));
        }
    }
    if !missing.is_empty() {
        return Err(StoreError::Manifest(missing.join("; ")));
    }

    for (project_id, entry) in &featured_projects {
        sqlx::query(
            "INSERT INTO ecosystem.featured_entries (project_id, position, priority, label, focus, reason, website_required) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (project_id) DO UPDATE SET position = EXCLUDED.position, priority = EXCLUDED.priority, \
               label = EXCLUDED.label, focus = EXCLUDED.focus, reason = EXCLUDED.reason, \
               website_required = EXCLUDED.website_required",
        )
        .bind(project_id)
        .bind(entry.position)
        .bind(entry.priority)
        .bind(&entry.label)
        .bind(&entry.focus)
        .bind(&entry.reason)
        .bind(entry.website_required)
        .execute(&mut *tx)
        .await?;
    }
    let featured_ids: Vec<i64> = featured_projects.iter().map(|(id, _)| *id).collect();
    let removed = sqlx::query("DELETE FROM ecosystem.featured_entries WHERE NOT (project_id = ANY($1))")
        .bind(&featured_ids)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    report.featured = (featured_ids.len(), removed as usize);

    for (position, tool) in manifests.tools.iter().enumerate() {
        sqlx::query(
            "INSERT INTO ecosystem.stack_tools (name, category_slug, family, evidence, position) \
             SELECT $1, slug, $3, $4, $5 FROM ecosystem.stack_categories WHERE name = $2 \
             ON CONFLICT (name) DO UPDATE SET category_slug = EXCLUDED.category_slug, family = EXCLUDED.family, \
               evidence = EXCLUDED.evidence, position = EXCLUDED.position",
        )
        .bind(&tool.name)
        .bind(&tool.category)
        .bind(&tool.family)
        .bind(&tool.evidence)
        .bind(i16::try_from(position + 1).unwrap_or(i16::MAX))
        .execute(&mut *tx)
        .await?;
    }
    let names: Vec<String> = manifests.tools.iter().map(|tool| tool.name.clone()).collect();
    let removed = sqlx::query("DELETE FROM ecosystem.stack_tools WHERE NOT (name = ANY($1))")
        .bind(&names)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    report.tools = (names.len(), removed as usize);

    // Sites manuais: privado nunca anuncia site; repositório ausente é ignorado.
    let mut kept = Vec::new();
    for (full_name, url) in &manifests.sites {
        match project_of(&mut tx, full_name).await? {
            Some((project_id, false)) => {
                let website_id: i64 = sqlx::query_scalar(
                    "INSERT INTO ecosystem.websites (project_id, url, source, is_primary) VALUES ($1, $2, 'manifest', false) \
                     ON CONFLICT (project_id, url) DO UPDATE SET retired_at = NULL RETURNING id",
                )
                .bind(project_id)
                .bind(url)
                .fetch_one(&mut *tx)
                .await?;
                kept.push(website_id);
                refresh_primary(&mut tx, project_id).await?;
            }
            Some((_, true)) => report.skipped_sites.push(format!("{full_name} (privado)")),
            None => report.skipped_sites.push(format!("{full_name} (fora do banco)")),
        }
    }
    let retired: Vec<i64> = sqlx::query_scalar(
        "UPDATE ecosystem.websites SET retired_at = now(), is_primary = false \
         WHERE source = 'manifest' AND retired_at IS NULL AND NOT (id = ANY($1)) RETURNING project_id",
    )
    .bind(&kept)
    .fetch_all(&mut *tx)
    .await?;
    for project_id in &retired {
        refresh_primary(&mut tx, *project_id).await?;
    }
    report.sites = (kept.len(), retired.len());

    let changed = report.exclusions.1 + report.featured.1 + report.tools.1 + report.sites.1;
    close_run(
        &mut tx,
        sync_run_id,
        manifests.exclusions.len() + manifests.featured.len() + manifests.tools.len() + manifests.sites.len(),
        changed,
    )
    .await?;
    tx.commit().await?;
    Ok(report)
}
