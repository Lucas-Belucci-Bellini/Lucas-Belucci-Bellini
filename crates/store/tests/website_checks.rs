//! Histórico de verificação de sites (`Database::record_website_checks`)
//! contra um PostgreSQL de verdade, sobre o seed sintético.

mod common;

use common::{TempDb, load_seed};
use sqlx::postgres::PgConnection;
use store::{RunContext, WebsiteCheckRecord};

const AURORA: &str = "https://aurora-web.example.org";

fn run() -> RunContext {
    RunContext { trigger: "test".into(), source: "cargo-test".into(), code_version: None, external_ref: None }
}

fn verified(url: &str) -> WebsiteCheckRecord {
    WebsiteCheckRecord {
        url: url.into(),
        checked_at: "2026-09-26T12:00:00+00:00".into(),
        outcome: "verified".into(),
        http_status: Some(200),
        final_url: Some(format!("{url}/")),
        redirect_count: 1,
        response_time_ms: Some(42),
        attempts: 1,
        error_kind: None,
        error_message: None,
    }
}

fn unreachable(url: &str) -> WebsiteCheckRecord {
    WebsiteCheckRecord {
        outcome: "unreachable".into(),
        http_status: None,
        final_url: Some(url.into()),
        redirect_count: 0,
        attempts: 2,
        error_kind: Some("timeout".into()),
        error_message: Some("operation timed out".into()),
        ..verified(url)
    }
}

async fn count(conn: &mut PgConnection, sql: &'static str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(conn).await.expect(sql)
}

async fn prepared() -> Option<TempDb> {
    let db = TempDb::create().await?;
    db.database().migrate().await.expect("migrate");
    load_seed(&db).await;
    Some(db)
}

#[tokio::test]
async fn grava_so_sites_registrados_e_relata_os_outros() {
    let Some(db) = prepared().await else { return };
    let mut conn = db.conn().await;
    let before = count(&mut conn, "SELECT count(*) FROM ecosystem.website_checks").await;

    let report = db
        .database()
        .record_website_checks(&run(), &[verified(AURORA), unreachable("https://nao-registrado.example.org")])
        .await
        .expect("grava");
    assert_eq!(1, report.recorded);
    assert_eq!(vec!["https://nao-registrado.example.org".to_string()], report.unregistered);
    assert_eq!(before + 1, count(&mut conn, "SELECT count(*) FROM ecosystem.website_checks").await);

    let (kind, status, seen, changed): (String, String, i32, i32) =
        sqlx::query_as("SELECT kind, status, items_seen, items_changed FROM ecosystem.sync_runs WHERE id = $1")
            .bind(report.sync_run_id)
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!(("websites", "succeeded", 2, 1), (kind.as_str(), status.as_str(), seen, changed));

    let (outcome, http, redirects, ms): (String, Option<i16>, i16, Option<i32>) = sqlx::query_as(
        "SELECT outcome, http_status, redirect_count, response_time_ms \
         FROM ecosystem.website_status_current WHERE url = $1",
    )
    .bind(AURORA)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(
        ("verified", Some(200), 1, Some(42)),
        (outcome.as_str(), http, redirects, ms),
        "a checagem nova é a atual"
    );
}

#[tokio::test]
async fn historico_so_cresce() {
    let Some(db) = prepared().await else { return };
    let mut conn = db.conn().await;
    let database = db.database();
    let before = count(&mut conn, "SELECT count(*) FROM ecosystem.website_checks").await;
    database.record_website_checks(&run(), &[unreachable(AURORA)]).await.unwrap();
    database.record_website_checks(&run(), &[verified(AURORA)]).await.unwrap();
    assert_eq!(before + 2, count(&mut conn, "SELECT count(*) FROM ecosystem.website_checks").await);
    let failures: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM ecosystem.website_checks c JOIN ecosystem.websites w ON w.id = c.website_id \
         WHERE w.url = $1 AND c.error_kind = 'timeout' AND c.http_status IS NULL AND c.attempts = 2",
    )
    .bind(AURORA)
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(1, failures, "a falha continua no histórico depois da recuperação");
}

#[tokio::test]
async fn falha_nao_grava_nada_e_fica_registrada() {
    let Some(db) = prepared().await else { return };
    let mut conn = db.conn().await;
    let before = count(&mut conn, "SELECT count(*) FROM ecosystem.website_checks").await;
    // `verified` sem HTTP viola website_checks_verified_has_http.
    let broken = WebsiteCheckRecord { http_status: None, ..verified(AURORA) };
    let error = db.database().record_website_checks(&run(), &[verified(AURORA), broken]).await.expect_err("recusa");
    assert!(error.to_string().contains("website_checks_verified_has_http"), "{error}");

    assert_eq!(before, count(&mut conn, "SELECT count(*) FROM ecosystem.website_checks").await, "nada entrou");
    let (status, message): (String, Option<String>) = sqlx::query_as(
        "SELECT status, error_message FROM ecosystem.sync_runs WHERE kind = 'websites' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!("failed", status);
    assert!(message.is_some_and(|m| m.contains("website_checks_verified_has_http")));
}

#[tokio::test]
async fn mesma_url_em_dois_projetos_grava_para_cada_site() {
    let Some(db) = prepared().await else { return };
    let mut conn = db.conn().await;
    sqlx::query(
        "INSERT INTO ecosystem.websites (project_id, url, source, is_primary) \
         SELECT id, $1, 'manifest', false FROM ecosystem.projects WHERE slug <> 'aurora-web' ORDER BY id LIMIT 1",
    )
    .bind(AURORA)
    .execute(&mut conn)
    .await
    .unwrap();
    let report = db.database().record_website_checks(&run(), &[verified(AURORA)]).await.unwrap();
    assert_eq!(2, report.recorded);
    assert!(report.unregistered.is_empty());
}

#[tokio::test]
async fn site_aposentado_nao_recebe_checagem() {
    let Some(db) = prepared().await else { return };
    let mut conn = db.conn().await;
    sqlx::query("UPDATE ecosystem.websites SET retired_at = now() WHERE url = $1")
        .bind(AURORA)
        .execute(&mut conn)
        .await
        .unwrap();
    let report = db.database().record_website_checks(&run(), &[verified(AURORA)]).await.unwrap();
    assert_eq!(0, report.recorded);
    assert_eq!(vec![AURORA.to_string()], report.unregistered);
}
