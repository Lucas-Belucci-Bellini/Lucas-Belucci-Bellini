//! `profile-core catalog build` de ponta a ponta, sobre a raiz sintética do
//! teste golden do Python (`tests/fixtures/profile`): mesmos bytes do
//! `update_profile.py`, lendo o inventário de arquivo ou de um GitHub local.

mod common;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use common::{TempTree, profile_core_env, text};

const NOW: &str = "2026-09-25T12:00:00Z";

/// (caminho pedido, cabeçalho Authorization) de cada pedido ao GitHub local.
type RequestLog = Arc<Mutex<Vec<(String, Option<String>)>>>;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/profile")
}

fn expected() -> String {
    std::fs::read_to_string(fixture().join("expected/project-catalog.json")).unwrap()
}

/// GitHub local: responde o inventário do fixture nas duas listagens e
/// registra (linha de pedido, Authorization).
fn github(repos: String) -> (String, RequestLog) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let log = Arc::new(Mutex::new(Vec::new()));
    let seen = log.clone();
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buffer[..n]),
                }
            }
            let request = String::from_utf8_lossy(&request).to_string();
            let path = request.split(' ').nth(1).unwrap_or_default().to_string();
            let auth = request
                .lines()
                .find_map(|l| l.strip_prefix("authorization: ").or_else(|| l.strip_prefix("Authorization: ")))
                .map(str::to_string);
            seen.lock().unwrap().push((path.clone(), auth));
            let body = if path == "/user/repos?affiliation=owner,collaborator,organization_member&per_page=100&page=1"
                || path == "/users/Lucas-Belucci-Bellini/repos?type=owner&per_page=100&page=1"
            {
                repos.clone()
            } else {
                "[]".to_string()
            };
            let _ = stream.write_all(
                format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
                    .as_bytes(),
            );
        }
    });
    (base, log)
}

#[test]
fn sem_write_imprime_o_catalogo_do_python_e_nao_grava() {
    let root = TempTree::copy_of(&fixture().join("input"));
    let args = [
        "catalog",
        "build",
        "--root",
        root.path(),
        "--input-repos",
        &root.join("repos.json"),
        "--site-checks-fixture",
        &root.join("site-checks.json"),
        "--now",
        NOW,
    ];
    let output = profile_core_env(&args, None, &[]);
    assert_eq!(Some(0), output.status.code(), "{}", text(&output.stderr));
    assert_eq!(expected(), text(&output.stdout));
    assert!(!root.0.join("docs/project-catalog.json").exists(), "sem --write nada é gravado");
    assert!(text(&output.stderr).contains("catálogo: 12 projetos (11 públicos, 1 privados), 2 com site no ar"));
}

#[test]
fn write_grava_so_quando_muda() {
    let root = TempTree::copy_of(&fixture().join("input"));
    let out = root.join("checks-out.json");
    let args = [
        "catalog",
        "build",
        "--root",
        root.path(),
        "--input-repos",
        &root.join("repos.json"),
        "--site-checks-fixture",
        &root.join("site-checks.json"),
        "--now",
        NOW,
        "--write",
        "--site-checks-out",
        &out,
    ];
    let first = profile_core_env(&args, None, &[]);
    assert_eq!(Some(0), first.status.code(), "{}", text(&first.stderr));
    assert!(first.stdout.is_empty());
    assert!(text(&first.stderr).ends_with("; gravado\n"));
    assert_eq!(expected(), std::fs::read_to_string(root.0.join("docs/project-catalog.json")).unwrap());

    let second = profile_core_env(&args, None, &[]);
    assert!(text(&second.stderr).ends_with("; sem mudança\n"), "{}", text(&second.stderr));

    // As checagens usadas voltam no formato do fixture, só com as URLs descobertas.
    let used: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let given: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("site-checks.json")).unwrap()).unwrap();
    for (url, check) in used.as_object().unwrap() {
        assert_eq!(given[url]["status"], check["status"], "{url}");
    }
    assert_eq!(4, used.as_object().unwrap().len(), "4 sites declarados no golden");
}

#[test]
fn write_sem_token_recusa_antes_da_rede() {
    let root = TempTree::copy_of(&fixture().join("input"));
    let output = profile_core_env(
        &["catalog", "build", "--root", root.path(), "--skip-site-check", "--write"],
        None,
        &[("GITHUB_API_URL", "http://127.0.0.1:9")],
    );
    assert_eq!(Some(2), output.status.code());
    assert!(text(&output.stderr).contains("refusing write mode without PROFILE_GITHUB_TOKEN"));
}

#[test]
fn inventario_do_github_com_e_sem_token() {
    let repos = std::fs::read_to_string(fixture().join("input/repos.json")).unwrap();
    let (base, log) = github(repos);
    let root = TempTree::copy_of(&fixture().join("input"));
    let args = [
        "catalog",
        "build",
        "--root",
        root.path(),
        "--site-checks-fixture",
        &root.join("site-checks.json"),
        "--now",
        NOW,
    ];
    let with_token = profile_core_env(&args, None, &[("GITHUB_API_URL", &base), ("PROFILE_GITHUB_TOKEN", "tok")]);
    assert_eq!(Some(0), with_token.status.code(), "{}", text(&with_token.stderr));
    assert_eq!(expected(), text(&with_token.stdout));

    let anonymous = profile_core_env(&args, None, &[("GITHUB_API_URL", &base)]);
    assert_eq!(Some(0), anonymous.status.code(), "{}", text(&anonymous.stderr));

    let log = log.lock().unwrap();
    assert_eq!(
        vec![
            (
                "/user/repos?affiliation=owner,collaborator,organization_member&per_page=100&page=1".to_string(),
                Some("Bearer tok".to_string())
            ),
            ("/users/Lucas-Belucci-Bellini/repos?type=owner&per_page=100&page=1".to_string(), None),
        ],
        *log
    );
}

#[test]
fn entrada_invalida_sai_com_2() {
    let root = TempTree::copy_of(&fixture().join("input"));
    std::fs::write(root.0.join("not-array.json"), "{}").unwrap();
    let output = profile_core_env(
        &[
            "catalog",
            "build",
            "--root",
            root.path(),
            "--input-repos",
            &root.join("not-array.json"),
            "--skip-site-check",
        ],
        None,
        &[],
    );
    assert_eq!(Some(2), output.status.code());
    assert!(text(&output.stderr).contains("repository input must be a JSON array"));

    let output = profile_core_env(
        &[
            "catalog",
            "build",
            "--root",
            root.path(),
            "--input-repos",
            &root.join("repos.json"),
            "--skip-site-check",
            "--now",
            "2026-09-25T12:00:00",
        ],
        None,
        &[],
    );
    assert_eq!(Some(2), output.status.code());
    assert!(text(&output.stderr).contains("--now must include a timezone"));
}
