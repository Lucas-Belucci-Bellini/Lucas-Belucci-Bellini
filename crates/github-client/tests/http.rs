//! O cliente contra um servidor local: paginação, políticas de nova
//! tentativa, cabeçalhos e o texto dos erros (o `str(exc)` do Python).

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use github_client::{ApiError, Client, Endpoints, Retry, Settings};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type Log = Arc<Mutex<Vec<String>>>;
type Responder = fn(&str, usize) -> Option<Vec<u8>>;

/// Servidor que registra cada pedido cru e responde por (caminho, nº do pedido nesse caminho).
async fn server(respond: Responder) -> (SocketAddr, Log) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("porta local");
    let address = listener.local_addr().expect("endereço");
    let log: Log = Arc::default();
    let shared = log.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else { return };
            let log = shared.clone();
            tokio::spawn(async move {
                let mut data = Vec::new();
                let mut buffer = [0_u8; 4096];
                loop {
                    let head_end = data.windows(4).position(|w| w == b"\r\n\r\n");
                    if let Some(end) = head_end {
                        let head = String::from_utf8_lossy(&data[..end]).to_lowercase();
                        let length = head
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length: "))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        if data.len() >= end + 4 + length {
                            break;
                        }
                    }
                    match stream.read(&mut buffer).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => data.extend_from_slice(&buffer[..n]),
                    }
                }
                let raw = String::from_utf8_lossy(&data).to_string();
                let path = raw.split(' ').nth(1).unwrap_or_default().to_string();
                let seen = {
                    let mut log = log.lock().unwrap();
                    log.push(raw);
                    log.iter().filter(|r| r.split(' ').nth(1) == Some(path.as_str())).count()
                };
                match respond(&path, seen) {
                    Some(bytes) => {
                        let _ = stream.write_all(&bytes).await;
                    }
                    // Sem resposta: "/hang" segura a conexão aberta; o resto fecha.
                    None if path == "/hang" => tokio::time::sleep(Duration::from_secs(5)).await,
                    None => {}
                }
            });
        }
    });
    (address, log)
}

fn reply(status: &str, body: &str, extra: &str) -> Option<Vec<u8>> {
    Some(
        format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{body}", body.len())
            .into_bytes(),
    )
}

