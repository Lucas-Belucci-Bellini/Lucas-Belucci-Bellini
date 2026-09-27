//! `--from-db` (Fase 5, D-037): o README e o catálogo gerados do banco são
//! os mesmos gerados do inventário em arquivo, quando o banco recebeu esse
//! inventário (`sync github`), os manifestos (`import manifests`) e as mesmas
//! checagens de site.
//!
//! O caminho por arquivo já é igual ao Python (golden, fixture de paridade e
//! o GitHub simulado); este teste fecha o elo banco = arquivo. A árvore é a do
//! golden com dois repositórios empatados e um mapa de linguagens fora da
//! ordem de bytes — sem a ordem gravada pela migration 0008 o README muda,
//! e o teste confere isso também.

mod common;

use std::path::Path;

use common::{TempDb, TempTree, profile_core, text};
use serde_json::{Value, json};
use store::{Database, RunContext, WebsiteCheckRecord};

const NOW: &str = "2026-09-25T12:00:00Z";

fn golden() -> TempTree {
    let tree = TempTree::copy_of(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/profile/input"));
    // O banco só aceita as categorias do arsenal da migration 0006; o arquivo
    // é o mesmo para os dois lados.
    let stack = tree.0.join("docs/README_STACK.json");
    let text = std::fs::read_to_string(&stack).unwrap().replace("Ferramentas Extras", "Hardware & Simulação");
    std::fs::write(stack, text).unwrap();

    // O sync github exige id e dono; o gerador ignora os dois.
    let path = tree.0.join("repos.json");
    let mut repos: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    // Empatados em tudo; na listagem, "zz" vem antes de "aa".
    for name in ["zz-empate", "aa-empate"] {
        repos.insert(
            3,
            json!({"name": name, "full_name": format!("Lucas-Belucci-Bellini/{name}"),
                   "description": "Empate de propósito: mesma data, mesmo tamanho de descrição.",
                   "private": false, "fork": false, "archived": false, "homepage": null,
                   "pushed_at": "2026-09-19T10:00:00Z", "updated_at": "2026-09-19T10:00:00Z", "size": 77}),
        );
    }
    for (index, repo) in repos.iter_mut().enumerate() {
        let owner = repo["full_name"].as_str().unwrap().split('/').next().unwrap().to_string();
        repo["id"] = json!(9000 + index);
        repo["owner"] = json!({"id": if owner == "other-owner" { 2 } else { 1 }, "login": owner, "type": "User"});
        repo["visibility"] = json!(if repo["private"] == true { "private" } else { "public" });
    }
    std::fs::write(&path, Value::Array(repos).to_string()).unwrap();
    // Linguagens fora da ordem de bytes: a coluna de stack mostra as cinco
    // primeiras na ordem da API.
    std::fs::write(
        tree.0.join("languages/Lucas-Belucci-Bellini__zz-empate.json"),
        r#"{"Rust": 10, "Python": 5000, "Go": 7, "C": 1, "Zig": 300, "Nim": 2}"#,
    )
    .unwrap();
    tree
}

/// As checagens do fixture, gravadas como se o monitor as tivesse feito.
async fn record_checks(url: &str, tree: &TempTree) {
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(tree.0.join("site-checks.json")).unwrap()).unwrap();
    let records: Vec<WebsiteCheckRecord> = fixture
        .as_object()
        .unwrap()
        .iter()
        .map(|(site, check)| {
            let verified = check["status"] == "verified";
            let http = check["http_status"].as_i64().filter(|code| (100..=599).contains(code));
            WebsiteCheckRecord {
                url: site.clone(),
                checked_at: "2026-09-25T11:00:00+00:00".into(),
                outcome: check["status"].as_str().unwrap().into(),
                http_status: http.map(|code| code as i16),
                final_url: check["final_url"].as_str().map(str::to_string),
                redirect_count: 0,
                response_time_ms: None,
                attempts: 1,
                error_kind: (!verified).then(|| if http.is_some() { "http_status" } else { "connect" }.to_string()),
                error_message: None,
            }
        })
        .collect();
    let run = RunContext { trigger: "test".into(), source: "test".into(), code_version: None, external_ref: None };
    let report = Database::from_url(url).unwrap().record_website_checks(&run, &records).await.unwrap();
    assert!(report.unregistered.is_empty(), "sites sem registro: {:?}", report.unregistered);
}

fn read(dir: &Path, file: &str) -> String {
    std::fs::read_to_string(dir.join(file)).unwrap_or_else(|error| panic!("{file}: {error}"))
}

