//! `profile-core check sites` de ponta a ponta: o binário de verdade contra um
//! servidor HTTP local e um PostgreSQL descartável. A paridade do relatório
//! com o Python está em `tests/e2e/check_sites_parity.py`; aqui fica o que só
//! o Rust faz (histórico no banco) e os códigos de saída.

mod common;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::{TempDb, profile_core, text};
use sqlx::postgres::PgConnection;

static ROOTS: AtomicUsize = AtomicUsize::new(0);

/// Servidor HTTP bloqueante numa thread: 200 em `/ok`, 404 no resto.
fn server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("porta local");
    let base = format!("http://{}", listener.local_addr().expect("endereço"));
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buffer[..n]),
                }
            }
            let ok = String::from_utf8_lossy(&request).starts_with("GET /ok ");
            let status = if ok { "200 OK" } else { "404 Not Found" };
            let _ = stream
                .write_all(format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes());
        }
    });
    base
}

/// Raiz com `docs/project-catalog.json`, apagada no fim.
struct Root(PathBuf);

impl Root {
    fn new(catalog: Option<&str>) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "check-sites-{}-{}",
            std::process::id(),
            ROOTS.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(dir.join("docs")).expect("docs/");
        if let Some(catalog) = catalog {
            std::fs::write(dir.join("docs/project-catalog.json"), catalog).expect("catálogo");
        }
        Self(dir)
    }

    fn path(&self) -> &str {
        self.0.to_str().expect("caminho UTF-8")
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn sem_banco_e_sem_no_db_recusa_antes_de_verificar() {
    let root = Root::new(Some(r#"{"projects": [{"repository": "o/a", "website": "https://example.org"}]}"#));
    let output = profile_core(&["check", "sites", "--root", root.path()], None);
    assert_eq!(Some(2), output.status.code());
    assert!(text(&output.stderr).contains("defina DATABASE_URL ou passe --no-db"), "{}", text(&output.stderr));
    assert!(output.stdout.is_empty(), "nada foi verificado");
}

#[test]
fn uso_e_catalogo_invalidos_saem_com_2() {
    let root = Root::new(Some("[]"));
    let timeout = profile_core(&["check", "sites", "--no-db", "--timeout", "0", "--root", root.path()], None);
    assert_eq!(Some(2), timeout.status.code());

    let catalog = profile_core(&["check", "sites", "--no-db", "--root", root.path()], None);
    assert_eq!(Some(2), catalog.status.code());
    assert!(text(&catalog.stderr).contains("AttributeError"), "{}", text(&catalog.stderr));
}

#[test]
fn sem_url_nenhuma_diz_isso_e_sai_com_0() {
    let root = Root::new(None);
    let output = profile_core(&["check", "sites", "--no-db", "--fail-on-down", "--root", root.path()], None);
    assert_eq!(Some(0), output.status.code());
    assert_eq!(profile_core::sites::NO_URLS, text(&output.stdout));
}

async fn sync_runs(conn: &mut PgConnection) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM ecosystem.sync_runs WHERE kind = 'websites'")
        .fetch_one(conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn grava_o_historico_dos_sites_registrados() {
    let Some(db) = TempDb::create().await else { return };
    let url = Some(db.url.as_str());
    assert!(profile_core(&["db", "migrate"], url).status.success());
    db.load_seed().await;
    let base = server();
    let mut conn = db.conn().await;
    sqlx::query(
        "INSERT INTO ecosystem.websites (project_id, url, source, is_primary) \
         SELECT id, $1, 'manifest', false FROM ecosystem.projects ORDER BY id LIMIT 1",
    )
    .bind(format!("{base}/ok"))
    .execute(&mut conn)
    .await
    .unwrap();
    let root = Root::new(Some(&format!(
        r#"{{"projects": [{{"repository": "o/registrado", "website": "{base}/ok"}},
                          {{"repository": "o/solto", "website": "{base}/404"}}]}}"#
    )));
    let args = ["check", "sites", "--root", root.path(), "--timeout", "2", "--retries", "0", "--trigger", "test"];

    let output = profile_core(&args, url);
    let stderr = text(&output.stderr);
    assert_eq!(Some(0), output.status.code(), "{stderr}");
    assert!(text(&output.stdout).contains("2 sites verificados · 1 fora do ar"));
    assert!(stderr.contains("1 checagens gravadas") && stderr.contains("1 URLs sem site registrado"), "{stderr}");

    let (outcome, http, attempts, final_url): (String, Option<i16>, i16, Option<String>) = sqlx::query_as(
        "SELECT c.outcome, c.http_status, c.attempts, c.final_url FROM ecosystem.website_checks c \
         JOIN ecosystem.websites w ON w.id = c.website_id WHERE w.url = $1",
    )
    .bind(format!("{base}/ok"))
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(("verified", Some(200), 1, Some(format!("{base}/ok"))), (outcome.as_str(), http, attempts, final_url));
    let (trigger, source, status, seen, changed): (String, String, String, i32, i32) = sqlx::query_as(
        "SELECT trigger, source, status, items_seen, items_changed FROM ecosystem.sync_runs \
         WHERE kind = 'websites' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(("test", "cli", "succeeded", 2, 1), (trigger.as_str(), source.as_str(), status.as_str(), seen, changed));

    // --no-db não grava, mesmo com DATABASE_URL no ambiente.
    let before = sync_runs(&mut conn).await;
    let mut no_db = args.to_vec();
    no_db.push("--no-db");
    assert!(profile_core(&no_db, url).status.success());
    assert_eq!(before, sync_runs(&mut conn).await);

    // --fail-on-down sai com 1 e grava do mesmo jeito.
    let mut fail = args.to_vec();
    fail.push("--fail-on-down");
    assert_eq!(Some(1), profile_core(&fail, url).status.code());
    assert_eq!(before + 1, sync_runs(&mut conn).await);
}
