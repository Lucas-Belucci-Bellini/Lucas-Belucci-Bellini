//! O verificador contra um servidor local: o que o relatório do Python não
//! mostra (tipo do erro, tentativas, redirects, tempo) e as respostas que
//! derrubam o `check_websites.py` (achado A22). A paridade com o Python está
//! em `tests/e2e/check_sites_parity.py`.

use std::net::SocketAddr;
use std::time::Duration;

use site_monitor::{Checker, ErrorKind, Options, Status};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Servidor que responde por caminho com bytes crus.
async fn server(respond: fn(&str) -> Option<Vec<u8>>) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("porta local");
    let address = listener.local_addr().expect("endereço");
    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else { return };
            tokio::spawn(async move {
                let path = read_path(&mut stream).await;
                if path == "/slow" {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
                if let Some(bytes) = respond(&path) {
                    let _ = stream.write_all(&bytes).await;
                }
            });
        }
    });
    address
}

async fn read_path(stream: &mut TcpStream) -> String {
    let mut data = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !data.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(n) => data.extend_from_slice(&buffer[..n]),
        }
    }
    String::from_utf8_lossy(&data).split(' ').nth(1).unwrap_or_default().to_string()
}

fn reply(status: u16, extra: &str) -> Option<Vec<u8>> {
    Some(format!("HTTP/1.1 {status} X\r\nContent-Length: 0\r\nConnection: close\r\n{extra}\r\n").into_bytes())
}

fn routes(path: &str) -> Option<Vec<u8>> {
    match path {
        "/ok" | "/slow" => reply(200, ""),
        "/a" => reply(301, "Location: /b\r\n"),
        "/b" => reply(302, "Location: /ok\r\n"),
        "/404" => reply(404, ""),
        "/garbage" => Some(b"isto nao e HTTP\r\n\r\n".to_vec()),
        _ => None,
    }
}

fn checker(retries: u32) -> Checker {
    Checker::new(Options { timeout: Duration::from_millis(1500), retries, max_workers: 4 }).expect("cliente")
}

#[tokio::test]
async fn sucesso_conta_redirects_e_tempo() {
    let address = server(routes).await;
    let check = checker(1).check_website(&format!("http://{address}/a")).await;
    assert_eq!(Status::Verified, check.status);
    assert_eq!((200, 2, 1), (check.http_status, check.redirect_count, check.attempts));
    assert_eq!(format!("http://{address}/ok"), check.final_url);
    assert!(check.response_time_ms.is_some());
    assert_eq!(None, check.error_kind);
}

#[tokio::test]
async fn codigo_de_erro_tenta_de_novo_e_e_classificado() {
    let address = server(routes).await;
    let check = checker(2).check_website(&format!("http://{address}/404")).await;
    assert_eq!((Status::Unreachable, 404, 3), (check.status, check.http_status, check.attempts));
    assert_eq!(Some(ErrorKind::HttpStatus), check.error_kind);
}

#[tokio::test]
async fn conexao_recusada_e_timeout() {
    let address = server(routes).await;
    let closed = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    };
    let checker = checker(0);
    let refused = checker.check_website(&format!("http://{closed}/")).await;
    assert_eq!((Status::Unreachable, 0), (refused.status, refused.http_status));
    assert_eq!(Some(ErrorKind::Connect), refused.error_kind, "{:?}", refused.error_message);
    assert_eq!(format!("http://{closed}/"), refused.final_url, "falha de rede: final_url é a URL original");

    let slow = checker.check_website(&format!("http://{address}/slow")).await;
    assert_eq!(Some(ErrorKind::Timeout), slow.error_kind, "{:?}", slow.error_message);
}

#[tokio::test]
async fn tls_invalido_e_classificado() {
    // Responde texto puro ao ClientHello, sem esperar pedido nenhum.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("porta local");
    let address = listener.local_addr().expect("endereço");
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await;
        }
    });
    let check = checker(0).check_website(&format!("https://{address}/tls")).await;
    assert_eq!((Status::Unreachable, 0), (check.status, check.http_status));
    assert_eq!(Some(ErrorKind::Tls), check.error_kind, "{:?}", check.error_message);
}

#[tokio::test]
async fn o_que_derruba_o_python_aqui_vira_fora_do_ar() {
    let address = server(routes).await;
    let checker = checker(0);

    // Linha de status inválida: BadStatusLine (HTTPException) escapa do Python.
    let garbage = checker.check_website(&format!("http://{address}/garbage")).await;
    assert_eq!((Status::Unreachable, 0), (garbage.status, garbage.http_status));

    // Porta não numérica: InvalidURL antes da rede, também escapa do Python.
    let port = checker.check_website("http://127.0.0.1:abc/").await;
    assert_eq!(Status::Unreachable, port.status);
    assert_eq!(Some("InvalidURL"), port.python_crash);
    assert_eq!(Some(ErrorKind::InvalidUrl), port.error_kind);
}

#[tokio::test]
async fn url_invalida_nao_toca_a_rede() {
    let check = checker(3).check_website("ftp://example.org/x").await;
    assert_eq!((Status::Invalid, 0, 0), (check.status, check.http_status, check.attempts));
    assert_eq!("", check.final_url);
}

#[tokio::test]
async fn urls_repetidas_sao_verificadas_uma_vez() {
    let address = server(routes).await;
    let url = format!("http://{address}/ok");
    let results = checker(0).check_websites([url.clone(), url.clone(), String::new()]).await;
    assert_eq!(1, results.len());
    assert_eq!(Status::Verified, results[&url].status);
}
