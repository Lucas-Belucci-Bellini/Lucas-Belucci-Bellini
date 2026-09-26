//! `profile-core sync commits` de ponta a ponta: o binário contra um GitHub
//! local e um PostgreSQL descartável. A paridade com o `ecosystem_watch.py`
//! está em `parity_commits.rs` (fixture) e em `tests/e2e/github_parity.py`.

mod common;

use common::{FakeGitHub, TempDb, TempTree, profile_core, profile_core_env, text};
use serde_json::{Value, json};
use sqlx::postgres::PgConnection;

const OWNER: &str = "demo-owner";
const A1: &str = "1111111111111111111111111111111111111111";
const A2: &str = "2222222222222222222222222222222222222222";

fn commit(sha: &str, message: &str) -> Value {
    json!([{"sha": sha, "commit": {"committer": {"date": "2026-09-20T10:00:00Z"}, "message": message},
            "html_url": format!("https://github.com/{OWNER}/x/commit/{sha}")}])
}

fn github() -> FakeGitHub {
    let fake = FakeGitHub::start();
    fake.set(
        &format!("/users/{OWNER}/repos?type=owner&per_page=100&page=1"),
        200,
        json!([
            {"name": "aurora-web", "default_branch": "main"},
            {"name": "nimbus-cli", "default_branch": "main"},
            {"name": "fora-do-banco", "default_branch": "main"},
            {"name": "forked", "fork": true},
            {"name": OWNER},
        ]),
    );
    fake.set("/repos/demo-owner/aurora-web/commits?sha=main&per_page=1", 200, commit(A1, "feat: aurora"));
    fake.set(
        "/repos/demo-owner/nimbus-cli/commits?sha=main&per_page=1",
        409,
        json!({"message": "Git Repository is empty."}),
    );
    fake.set("/repos/demo-owner/fora-do-banco/commits?sha=main&per_page=1", 200, json!([]));
    fake
}

