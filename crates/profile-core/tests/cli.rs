//! O binário `profile-core` de ponta a ponta: processo de verdade, códigos de
//! saída, saída em texto e JSON, contra um PostgreSQL de verdade.

mod common;

use common::{TempDb, profile_core, text};
use sqlx::Connection;
use sqlx::postgres::PgConnection;
use std::process::Output;

fn states(output: &Output) -> Vec<String> {
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("status --json é JSON");
    parsed["migrations"].as_array().expect("lista").iter().map(|m| m["state"].as_str().unwrap().to_string()).collect()
}

#[test]
fn ajuda_nao_vaza_a_url_do_ambiente() {
    let secret = "postgres://usuario:segredo-da-ajuda@banco.example.org/ecosystem";
    for args in [&["db", "status", "--help"][..], &["db", "migrate", "--help"], &["db", "revert", "--help"]] {
        let output = profile_core(args, Some(secret));
        assert!(output.status.success());
        let all = text(&output.stdout) + &text(&output.stderr);
        assert!(!all.contains("segredo-da-ajuda"), "{all}");
        assert!(all.contains("DATABASE_URL"), "a ajuda cita a variável, sem o valor");
    }
}

#[test]
fn uso_incorreto_sai_com_2() {
    assert_eq!(Some(2), profile_core(&["db", "migrate"], None).status.code(), "sem URL");
    assert_eq!(Some(2), profile_core(&["db", "voar"], Some("postgres://x/y")).status.code());
    let both = profile_core(&["db", "revert", "--to", "3", "--all"], Some("postgres://x/y"));
    assert_eq!(Some(2), both.status.code());
}

#[test]
fn url_invalida_ou_servidor_fora_sai_com_2_sem_repetir_a_senha() {
    let output = profile_core(&["db", "status"], Some("mysql://usuario:segredo-mysql@h/db"));
    assert_eq!(Some(2), output.status.code());
    assert!(text(&output.stderr).contains("DATABASE_URL inválida"));
    assert!(!text(&output.stderr).contains("segredo-mysql"));

    // Porta 1 no loopback: conexão recusada na hora, sem depender de rede.
    let output = profile_core(&["db", "status"], Some("postgres://usuario:segredo-porta@127.0.0.1:1/db"));
    assert_eq!(Some(2), output.status.code());
    let stderr = text(&output.stderr);
    assert!(stderr.contains("não foi possível conectar"), "{stderr}");
    assert!(!stderr.contains("segredo-porta"), "{stderr}");
}

#[tokio::test]
async fn ciclo_completo_com_trava_de_perda_de_dados() {
    let Some(db) = TempDb::create().await else { return };
    let url = Some(db.url.as_str());

    let status = profile_core(&["db", "status", "--json"], url);
    assert!(status.status.success(), "{}", text(&status.stderr));
    assert_eq!(vec!["pending"; 7], states(&status));

    let migrate = profile_core(&["db", "migrate"], url);
    assert!(migrate.status.success(), "{}", text(&migrate.stderr));
    assert_eq!(7, text(&migrate.stdout).lines().filter(|l| l.starts_with("aplicada")).count());
    assert!(text(&profile_core(&["db", "migrate"], url).stdout).starts_with("nada a aplicar"));
    assert_eq!(Some(2), profile_core(&["db", "revert", "--to", "42"], url).status.code(), "alvo inexistente");

    let table = profile_core(&["db", "status"], url);
    assert!(text(&table.stdout).contains("7 migrations: 7 aplicadas, 0 pendentes"), "{}", text(&table.stdout));

    db.load_seed().await;
    let refused = profile_core(&["db", "revert", "--all"], url);
    assert_eq!(Some(1), refused.status.code());
    let stderr = text(&refused.stderr);
    assert!(stderr.contains("apagaria dados") && stderr.contains("--allow-data-loss"), "{stderr}");
    assert_eq!(vec!["applied"; 7], states(&profile_core(&["db", "status", "--json"], url)), "nada mudou");

    let last = profile_core(&["db", "revert"], url);
    assert!(last.status.success(), "{}", text(&last.stderr));
    assert_eq!("revertida  0007  public views\n", text(&last.stdout));

    let all = profile_core(&["db", "revert", "--all", "--allow-data-loss"], url);
    assert!(all.status.success(), "{}", text(&all.stderr));
    assert_eq!(6, text(&all.stdout).lines().count());
    assert_eq!(vec!["pending"; 7], states(&profile_core(&["db", "status", "--json"], url)));
}

#[tokio::test]
async fn status_de_banco_inconsistente_sai_com_1() {
    let Some(db) = TempDb::create().await else { return };
    let url = Some(db.url.as_str());
    assert!(profile_core(&["db", "migrate"], url).status.success());
    let mut conn = PgConnection::connect_with(&db.admin.clone().database(&db.name)).await.unwrap();
    sqlx::query("UPDATE _sqlx_migrations SET checksum = '\\x00' WHERE version = 5").execute(&mut conn).await.unwrap();

    let status = profile_core(&["db", "status"], url);
    assert_eq!(Some(1), status.status.code());
    assert!(text(&status.stdout).contains("modified"), "a tabela sai mesmo assim");
    assert!(text(&status.stderr).contains("migration 5 está modified"));
    let migrate = profile_core(&["db", "migrate"], url);
    assert_eq!(Some(1), migrate.status.code());
    assert!(text(&migrate.stderr).contains("a migration 5 foi alterada depois de aplicada"));
}
