//! `--classifier` (D-039): o padrão é o `py-classify@1` do Python (o golden
//! continua byte a byte); `classifier@2` corrige A6 e muda só o que depende do
//! rótulo — aqui, o `demo-daily-notes` ("diárias" tem "ai" por substring).

mod common;

use std::path::Path;

use common::{TempDb, TempTree, profile_core, text};
use serde_json::Value;

const NOW: &str = "2026-09-25T12:00:00Z";

fn golden() -> TempTree {
    TempTree::copy_of(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/profile/input"))
}

fn catalog_label(stdout: &str, name: &str) -> String {
    let catalog: Value = serde_json::from_str(stdout).expect("catálogo em JSON");
    let project = catalog["projects"].as_array().unwrap().iter().find(|p| p["name"] == name).unwrap();
    project["category_label"].as_str().unwrap().to_string()
}

#[test]
fn padrao_e_o_python_e_classifier_2_corrige_a6() {
    let tree = golden();
    let root = tree.path();
    let args = |classifier: &'static str| {
        vec![
            "catalog".to_string(),
            "build".into(),
            "--root".into(),
            root.to_string(),
            "--input-repos".into(),
            tree.join("repos.json"),
            "--site-checks-fixture".into(),
            tree.join("site-checks.json"),
            "--now".into(),
            NOW.into(),
            "--classifier".into(),
            classifier.into(),
        ]
    };
    let run = |classifier| {
        let args = args(classifier);
        let output = profile_core(&args.iter().map(String::as_str).collect::<Vec<_>>(), None);
        assert!(output.status.success(), "{}", text(&output.stderr));
        text(&output.stdout)
    };
    let v1 = run("py-classify@1");
    let expected = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/profile/expected/project-catalog.json"),
    )
    .unwrap();
    assert_eq!(expected, v1, "py-classify@1 é o golden, byte a byte");
    assert_eq!("IA & Automação", catalog_label(&v1, "demo-daily-notes"), "A6 reproduzido");

    let v2 = run("classifier@2");
    assert_eq!("Software & Ferramentas", catalog_label(&v2, "demo-daily-notes"), "A6 corrigido");
    assert_eq!("Academia", catalog_label(&v2, "demo-java-course"), "palavra inteira continua casando");
    let changed: Vec<String> = ["demo-web-app", "demo-lab", "demo-game", "demo-fork", "demo-baluarte-tool"]
        .into_iter()
        .filter(|name| catalog_label(&v1, name) != catalog_label(&v2, name))
        .map(str::to_string)
        .collect();
    assert!(changed.is_empty(), "só o A6 muda no golden: {changed:?}");

    let unknown = profile_core(&["catalog", "build", "--classifier", "classifier@3", "--skip-site-check"], None);
    assert_eq!(Some(2), unknown.status.code());
}

/// Rótulo e versão gravados para o `demo-daily-notes`.
async fn label(conn: &mut sqlx::postgres::PgConnection) -> (Option<String>, Option<String>) {
    sqlx::query_as(
        "SELECT p.label_slug, p.classifier_version FROM ecosystem.projects p WHERE p.name = 'demo-daily-notes'",
    )
    .fetch_one(conn)
    .await
    .unwrap()
}

#[tokio::test]
async fn sync_github_grava_a_versao_do_rotulo() {
    let Some(db) = TempDb::create().await else { return };
    let url = Some(db.url.as_str());
    assert!(profile_core(&["db", "migrate"], url).status.success());
    let tree = golden();
    // O sync exige id e dono; o rótulo não depende deles.
    let path = tree.0.join("repos.json");
    let mut repos: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for (index, repo) in repos.iter_mut().enumerate() {
        repo["id"] = Value::from(7000 + index);
        let owner = repo["full_name"].as_str().unwrap().split('/').next().unwrap().to_string();
        repo["owner"] = serde_json::json!({"id": if owner == "other-owner" { 2 } else { 1 }, "login": owner});
    }
    std::fs::write(&path, Value::Array(repos).to_string()).unwrap();
    let sync = |classifier: &str| {
        profile_core(
            &[
                "sync",
                "github",
                "--root",
                tree.path(),
                "--input-repos",
                &tree.join("repos.json"),
                "--classifier",
                classifier,
            ],
            url,
        )
    };
    let mut conn = db.conn().await;
    assert!(sync("py-classify@1").status.success());
    assert_eq!((Some("ia-automacao".into()), Some("py-classify@1".into())), label(&mut conn).await);
    let second = sync("classifier@2");
    assert!(second.status.success(), "{}", text(&second.stderr));
    assert_eq!((Some("software-ferramentas".into()), Some("classifier@2".into())), label(&mut conn).await);
}
