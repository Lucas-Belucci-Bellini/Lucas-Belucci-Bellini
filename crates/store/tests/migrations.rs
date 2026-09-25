//! Migrations contra um PostgreSQL de verdade.
//!
//! Cada teste cria um banco próprio (`store_test_<pid>_<n>`) a partir de
//! `STORE_TEST_DATABASE_URL` (ou `DATABASE_URL`) e o apaga ao terminar — o
//! banco apontado pela URL nunca é tocado, só usado para `CREATE DATABASE`.
//! Sem URL, os testes são pulados; com `STORE_TESTS_REQUIRED=1` (o CI), a
//! falta de URL reprova em vez de pular calada.

use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};

use ecosystem_domain::taxonomy::{CATEGORIES, CATEGORY_ALIASES, DOMAINS};
use sqlx::postgres::{PgConnectOptions, PgConnection};
use sqlx::{AssertSqlSafe, Connection};
use store::{Database, MigrationState, RevertTarget, StoreError};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Banco descartável; apagado no `Drop`, inclusive quando o teste falha.
struct TempDb {
    admin: PgConnectOptions,
    name: String,
    options: PgConnectOptions,
}

impl TempDb {
    async fn create() -> Option<Self> {
        let url = std::env::var("STORE_TEST_DATABASE_URL").or_else(|_| std::env::var("DATABASE_URL"));
        let Ok(url) = url else {
            assert!(
                std::env::var("STORE_TESTS_REQUIRED").is_err(),
                "STORE_TESTS_REQUIRED está ligado, mas não há STORE_TEST_DATABASE_URL nem DATABASE_URL"
            );
            eprintln!("pulado: defina STORE_TEST_DATABASE_URL para rodar os testes de banco");
            return None;
        };
        let admin = PgConnectOptions::from_str(&url).expect("URL de teste válida");
        let name = format!("store_test_{}_{}", std::process::id(), COUNTER.fetch_add(1, Ordering::SeqCst));
        let mut conn = PgConnection::connect_with(&admin).await.expect("conexão administrativa");
        sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut conn)
            .await
            .expect("CREATE DATABASE");
        let options = admin.clone().database(&name);
        Some(Self { admin, name, options })
    }

    fn database(&self) -> Database {
        Database::from_options(self.options.clone())
    }

    async fn conn(&self) -> PgConnection {
        PgConnection::connect_with(&self.options).await.expect("conexão ao banco de teste")
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let (admin, name) = (self.admin.clone(), self.name.clone());
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
            runtime.block_on(async {
                if let Ok(mut conn) = PgConnection::connect_with(&admin).await {
                    let drop = format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)");
                    let _ = sqlx::query(AssertSqlSafe(drop)).execute(&mut conn).await;
                }
            });
        })
        .join()
        .ok();
    }
}

fn states(status: &[store::MigrationStatus]) -> Vec<(i64, MigrationState)> {
    status.iter().map(|m| (m.version, m.state)).collect()
}

fn all(state: MigrationState) -> Vec<(i64, MigrationState)> {
    (1..=7).map(|version| (version, state)).collect()
}

async fn scalar_i64(conn: &mut PgConnection, sql: &'static str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(conn).await.expect(sql)
}

async fn load_seed(db: &TempDb) {
    let seed = include_str!("../../../db/seeds/dev/0001_demo_ecosystem.sql");
    sqlx::raw_sql(seed).execute(&mut db.conn().await).await.expect("seed de desenvolvimento");
}

