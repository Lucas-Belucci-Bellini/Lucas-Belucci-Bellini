//! `profile-core sync contributions` de ponta a ponta: o binário contra um
//! GraphQL local e um PostgreSQL descartável. A paridade com o Python está
//! em `parity_contributions.rs`.

mod common;

use common::{FakeGitHub, TempDb, TempTree, profile_core, profile_core_env, text};
use serde_json::{Value, json};

const NOW: &str = "2026-09-25T12:00:00Z";

fn collection(commits: i64) -> Value {
    json!({"data": {"user": {"contributionsCollection": {
        "contributionCalendar": {"totalContributions": commits + 5},
        "totalCommitContributions": commits,
        "totalIssueContributions": 1,
        "totalPullRequestContributions": 2,
        "totalPullRequestReviewContributions": 0,
        "totalRepositoryContributions": 1,
        "restrictedContributionsCount": 1,
    }}}})
}

fn run(fake: &FakeGitHub, root: &TempTree, extra: &[&str], db: Option<&str>) -> std::process::Output {
    let mut args = vec!["sync", "contributions", "--root", root.path(), "--now", NOW, "--trigger", "test"];
    args.extend_from_slice(extra);
    let graphql = format!("{}/graphql", fake.url);
    profile_core_env(&args, db, &[("GITHUB_GRAPHQL_URL", &graphql), ("PROFILE_README_TOKEN", "tok")])
}

#[test]
fn sem_token_sai_com_2() {
    let root = TempTree::empty();
    let output = profile_core(&["sync", "contributions", "--root", root.path(), "--no-db"], None);
    assert_eq!(Some(2), output.status.code());
    assert!(text(&output.stderr).contains("PROFILE_README_TOKEN ou GITHUB_TOKEN ausente"));
}

#[test]
fn grava_o_json_e_a_pagina() {
    let fake = FakeGitHub::start();
    fake.set("/graphql", 200, collection(9));
    let root = TempTree::empty();
    let output = run(&fake, &root, &["--write", "--no-db"], None);
    assert_eq!(Some(0), output.status.code(), "{}", text(&output.stderr));
    assert_eq!("timeline updated: rows=13 html=docs/assets/contributions-timeline.html\n", text(&output.stdout));
    let data: Value = serde_json::from_str(
        &std::fs::read_to_string(root.0.join("docs/assets/contributions-timeline-data.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(("2025-09-25", 13), (data["window_start"].as_str().unwrap(), data["rows"].as_array().unwrap().len()));
    assert!(
        std::fs::read_to_string(root.0.join("docs/assets/contributions-timeline.html"))
            .unwrap()
            .contains("Plotly.newPlot")
    );
    assert!(
        fake.requests
            .lock()
            .unwrap()
            .iter()
            .all(|(path, auth)| path == "/graphql" && auth.as_deref() == Some("Bearer tok"))
    );

    fake.set("/graphql", 502, json!({"message": "Bad Gateway"}));
    let failed = run(&fake, &root, &["--no-db"], None);
    assert_eq!(Some(2), failed.status.code());
    assert!(text(&failed.stderr).contains("timeline generation failed: GitHub GraphQL HTTP 502"));
}

#[tokio::test]
async fn banco_recebe_so_o_que_mudou() {
    let Some(db) = TempDb::create().await else { return };
    let url = Some(db.url.as_str());
    assert!(profile_core(&["db", "migrate"], url).status.success());
    let fake = FakeGitHub::start();
    fake.set("/graphql", 200, collection(9));
    let root = TempTree::empty();

    let first = run(&fake, &root, &[], url);
    assert!(text(&first.stderr).contains("91 de 91 amostras"), "{}", text(&first.stderr));
    let second = run(&fake, &root, &[], url);
    assert!(text(&second.stderr).contains("0 de 91 amostras"), "{}", text(&second.stderr));

    fake.set("/graphql", 200, collection(10));
    let third = run(&fake, &root, &[], url);
    assert!(
        text(&third.stderr).contains("26 de 91 amostras"),
        "commits e total de 13 janelas: {}",
        text(&third.stderr)
    );

    let mut conn = db.conn().await;
    let (value, start, end): (String, String, String) = sqlx::query_as(
        "SELECT value::text, to_char(window_start AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS'), \
                to_char(window_end AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS') \
         FROM ecosystem.metric_latest WHERE metric_key = 'profile.contributions.commits' ORDER BY window_start LIMIT 1",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(
        ("10".to_string(), "2025-09-25 00:00:00".to_string(), "2025-09-30 23:59:59".to_string()),
        (value, start, end)
    );
}
