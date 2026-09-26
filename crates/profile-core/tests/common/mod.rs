//! Apoio dos testes de ponta a ponta do binário: executa o `profile-core` e
//! cria um banco descartável por teste a partir de `STORE_TEST_DATABASE_URL`
//! (sem URL os testes de banco pulam; com `STORE_TESTS_REQUIRED=1`, reprovam).

#![allow(dead_code)]

use std::process::{Command, Output};
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};

use sqlx::postgres::{PgConnectOptions, PgConnection};
use sqlx::{AssertSqlSafe, Connection};

pub const BIN: &str = env!("CARGO_BIN_EXE_profile-core");
static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Variáveis do ambiente que mudam o que o binário faz: proveniência gravada,
/// tokens e o endereço da API do GitHub. Os testes não as herdam (no GitHub
/// Actions elas existem, localmente não); quem precisa passa explicitamente.
const INHERITED: [&str; 11] = [
    "GH_USER",
    "PROFILE_REPOSITORY_NAME",
    "PROFILE_LOGIN",
    "GITHUB_WORKFLOW",
    "GITHUB_SHA",
    "GITHUB_RUN_ID",
    "GITHUB_API_URL",
    "GITHUB_GRAPHQL_URL",
    "GITHUB_TOKEN",
    "PROFILE_GITHUB_TOKEN",
    "PROFILE_README_TOKEN",
];

pub fn profile_core(args: &[&str], database_url: Option<&str>) -> Output {
    profile_core_env(args, database_url, &[])
}

/// Como [`profile_core`], com variáveis de ambiente extras.
pub fn profile_core_env(args: &[&str], database_url: Option<&str>, env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(BIN);
    command.args(args).env_remove("DATABASE_URL");
    for name in INHERITED {
        command.env_remove(name);
    }
    if let Some(url) = database_url {
        command.env("DATABASE_URL", url);
    }
    command.envs(env.iter().copied());
    command.output().expect("executa profile-core")
}

pub fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Cópia de um diretório numa pasta temporária, apagada no `Drop`.
pub struct TempTree(pub std::path::PathBuf);

impl TempTree {
    pub fn copy_of(source: &std::path::Path) -> Self {
        let target = std::env::temp_dir().join(format!(
            "profile-core-tree-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        copy_dir(source, &target);
        Self(target)
    }

    /// Uma raiz vazia, só com `docs/`.
    pub fn empty() -> Self {
        let target = std::env::temp_dir().join(format!(
            "profile-core-tree-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(target.join("docs")).expect("docs/");
        Self(target)
    }

    pub fn path(&self) -> &str {
        self.0.to_str().expect("caminho UTF-8")
    }

    pub fn join(&self, relative: &str) -> String {
        self.0.join(relative).to_str().expect("caminho UTF-8").to_string()
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn copy_dir(source: &std::path::Path, target: &std::path::Path) {
    std::fs::create_dir_all(target).expect("diretório de destino");
    for entry in std::fs::read_dir(source).expect("lê diretório") {
        let entry = entry.expect("entrada");
        let destination = target.join(entry.file_name());
        if entry.file_type().expect("tipo").is_dir() {
            copy_dir(&entry.path(), &destination);
        } else {
            std::fs::copy(entry.path(), destination).expect("copia arquivo");
        }
    }
}

pub struct TempDb {
    pub admin: PgConnectOptions,
    pub name: String,
    pub url: String,
}

impl TempDb {
    pub async fn create() -> Option<Self> {
        let Ok(url) = std::env::var("STORE_TEST_DATABASE_URL").or_else(|_| std::env::var("DATABASE_URL")) else {
            assert!(std::env::var("STORE_TESTS_REQUIRED").is_err(), "STORE_TESTS_REQUIRED sem URL de teste");
            eprintln!("pulado: defina STORE_TEST_DATABASE_URL para rodar os testes de banco");
            return None;
        };
        let admin = PgConnectOptions::from_str(&url).expect("URL de teste válida");
        let name = format!("cli_test_{}_{}", std::process::id(), COUNTER.fetch_add(1, Ordering::SeqCst));
        let mut conn = PgConnection::connect_with(&admin).await.expect("conexão administrativa");
        sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut conn)
            .await
            .expect("CREATE DATABASE");
        // Troca só o nome do banco no fim da URL (sem parâmetros de consulta nos testes).
        let base = url.rsplit_once('/').expect("URL com /banco").0;
        Some(Self { admin, url: format!("{base}/{name}"), name })
    }

    pub async fn load_seed(&self) {
        let seed = include_str!("../../../../db/seeds/dev/0001_demo_ecosystem.sql");
        let mut conn = PgConnection::connect_with(&self.admin.clone().database(&self.name)).await.expect("conexão");
        sqlx::raw_sql(seed).execute(&mut conn).await.expect("seed");
    }

    pub async fn conn(&self) -> PgConnection {
        PgConnection::connect_with(&self.admin.clone().database(&self.name)).await.expect("conexão")
    }
}

impl Drop for TempDb {
    fn drop(&mut self) {
        let (admin, name) = (self.admin.clone(), self.name.clone());
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("runtime");
            runtime.block_on(async {
                if let Ok(mut conn) = PgConnection::connect_with(&admin).await {
                    let drop = format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)");
                    let _ = sqlx::query(AssertSqlSafe(drop)).execute(&mut conn).await;
                }
            });
        })
        .join()
        .ok();
    }
}

/// Rotas do GitHub local: caminho com query → (status, corpo).
pub type Routes = std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, (u16, String)>>>;
/// Pedidos recebidos: (caminho com query, Authorization).
pub type Requests = std::sync::Arc<std::sync::Mutex<Vec<(String, Option<String>)>>>;

/// GitHub local para os testes do binário: rotas (caminho com query → status
/// e corpo) que o teste pode trocar entre execuções, e o registro de cada
/// pedido (caminho, Authorization).
pub struct FakeGitHub {
    pub url: String,
    pub routes: Routes,
    pub requests: Requests,
}

impl FakeGitHub {
    pub fn start() -> Self {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("porta local");
        let url = format!("http://{}", listener.local_addr().expect("endereço"));
        let routes: Routes = Routes::default();
        let requests: Requests = Requests::default();
        let (table, log) = (routes.clone(), requests.clone());
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
                let auth = request.lines().find_map(|line| {
                    let (name, value) = line.split_once(": ")?;
                    name.eq_ignore_ascii_case("authorization").then(|| value.to_string())
                });
                log.lock().unwrap().push((path.clone(), auth));
                let (status, body) = table
                    .lock()
                    .unwrap()
                    .get(&path)
                    .cloned()
                    .unwrap_or((404, r#"{"message":"Not Found"}"#.to_string()));
                let reason = match status {
                    200 => "OK",
                    404 => "Not Found",
                    409 => "Conflict",
                    500 => "Internal Server Error",
                    _ => "Status",
                };
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                );
            }
        });
        Self { url, routes, requests }
    }

    pub fn set(&self, path: &str, status: u16, body: serde_json::Value) {
        self.routes.lock().unwrap().insert(path.to_string(), (status, body.to_string()));
    }
}