fn env(fake: &FakeGitHub) -> [(&'static str, String); 3] {
    [
        ("GITHUB_API_URL", fake.url.clone()),
        ("GH_USER", OWNER.to_string()),
        ("PROFILE_REPOSITORY_NAME", OWNER.to_string()),
    ]
}

fn run(args: &[&str], db: Option<&str>, fake: &FakeGitHub) -> std::process::Output {
    let env = env(fake);
    let pairs: Vec<(&str, &str)> = env.iter().map(|(k, v)| (*k, v.as_str())).collect();
    profile_core_env(args, db, &pairs)
}

fn empty_root() -> TempTree {
    TempTree::empty()
}

#[test]
fn sem_banco_e_sem_no_db_recusa_antes_da_rede() {
    let root = empty_root();
    let output = profile_core(&["sync", "commits", "--root", root.path()], None);
    assert_eq!(Some(2), output.status.code());
    assert!(text(&output.stderr).contains("sync commits grava o histórico no banco"), "{}", text(&output.stderr));
}

#[test]
fn varre_publica_e_a_segunda_varredura_igual_nao_reescreve() {
    let fake = github();
    let root = empty_root();
    let args = ["sync", "commits", "--root", root.path(), "--now", "2026-09-26T07:17:00Z", "--write", "--no-db"];
    let first = run(&args, None, &fake);
    assert_eq!(Some(0), first.status.code(), "{}", text(&first.stderr));
    assert!(first.stdout.is_empty());
    let state: Value =
        serde_json::from_str(&std::fs::read_to_string(root.0.join("docs/ECOSYSTEM-COMMIT-STATE.json")).unwrap())
            .unwrap();
    assert_eq!(
        json!(["aurora-web", "nimbus-cli", "fora-do-banco"]),
        json!(state["repositories"].as_object().unwrap().keys().collect::<Vec<_>>())
    );
    assert_eq!("HTTP Error 409: Conflict", state["repositories"]["nimbus-cli"]["error"]);
    assert_eq!(json!(1539), state["metrics"]["tracked_commits"]);
    assert!(
        std::fs::read_to_string(root.0.join("docs/ECOSYSTEM-COMMIT-MONITOR.md"))
            .unwrap()
            .contains("`2026-09-26T07:17:00Z`")
    );

    let second = run(&args, None, &fake);
    assert_eq!("Nenhuma mudança semântica; snapshot não será reescrito.\n", text(&second.stdout));
    let again: Value =
        serde_json::from_str(&std::fs::read_to_string(root.0.join("docs/ECOSYSTEM-COMMIT-STATE.json")).unwrap())
            .unwrap();
    assert_eq!(state, again);

    // Anônimo sem GITHUB_TOKEN; com ele, Bearer em todas as chamadas.
    assert!(fake.requests.lock().unwrap().iter().all(|(_, auth)| auth.is_none()));
}

#[test]
fn estado_que_nao_e_objeto_sai_com_1_sem_gravar() {
    let fake = github();
    let root = empty_root();
    std::fs::write(root.0.join("docs/ECOSYSTEM-COMMIT-STATE.json"), "[]\n").unwrap();
    let output = run(&["sync", "commits", "--root", root.path(), "--write", "--no-db"], None, &fake);
    assert_eq!(Some(1), output.status.code());
    assert!(text(&output.stderr).contains("AttributeError"), "{}", text(&output.stderr));
    assert!(fake.requests.lock().unwrap().is_empty(), "quebra antes da rede");
    assert_eq!("[]\n", std::fs::read_to_string(root.0.join("docs/ECOSYSTEM-COMMIT-STATE.json")).unwrap());
}

async fn scalar(conn: &mut PgConnection, sql: &'static str) -> i64 {
    sqlx::query_scalar(sql).fetch_one(conn).await.expect(sql)
}

#[tokio::test]
async fn grava_transicoes_e_contadores() {
    let Some(db) = TempDb::create().await else { return };
    let url = Some(db.url.as_str());
    assert!(profile_core(&["db", "migrate"], url).status.success());
    db.load_seed().await;
    let mut conn = db.conn().await;
    let observations_before = scalar(&mut conn, "SELECT count(*) FROM ecosystem.commit_observations").await;

    let fake = github();
    let root = empty_root();
    let args = ["sync", "commits", "--root", root.path(), "--write", "--trigger", "test"];
    let first = run(&args, url, &fake);
    let stderr = text(&first.stderr);
    assert_eq!(Some(0), first.status.code(), "{stderr}");
    assert!(stderr.contains("2 transições gravadas") && stderr.contains("1 repositórios sem registro"), "{stderr}");
    assert_eq!(observations_before + 2, scalar(&mut conn, "SELECT count(*) FROM ecosystem.commit_observations").await);
    let tracked: String = sqlx::query_scalar(
        "SELECT value::text FROM ecosystem.metric_samples WHERE metric_key = 'ecosystem.commits.tracked_total' \
         ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!("1539", tracked);

    // Um commit novo em aurora-web: uma transição, com a contagem.
    fake.set("/repos/demo-owner/aurora-web/commits?sha=main&per_page=1", 200, commit(A2, "fix: aurora"));
    fake.set(&format!("/repos/demo-owner/aurora-web/compare/{A1}...{A2}"), 200, json!({"ahead_by": 4}));
    assert!(run(&args, url, &fake).status.success());
    let (sha, count): (String, Option<i32>) = sqlx::query_as(
        "SELECT o.head_sha, o.commits_since_previous FROM ecosystem.commit_observations o \
         JOIN ecosystem.repositories r ON r.id = o.repository_id WHERE r.name = 'aurora-web' ORDER BY o.id DESC LIMIT 1",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!((A2.to_string(), Some(4)), (sha, count));
    let tracked: String = sqlx::query_scalar(
        "SELECT value::text FROM ecosystem.metric_latest WHERE metric_key = 'ecosystem.commits.tracked_total'",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!("1544", tracked, "1538 + 4 dos projetos + 2 do monitor");

    // Varredura sem mudança: só a linha em sync_runs.
    let samples = scalar(&mut conn, "SELECT count(*) FROM ecosystem.metric_samples").await;
    let observations = scalar(&mut conn, "SELECT count(*) FROM ecosystem.commit_observations").await;
    assert!(run(&args, url, &fake).status.success());
    assert_eq!(samples, scalar(&mut conn, "SELECT count(*) FROM ecosystem.metric_samples").await);
    assert_eq!(observations, scalar(&mut conn, "SELECT count(*) FROM ecosystem.commit_observations").await);
    let (status, changed): (String, i32) = sqlx::query_as(
        "SELECT status, items_changed FROM ecosystem.sync_runs WHERE kind = 'commits' ORDER BY id DESC LIMIT 1",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(("succeeded".to_string(), 0), (status, changed));
}
