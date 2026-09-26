//! `profile-core sync github` de ponta a ponta: inventário de arquivo ou de
//! um GitHub local, gravado num PostgreSQL descartável.

mod common;

use common::{FakeGitHub, TempDb, TempTree, profile_core, profile_core_env, text};
use serde_json::{Value, json};
use sqlx::postgres::PgConnection;

fn repo(id: i64, owner: &str, name: &str, extra: Value) -> Value {
    let mut value = json!({
        "id": id,
        "node_id": format!("R_{id}"),
        "name": name,
        "full_name": format!("{owner}/{name}"),
        "owner": {"id": if owner == "me" { 1 } else { 2 }, "login": owner, "type": "User"},
        "description": format!("Descrição de {name}."),
        "private": false,
        "visibility": "public",
        "fork": false,
        "archived": false,
        "is_template": false,
        "default_branch": "main",
        "language": "Rust",
        "homepage": "",
        "topics": ["core"],
        "size": 120,
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2026-09-20T10:00:00Z",
        "pushed_at": "2026-09-20T10:00:00Z",
    });
    for (key, field) in extra.as_object().unwrap() {
        value[key] = field.clone();
    }
    value
}

/// Raiz com o manifesto de exclusões, o inventário e as linguagens.
fn root(repos: &[Value], languages: &[(&str, Value)]) -> TempTree {
    let tree = TempTree::empty();
    std::fs::write(tree.0.join("docs/README_EXCLUDED.json"), r#"{"repositories": ["excluido"]}"#).unwrap();
    std::fs::write(tree.0.join("repos.json"), Value::Array(repos.to_vec()).to_string()).unwrap();
    std::fs::create_dir_all(tree.0.join("languages")).unwrap();
    for (full_name, map) in languages {
        std::fs::write(tree.0.join(format!("languages/{}.json", full_name.replace('/', "__"))), map.to_string())
            .unwrap();
    }
    tree
}

fn inventory() -> Vec<Value> {
    vec![
        repo(101, "me", "alpha", json!({"homepage": "https://alpha.example.org"})),
        repo(102, "me", "segredo", json!({"private": true, "visibility": "private", "language": "Brainfuck"})),
        repo(103, "me", "excluido", json!({})),
        repo(104, "outro", "alpha", json!({"description": "Mini game colaborativo."})),
    ]
}

async fn scalar(conn: &mut PgConnection, sql: &'static str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(conn).await.expect(sql)
}

#[test]
fn gravar_sem_inventario_completo_recusa_antes_da_rede() {
    let tree = root(&[], &[]);
    let output = profile_core_env(
        &["sync", "github", "--root", tree.path()],
        Some("postgres://ninguem@127.0.0.1:9/x"),
        &[("GITHUB_API_URL", "http://127.0.0.1:9")],
    );
    assert_eq!(Some(2), output.status.code());
    assert!(text(&output.stderr).contains("grava o inventário completo"), "{}", text(&output.stderr));
}

#[test]
fn sem_banco_relata_o_inventario_publico() {
    let fake = FakeGitHub::start();
    let public: Vec<Value> = inventory().into_iter().filter(|r| r["private"] == false).collect();
    fake.set("/users/Lucas-Belucci-Bellini/repos?type=owner&per_page=100&page=1", 200, Value::Array(public));
    fake.set("/repos/me/alpha/languages", 200, json!({"Rust": 900, "Shell": 100}));
    fake.set("/repos/outro/alpha/languages", 500, json!({}));
    let tree = root(&[], &[]);
    let output = profile_core_env(
        &["sync", "github", "--root", tree.path(), "--no-db"],
        None,
        &[("GITHUB_API_URL", &fake.url), ("GITHUB_TOKEN", "actions")],
    );
    assert_eq!(Some(0), output.status.code(), "{}", text(&output.stderr));
    let summary: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    assert_eq!(
        json!({"repositories": 2, "public": 2, "private": 0, "excluded": ["me/excluido"], "languages": 2, "complete": false}),
        summary
    );
    let requests = fake.requests.lock().unwrap();
    assert_eq!(None, requests[0].1, "inventário anônimo sem PROFILE_GITHUB_TOKEN");
    assert!(
        requests[1..].iter().all(|(_, auth)| auth.as_deref() == Some("Bearer actions")),
        "linguagens com GITHUB_TOKEN"
    );
    assert!(!requests.iter().any(|(path, _)| path.contains("excluido")), "excluído nem é consultado");
}

#[tokio::test]
async fn grava_renomeia_marca_sumido_e_preserva_o_editorial() {
    let Some(db) = TempDb::create().await else { return };
    let url = Some(db.url.as_str());
    assert!(profile_core(&["db", "migrate"], url).status.success());
    let mut conn = db.conn().await;

    let tree = root(
        &inventory(),
        &[("me/alpha", json!({"Rust": 900, "Shell": 100})), ("me/segredo", json!({"Brainfuck": 5}))],
    );
    let args = [
        "sync",
        "github",
        "--root",
        tree.path(),
        "--input-repos",
        &tree.join("repos.json"),
        "--languages-dir",
        &tree.join("languages"),
        "--trigger",
        "test",
    ];
    let first = profile_core(&args, url);
    assert_eq!(Some(0), first.status.code(), "{}", text(&first.stderr));
    assert!(
        text(&first.stderr).contains("3 repositórios (3 novos, 0 alterados, 0 sumidos)"),
        "{}",
        text(&first.stderr)
    );
    assert_eq!(
        0,
        scalar(&mut conn, "SELECT count(*) FROM ecosystem.repositories WHERE name = 'excluido'").await,
        "D-019"
    );
    assert_eq!(2, scalar(&mut conn, "SELECT count(*) FROM ecosystem.github_owners").await);
    let slugs: Vec<String> =
        sqlx::query_scalar("SELECT slug FROM ecosystem.projects ORDER BY id").fetch_all(&mut conn).await.unwrap();
    assert_eq!(vec!["alpha", "segredo", "outro-alpha"], slugs, "nome repetido ganha o dono no slug");
    let label: String =
        sqlx::query_scalar("SELECT p.label_slug FROM ecosystem.projects p WHERE p.slug = 'outro-alpha'")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!("games", label, "heurística py-classify@1: \"game\" na descrição");
    assert_eq!(1, scalar(&mut conn, "SELECT count(*) FROM ecosystem.languages WHERE name = 'Brainfuck'").await);

    // O dono muda o rótulo à mão; a heurística não pode sobrescrever.
    sqlx::query("UPDATE ecosystem.projects SET label_slug = 'web', label_source = 'editorial', classifier_version = NULL WHERE slug = 'alpha'")
        .execute(&mut conn)
        .await
        .unwrap();

    // Segunda rodada: alpha renomeado (mesmo id), segredo sumiu, linguagem de outro/alpha
    // sem arquivo (fica como estava) e a de me/alpha mudou.
    let mut renamed = inventory();
    renamed[0] = repo(101, "me", "alpha-v2", json!({"homepage": "https://alpha.example.org"}));
    renamed.remove(1);
    let tree = root(&renamed, &[("me/alpha-v2", json!({"Rust": 1000}))]);
    let args = [
        "sync",
        "github",
        "--root",
        tree.path(),
        "--input-repos",
        &tree.join("repos.json"),
        "--languages-dir",
        &tree.join("languages"),
        "--trigger",
        "test",
    ];
    let second = profile_core(&args, url);
    assert_eq!(Some(0), second.status.code(), "{}", text(&second.stderr));
    assert!(text(&second.stderr).contains("(0 novos, 1 alterados, 1 sumidos)"), "{}", text(&second.stderr));
    let (name, full_name): (String, String) =
        sqlx::query_as("SELECT name, full_name FROM ecosystem.repositories WHERE github_id = 101")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!(("alpha-v2".to_string(), "me/alpha-v2".to_string()), (name, full_name));
    assert_eq!(
        1,
        scalar(&mut conn, "SELECT count(*) FROM ecosystem.repositories WHERE github_id = 102 AND gone_at IS NOT NULL")
            .await
    );
    assert_eq!(3, scalar(&mut conn, "SELECT count(*) FROM ecosystem.projects").await, "renomear não cria projeto");
    let editorial: (String, String) =
        sqlx::query_as("SELECT label_slug, label_source FROM ecosystem.projects WHERE slug = 'alpha'")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!(("web".to_string(), "editorial".to_string()), editorial);
    let languages: Vec<(String, i64)> = sqlx::query_as(
        "SELECT l.language, l.bytes FROM ecosystem.repository_languages l JOIN ecosystem.repositories r ON r.id = l.repository_id \
         WHERE r.github_id = 101 ORDER BY 1",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();
    assert_eq!(vec![("Rust".to_string(), 1000)], languages);

    // Terceira rodada igual: nada muda.
    let third = profile_core(&args, url);
    assert!(text(&third.stderr).contains("(0 novos, 0 alterados, 0 sumidos); 0 mapas"), "{}", text(&third.stderr));
    let (status, seen, changed): (String, i32, i32) = sqlx::query_as(
        "SELECT status, items_seen, items_changed FROM ecosystem.sync_runs WHERE kind = 'github_inventory' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(("succeeded".to_string(), 2, 0), (status, seen, changed));
}
