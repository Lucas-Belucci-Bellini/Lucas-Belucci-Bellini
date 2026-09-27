//! Banco descartável para os testes de integração do `store`.
//!
//! Cada teste cria um banco próprio (`store_test_<pid>_<n>`) a partir de
//! `STORE_TEST_DATABASE_URL` (ou `DATABASE_URL`) e o apaga ao terminar — o
//! banco apontado pela URL nunca é tocado, só usado para `CREATE DATABASE`.
//! Sem URL, os testes são pulados; com `STORE_TESTS_REQUIRED=1` (o CI), a
//! falta de URL reprova em vez de pular calada.

#![allow(dead_code)]

use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};

use sqlx::postgres::{PgConnectOptions, PgConnection};
use sqlx::{AssertSqlSafe, Connection};
use store::Database;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Banco descartável; apagado no `Drop`, inclusive quando o teste falha.
pub struct TempDb {
    admin: PgConnectOptions,
    name: String,
    options: PgConnectOptions,
}

impl TempDb {
    pub async fn create() -> Option<Self> {
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

    pub fn database(&self) -> Database {
        Database::from_options(self.options.clone())
    }

    pub async fn conn(&self) -> PgConnection {
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

/// Carrega o seed sintético de desenvolvimento.
pub async fn load_seed(db: &TempDb) {
    let seed = include_str!("../../../../db/seeds/dev/0001_demo_ecosystem.sql");
    sqlx::raw_sql(seed).execute(&mut db.conn().await).await.expect("seed de desenvolvimento");
}