#[tokio::test]
async fn readme_e_catalogo_do_banco_iguais_aos_do_arquivo() {
    let Some(db) = TempDb::create().await else { return };
    let url = Some(db.url.as_str());
    let tree = golden();
    let root = tree.path();
    let (repos, languages) = (tree.join("repos.json"), tree.join("languages"));

    for args in [
        vec!["db", "migrate"],
        vec!["sync", "github", "--root", root, "--input-repos", &repos, "--languages-dir", &languages],
        vec!["import", "manifests", "--root", root],
    ] {
        let output = profile_core(&args, url);
        assert!(output.status.success(), "{args:?}: {}", text(&output.stderr));
    }
    record_checks(&db.url, &tree).await;

    let (from_files, from_db) = (tree.join("out-files"), tree.join("out-db"));
    let files = profile_core(
        &[
            "render",
            "readme",
            "--root",
            root,
            "--input-repos",
            &repos,
            "--languages-dir",
            &languages,
            "--site-checks-fixture",
            &tree.join("site-checks.json"),
            "--now",
            NOW,
            "--out-dir",
            &from_files,
        ],
        None,
    );
    assert!(files.status.success(), "{}", text(&files.stderr));
    let database =
        profile_core(&["render", "readme", "--root", root, "--from-db", "--now", NOW, "--out-dir", &from_db], url);
    assert!(database.status.success(), "{}", text(&database.stderr));
    assert!(!text(&database.stderr).contains("sem checagem"), "{}", text(&database.stderr));

    assert_eq!(text(&files.stdout), text(&database.stdout), "saída padrão");
    for file in ["README.md", "profile-snapshot.svg", "project-catalog.json"] {
        let (expected, actual) = (read(Path::new(&from_files), file), read(Path::new(&from_db), file));
        if expected != actual {
            let line = expected.lines().zip(actual.lines()).position(|(e, a)| e != a).unwrap_or(0);
            panic!(
                "{file} difere na linha {}:\n arquivo: {:?}\n banco:   {:?}",
                line + 1,
                expected.lines().nth(line),
                actual.lines().nth(line)
            );
        }
    }
    let readme = read(Path::new(&from_db), "README.md");
    assert!(readme.contains("`Rust` `Python` `Go` `C` `Zig`"), "a ordem da API, não a de bytes");

    let catalog_files = profile_core(
        &[
            "catalog",
            "build",
            "--root",
            root,
            "--input-repos",
            &repos,
            "--site-checks-fixture",
            &tree.join("site-checks.json"),
            "--now",
            NOW,
        ],
        None,
    );
    let catalog_db = profile_core(&["catalog", "build", "--root", root, "--from-db", "--now", NOW], url);
    assert!(catalog_db.status.success(), "{}", text(&catalog_db.stderr));
    assert_eq!(text(&catalog_files.stdout), text(&catalog_db.stdout), "catálogo");

    // Sem a ordem gravada, os empates e a coluna de stack mudam: o teste
    // acima só passa porque a 0008 guarda a ordem.
    let mut conn = db.conn().await;
    sqlx::raw_sql(
        "UPDATE ecosystem.repositories SET inventory_position = NULL; \
         UPDATE ecosystem.repository_languages SET position = NULL;",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    let unordered = tree.join("out-unordered");
    let output =
        profile_core(&["render", "readme", "--root", root, "--from-db", "--now", NOW, "--out-dir", &unordered], url);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let unordered = read(Path::new(&unordered), "README.md");
    assert_ne!(readme, unordered, "sem ordem gravada o README teria de mudar");
    assert!(unordered.contains("`Python` `Zig` `Rust` `Go` `Nim`"), "sem posição: bytes, depois nome");

    // Um sync de novo regrava a ordem.
    let output =
        profile_core(&["sync", "github", "--root", root, "--input-repos", &repos, "--languages-dir", &languages], url);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let again = tree.join("out-again");
    let output =
        profile_core(&["render", "readme", "--root", root, "--from-db", "--now", NOW, "--out-dir", &again], url);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(readme, read(Path::new(&again), "README.md"));
}

#[tokio::test]
async fn site_sem_checagem_no_banco_conta_como_fora_do_ar_e_avisa() {
    let Some(db) = TempDb::create().await else { return };
    let url = Some(db.url.as_str());
    let tree = golden();
    let root = tree.path();
    let (repos, languages) = (tree.join("repos.json"), tree.join("languages"));
    for args in [
        vec!["db", "migrate"],
        vec!["sync", "github", "--root", root, "--input-repos", &repos, "--languages-dir", &languages],
    ] {
        assert!(profile_core(&args, url).status.success());
    }
    let output = profile_core(&["render", "readme", "--root", root, "--from-db", "--now", NOW], url);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(text(&output.stderr).contains("sem checagem no banco"), "{}", text(&output.stderr));
    assert!(!text(&output.stdout).contains("\"verified_sites\": 1"), "nenhum site verificado sem checagem");
}

#[test]
fn from_db_exige_a_url_e_nao_combina_com_inventario_de_arquivo() {
    let output = profile_core(&["render", "readme", "--from-db", "--skip-site-check"], None);
    assert_eq!(Some(2), output.status.code());
    assert!(text(&output.stderr).contains("--from-db lê o inventário do banco"), "{}", text(&output.stderr));
    let output = profile_core(&["catalog", "build", "--from-db", "--input-repos", "x.json"], Some("postgres://x/y"));
    assert_eq!(Some(2), output.status.code());
}