fn routes(path: &str, seen: usize) -> Option<Vec<u8>> {
    match path {
        p if p.starts_with("/users/o/repos") && p.ends_with("page=1") => {
            let page: Vec<Value> = (0..100).map(|i| json!({"name": format!("r{i}")})).collect();
            reply("200 OK", &Value::Array(page).to_string(), "")
        }
        p if p.starts_with("/users/o/repos") && p.ends_with("page=2") => reply("200 OK", r#"[{"name":"last"}]"#, ""),
        "/flaky" if seen == 1 => reply("503 Service Unavailable", "{}", "Retry-After: 0\r\n"),
        "/flaky" => reply("200 OK", r#"{"ok":true}"#, ""),
        "/limited" => reply("403 Forbidden", "{}", "X-RateLimit-Remaining: 0\r\nRetry-After: 0\r\n"),
        "/forbidden" => reply("403 Forbidden", "{}", "X-RateLimit-Remaining: 17\r\n"),
        "/empty-repo" => reply("409 Conflict", r#"{"message":"Git Repository is empty."}"#, ""),
        "/html" => reply("200 OK", "<html>", ""),
        "/graphql" => reply("200 OK", r#"{"data":{"user":null},"errors":[{"message":"x"}]}"#, ""),
        "/hang" | "/closed" => None,
        _ => reply("404 Not Found", r#"{"message":"Not Found"}"#, ""),
    }
}

fn client(address: SocketAddr, user_agent: &'static str, api_version_header: bool) -> Client {
    Client::new(Settings {
        user_agent,
        api_version_header,
        endpoints: Endpoints { api_url: format!("http://{address}"), graphql_url: format!("http://{address}/graphql") },
        timeout: Duration::from_millis(800),
        delay_unit: Duration::from_millis(1),
    })
    .expect("cliente")
}

fn requests_to(log: &Log, path: &str) -> usize {
    log.lock().unwrap().iter().filter(|r| r.split(' ').nth(1) == Some(path)).count()
}

#[tokio::test]
async fn pagina_cheia_pede_a_proxima() {
    let (address, log) = server(routes).await;
    let items = client(address, "t", false)
        .get_pages(|page| format!("/users/o/repos?type=owner&per_page=100&page={page}"), Retry::Never)
        .await
        .unwrap();
    assert_eq!(101, items.len());
    assert_eq!(json!({"name": "last"}), items[100]);
    assert_eq!(2, log.lock().unwrap().len());
}

#[tokio::test]
async fn politica_do_monitor_tenta_de_novo_e_a_do_gerador_nao() {
    let (address, log) = server(routes).await;
    let watch = client(address, "t", true);
    assert_eq!(json!({"ok": true}), watch.get_json("/flaky", Retry::Watch).await.unwrap());
    assert_eq!(2, requests_to(&log, "/flaky"));

    let limited = watch.get_json("/limited", Retry::Watch).await.unwrap_err();
    assert_eq!("HTTP Error 403: Forbidden", limited.to_string());
    assert_eq!(4, requests_to(&log, "/limited"), "rate limit: 4 tentativas");

    assert_eq!(Some(403), watch.get_json("/forbidden", Retry::Watch).await.unwrap_err().status());
    assert_eq!(1, requests_to(&log, "/forbidden"), "403 comum não tenta de novo");

    let empty = watch.get_json("/empty-repo", Retry::Watch).await.unwrap_err();
    assert_eq!("HTTP Error 409: Conflict", empty.to_string());
    assert_eq!(1, requests_to(&log, "/empty-repo"));

    let (address, log) = server(routes).await;
    let once = client(address, "t", false);
    assert!(once.get_json("/flaky", Retry::Never).await.is_err());
    assert_eq!(1, requests_to(&log, "/flaky"));
}

#[tokio::test]
async fn cabecalhos_e_token() {
    let (address, log) = server(routes).await;
    let anonymous = client(address, "profile-ecosystem-watch", true);
    let _ = anonymous.get_json("/x", Retry::Never).await;
    let _ = anonymous.with_token(Some("s3cr3t".into())).get_json("/y", Retry::Never).await;
    let _ = anonymous.with_token(Some(String::new())).get_json("/z", Retry::Never).await;
    let log = log.lock().unwrap();
    let lower: Vec<String> = log.iter().map(|r| r.to_lowercase()).collect();
    assert!(lower.iter().all(|r| r.contains("user-agent: profile-ecosystem-watch\r\n")));
    assert!(lower.iter().all(|r| r.contains("accept: application/vnd.github+json\r\n")));
    assert!(lower.iter().all(|r| r.contains("x-github-api-version: 2022-11-28\r\n")));
    assert!(!lower[0].contains("authorization"));
    assert!(log[1].contains("authorization: Bearer s3cr3t\r\n"));
    assert!(!lower[2].contains("authorization"), "token vazio é anônimo");
}

#[tokio::test]
async fn erros_de_transporte_com_o_texto_do_urllib() {
    let closed = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let refused = client(closed, "t", false).get_json("/x", Retry::Watch).await.unwrap_err();
    assert_eq!("<urlopen error [Errno 111] Connection refused>", refused.to_string());

    let (address, log) = server(routes).await;
    let client = client(address, "t", false);
    assert_eq!(ApiError::Timeout, client.get_json("/hang", Retry::Never).await.unwrap_err());
    let closed = client.get_json("/closed", Retry::Watch).await.unwrap_err();
    assert_eq!("Remote end closed connection without response", closed.to_string());
    assert_eq!(1, requests_to(&log, "/closed"), "RemoteDisconnected escapa sem nova tentativa");
    assert!(matches!(client.get_json("/html", Retry::Watch).await, Err(ApiError::Json(_))));
    assert_eq!(1, requests_to(&log, "/html"), "JSON inválido não tenta de novo");
}

#[tokio::test]
async fn graphql_devolve_o_corpo_com_errors() {
    let (address, log) = server(routes).await;
    let body = client(address, "t", false)
        .with_token(Some("tok".into()))
        .graphql("query { x }", json!({"login": "o", "from": "2026-01-01T00:00:00Z"}))
        .await
        .unwrap();
    assert_eq!(json!([{"message": "x"}]), body["errors"]);
    let log = log.lock().unwrap();
    let request = &log[0];
    assert!(request.starts_with("POST /graphql "));
    assert!(request.to_lowercase().contains("content-type: application/json\r\n"));
    let sent: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(json!({"query": "query { x }", "variables": {"login": "o", "from": "2026-01-01T00:00:00Z"}}), sent);
}