#[tokio::test]
async fn migrate_aplica_tudo_uma_vez_e_status_nao_escreve() {
    let Some(db) = TempDb::create().await else { return };
    let database = db.database();

    assert_eq!(all(MigrationState::Pending), states(&database.status().await.unwrap()));
    let mut conn = db.conn().await;
    assert_eq!(
        0,
        scalar_i64(&mut conn, "SELECT count(*) FROM pg_class WHERE relname = '_sqlx_migrations'").await,
        "status não cria nem a tabela de controle"
    );

    let applied = database.migrate().await.unwrap();
    assert_eq!((1..=7).collect::<Vec<_>>(), applied.iter().map(|m| m.version).collect::<Vec<_>>());
    let status = database.status().await.unwrap();
    assert_eq!(all(MigrationState::Applied), states(&status));
    assert!(status.iter().all(|m| m.installed_on.as_deref().is_some_and(|t| t.ends_with('Z'))));

    assert!(database.migrate().await.unwrap().is_empty(), "segunda execução não aplica nada");
    assert_eq!(1, scalar_i64(&mut conn, "SELECT count(*) FROM pg_namespace WHERE nspname = 'ecosystem'").await);
}

#[tokio::test]
async fn revert_ultima_e_revert_total_sem_dados() {
    let Some(db) = TempDb::create().await else { return };
    let database = db.database();
    database.migrate().await.unwrap();

    let reverted = database.revert(RevertTarget::Last, false).await.unwrap();
    assert_eq!(vec![7], reverted.iter().map(|m| m.version).collect::<Vec<_>>());
    let mut expected = all(MigrationState::Applied);
    expected[6].1 = MigrationState::Pending;
    assert_eq!(expected, states(&database.status().await.unwrap()));

    let reverted = database.revert(RevertTarget::Version(0), false).await.unwrap();
    assert_eq!((1..=6).rev().collect::<Vec<_>>(), reverted.iter().map(|m| m.version).collect::<Vec<_>>());
    assert_eq!(all(MigrationState::Pending), states(&database.status().await.unwrap()));
    let mut conn = db.conn().await;
    assert_eq!(0, scalar_i64(&mut conn, "SELECT count(*) FROM pg_namespace WHERE nspname = 'ecosystem'").await);

    assert!(database.revert(RevertTarget::Last, false).await.unwrap().is_empty(), "nada a reverter");
    assert!(matches!(database.revert(RevertTarget::Version(42), false).await, Err(StoreError::UnknownTarget(42))));
}

#[tokio::test]
async fn revert_com_dados_recusa_sem_opt_in_e_nao_muda_nada() {
    let Some(db) = TempDb::create().await else { return };
    let database = db.database();
    database.migrate().await.unwrap();
    load_seed(&db).await;
    let mut conn = db.conn().await;
    let projects = scalar_i64(&mut conn, "SELECT count(*) FROM ecosystem.projects").await;
    assert!(projects > 0, "o seed tem o que proteger");

    let refused = database.revert(RevertTarget::Version(0), false).await;
    let Err(StoreError::DataLossRefused { version, detail }) = refused else {
        panic!("esperava DataLossRefused, veio {refused:?}");
    };
    assert!((1..=6).contains(&version), "{version}");
    assert!(detail.starts_with("down migration would destroy data"), "{detail}");

    // Tudo ou nada: nem a 0007 (views, sem dado) ficou revertida.
    assert_eq!(all(MigrationState::Applied), states(&database.status().await.unwrap()));
    assert_eq!(projects, scalar_i64(&mut conn, "SELECT count(*) FROM ecosystem.projects").await);
    assert_eq!(
        1,
        scalar_i64(
            &mut conn,
            "SELECT count(*) FROM pg_views WHERE schemaname = 'ecosystem' AND viewname = 'public_projects'"
        )
        .await
    );

    // Com opt-in explícito, passa.
    let reverted = database.revert(RevertTarget::Version(0), true).await.unwrap();
    assert_eq!(7, reverted.len());
    assert_eq!(0, scalar_i64(&mut conn, "SELECT count(*) FROM pg_namespace WHERE nspname = 'ecosystem'").await);
}

#[tokio::test]
async fn opt_in_de_perda_de_dados_vale_so_para_o_revert() {
    let Some(db) = TempDb::create().await else { return };
    let database = db.database();
    database.migrate().await.unwrap();
    load_seed(&db).await;
    // Um revert com opt-in de outra conexão não deixa a permissão ligada no banco.
    database.revert(RevertTarget::Last, true).await.unwrap();
    let mut conn = db.conn().await;
    let setting: Option<String> = sqlx::query_scalar("SELECT current_setting('ecosystem.allow_data_loss', true)")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_ne!(Some("on"), setting.as_deref());
    database.migrate().await.unwrap();
    assert!(matches!(database.revert(RevertTarget::Version(0), false).await, Err(StoreError::DataLossRefused { .. })));
}

#[tokio::test]
async fn alterada_desconhecida_e_pela_metade_sao_recusadas_antes_de_mudar() {
    let Some(db) = TempDb::create().await else { return };
    let database = db.database();
    database.migrate().await.unwrap();
    let mut conn = db.conn().await;

    sqlx::query("UPDATE _sqlx_migrations SET checksum = '\\x00' WHERE version = 3").execute(&mut conn).await.unwrap();
    assert_eq!(MigrationState::Modified, database.status().await.unwrap()[2].state);
    assert!(matches!(database.migrate().await, Err(StoreError::Modified(3))));
    assert!(matches!(database.revert(RevertTarget::Last, false).await, Err(StoreError::Modified(3))));
    assert_eq!(0, scalar_i64(&mut conn, "SELECT 7 - count(*) FROM _sqlx_migrations").await, "nada revertido");

    sqlx::query("UPDATE _sqlx_migrations SET checksum = '\\x00', success = false WHERE version = 3")
        .execute(&mut conn)
        .await
        .unwrap();
    assert_eq!(MigrationState::Dirty, database.status().await.unwrap()[2].state);
    assert!(matches!(database.migrate().await, Err(StoreError::Dirty(3))));

    assert!(matches!(database.revert(RevertTarget::Version(0), false).await, Err(StoreError::Dirty(3))));

    // Banco à frente do binário: uma versão que este código não conhece.
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 3").execute(&mut conn).await.unwrap();
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) \
         VALUES (99, 'do futuro', true, '\\x00', 0)",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    let status = database.status().await.unwrap();
    assert_eq!(Some(&(99, MigrationState::Unknown)), states(&status).last());
    assert!(matches!(database.migrate().await, Err(StoreError::UnknownApplied(99))));
}

#[tokio::test]
async fn taxonomia_do_banco_bate_com_o_crate_de_dominio() {
    let Some(db) = TempDb::create().await else { return };
    db.database().migrate().await.unwrap();
    let mut conn = db.conn().await;

    let categories: Vec<String> = sqlx::query_scalar("SELECT name FROM ecosystem.categories ORDER BY position")
        .fetch_all(&mut conn)
        .await
        .unwrap();
    assert_eq!(CATEGORIES.to_vec(), categories, "0003 e ecosystem_domain::taxonomy::CATEGORIES");

    let mut aliases: Vec<(String, String)> = sqlx::query_as(
        "SELECT l.label, c.name FROM ecosystem.classification_labels l \
         JOIN ecosystem.categories c ON c.slug = l.category_slug",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();
    aliases.sort();
    let mut expected: Vec<(String, String)> =
        CATEGORY_ALIASES.iter().map(|(label, category)| (label.to_string(), category.to_string())).collect();
    expected.sort();
    assert_eq!(expected, aliases, "0003 e CATEGORY_ALIASES");

    // Ordena no Rust: a ordem do banco depende da collation do servidor.
    let mut domains: Vec<(String, Vec<String>)> = sqlx::query_as(
        "SELECT d.title, coalesce(array_agg(l.label) FILTER (WHERE l.label IS NOT NULL), '{}') \
         FROM ecosystem.showcase_domains d \
         LEFT JOIN ecosystem.classification_labels l ON l.domain_slug = d.slug \
         GROUP BY d.title, d.position ORDER BY d.position",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();
    domains.iter_mut().for_each(|(_, labels)| labels.sort());
    let expected: Vec<(String, Vec<String>)> = DOMAINS
        .iter()
        .map(|(title, labels)| {
            let mut labels: Vec<String> = labels.iter().map(|l| l.to_string()).collect();
            labels.sort();
            (title.to_string(), labels)
        })
        .collect();
    assert_eq!(expected, domains, "0003 e DOMAINS");
}
