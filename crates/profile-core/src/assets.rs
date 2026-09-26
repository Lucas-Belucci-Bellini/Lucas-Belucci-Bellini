//! `profile-core render lang-stats | cards | assets` — o
//! `.github/scripts/lang_stats.py` e o `.github/scripts/profile_cards.py`.
//!
//! - `lang-stats`: lista os repositórios (com `GITHUB_TOKEN`, `/user/repos`,
//!   caindo no endpoint público se ele recusar), tira os excluídos, soma as
//!   linguagens e conta os tipos de arquivo pela árvore git do branch padrão.
//!   Grava `assets/lang-stats.svg` e `assets/profile-top-langs.svg` só quando
//!   algo além do carimbo de hora mudou.
//! - `cards`: uma consulta GraphQL da janela de 365 dias; grava os quatro
//!   cards sempre.
//! - `assets`: os dois, nessa ordem, parando no primeiro que falhar — o que
//!   o passo do `lang-stats.yml` faz.
//!
//! A saída padrão é a dos scripts, byte a byte; os avisos vão para o stderr.
//! Onde o Python sai com traceback (código 1), aqui sai [`AssetsError::Crash`],
//! também com 1. As variáveis de ambiente são as mesmas: `GITHUB_TOKEN`,
//! `GH_USER`, `INCLUDE_FORKS=1`, `SEM_ARQUIVOS=1`, `GITHUB_API_URL` e
//! `GITHUB_GRAPHQL_URL`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{Datelike, NaiveDateTime, TimeDelta};
use ecosystem_domain::discovery::{json_truthy, py_str};
use github_client::{ApiError, Client, Retry, Settings};
use profile_render::cards;
use profile_render::lang_stats::{self, LangStats};
use profile_render::pytext::py_dumps;
use serde_json::{Map, Value, json};

/// `User-Agent` do `lang_stats.py`.
pub const LANG_STATS_AGENT: &str = "baluarte-lang-bot";
/// `User-Agent` do `profile_cards.py`.
pub const CARDS_AGENT: &str = "Lucas-Belucci-Bellini-profile-cards";
/// `urlopen(..., timeout=45)` do `lang_stats.py`.
pub const LANG_STATS_TIMEOUT: Duration = Duration::from_secs(45);

/// A consulta do `profile_cards.py`, igual.
pub const CARDS_QUERY: &str = r#"
query($login:String!, $from:DateTime!, $to:DateTime!) {
  user(login:$login) {
    login
    name
    contributionsCollection(from:$from, to:$to) {
      totalCommitContributions
      totalIssueContributions
      totalPullRequestContributions
      totalPullRequestReviewContributions
      totalRepositoryContributions
      restrictedContributionsCount
      contributionCalendar { totalContributions }
    }
    repositories(first:1, privacy:PUBLIC, ownerAffiliations:OWNER) { totalCount }
  }
}
"#;

/// Falhas.
#[derive(Debug, thiserror::Error)]
pub enum AssetsError {
    /// Onde o Python sairia com traceback (código 1).
    #[error("{0}")]
    Crash(String),
    /// O cliente HTTP não montou.
    #[error(transparent)]
    Client(#[from] github_client::BuildError),
}

fn crash(message: impl Into<String>) -> AssetsError {
    AssetsError::Crash(message.into())
}

/// O que o comando recebeu (as variáveis de ambiente já lidas).
#[derive(Debug, Clone)]
pub struct Options {
    /// Raiz com `docs/README_EXCLUDED.json` e `assets/`.
    pub root: PathBuf,
    /// Relógio fixo, com fuso.
    pub now: Option<String>,
    /// `GITHUB_TOKEN`.
    pub token: Option<String>,
    /// `GH_USER`.
    pub user: String,
    /// `INCLUDE_FORKS=1`.
    pub include_forks: bool,
    /// `SEM_ARQUIVOS=1`: pula as árvores git.
    pub skip_files: bool,
}

/// Resultado: a saída padrão do script e o código de saída.
#[derive(Debug, Default)]
pub struct Outcome {
    /// O que o Python imprimiria.
    pub stdout: String,
    /// `0`, ou `1` quando não havia linguagem nenhuma.
    pub code: u8,
}

/// `parse_now()`: `--now` com fuso; sem ele, o relógio de verdade. Um
/// `--now` inválido derruba o Python antes de qualquer coisa.
fn clock(now: Option<&str>) -> Result<NaiveDateTime, AssetsError> {
    match now {
        None => Ok(chrono::Utc::now().naive_utc()),
        Some(value) => catalog::parse_now_local(value)
            .map(|(_, local)| local)
            .map_err(|error| crash(format!("ValueError: {error}"))),
    }
}

/// `load_excluded_names()` do `lang_stats.py`: arquivo ausente é nenhuma
/// exclusão; JSON inválido derruba.
fn excluded(root: &Path) -> Result<Vec<String>, AssetsError> {
    let path = root.join(catalog::EXCLUDED_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(crash(format!("{}: {error}", path.display()))),
    };
    let data: Value =
        serde_json::from_str(&text).map_err(|error| crash(format!("JSONDecodeError: {}: {error}", path.display())))?;
    let Value::Object(data) = data else {
        return Err(crash(format!("AttributeError: {} não é um objeto", path.display())));
    };
    // `{str(name) for name in data.get("repositories", [])}`: o Python
    // itera o que vier — texto dá os caracteres, objeto dá as chaves.
    match data.get("repositories") {
        None => Ok(Vec::new()),
        Some(Value::Array(names)) => Ok(names.iter().map(py_str).collect()),
        Some(Value::String(text)) => Ok(text.chars().map(String::from).collect()),
        Some(Value::Object(names)) => Ok(names.keys().cloned().collect()),
        Some(other) => Err(crash(format!("TypeError: \"repositories\" não é iterável em {}: {other}", path.display()))),
    }
}

/// `list_repos()`: com token, `/user/repos`; 401/403/404 caem no público.
async fn list_repos(client: &Client, user: &str) -> Result<Vec<Value>, AssetsError> {
    let private = Retry::LangStats { retry_denied: false };
    let public = Retry::LangStats { retry_denied: true };
    if client.authenticated() {
        let listing = client
            .get_pages(|page| format!("/user/repos?affiliation=owner&sort=pushed&per_page=100&page={page}"), private)
            .await;
        match listing {
            Ok(repos) => return Ok(repos),
            Err(ApiError::Http { status: status @ (401 | 403 | 404), .. }) => eprintln!(
                "aviso: /user/repos indisponível para este token (HTTP {status}); usando o endpoint público — \
                 repositórios privados ficam de fora (esperado com o GITHUB_TOKEN do Actions)."
            ),
            Err(error) => return Err(crash(format!("/user/repos: {error}"))),
        }
    }
    checked(format!("/users/{user}/repos"))?;
    client
        .get_pages(|page| format!("/users/{user}/repos?type=owner&per_page=100&page={page}"), public)
        .await
        .map_err(|error| crash(format!("/users/{user}/repos: {error}")))
}

/// O `lang_stats.py` monta a URL sem codificar nada, e o `http.client`
/// recusa caminho com espaço, controle ou fora do ASCII (`InvalidURL`,
/// `UnicodeEncodeError`) antes da rede — e ninguém trata: a análise inteira
/// cai. Um branch padrão com acento derruba o script (achado A26); o port
/// reproduz, e a correção entra como versão nova (D-006).
fn checked(path: String) -> Result<String, AssetsError> {
    match path.chars().find(|c| *c <= ' ' || *c >= '\u{7f}') {
        Some(c) => Err(crash(format!("InvalidURL: o http.client recusa {c:?} no caminho {path:?}"))),
        None => Ok(path),
    }
}

/// Erro que o `lang_stats.py` trata (aviso e segue): HTTP, rede e timeout.
fn handled(error: &ApiError) -> bool {
    matches!(error, ApiError::Http { .. } | ApiError::Url(_) | ApiError::Timeout)
}

/// Soma num mapa com ordem de chegada.
fn add(map: &mut Vec<(String, i64)>, key: &str, amount: i64) {
    match map.iter_mut().find(|(name, _)| name == key) {
        Some((_, total)) => *total += amount,
        None => map.push((key.to_string(), amount)),
    }
}

/// `sorted(..., key=valor, reverse=True)`: estável.
fn by_value(mut map: Vec<(String, i64)>) -> Vec<(String, i64)> {
    map.sort_by(|a, b| b.1.cmp(&a.1));
    map
}

/// Bytes de uma linguagem: inteiro (ou booleano, que o Python soma como 0/1).
fn bytes_of(language: &str, value: &Value) -> Result<i64, AssetsError> {
    match value {
        Value::Bool(flag) => Ok(i64::from(*flag)),
        Value::Number(number) if number.as_i64().is_some() => Ok(number.as_i64().unwrap_or_default()),
        other => Err(crash(format!("TypeError: bytes de {language:?} não são inteiros: {other}"))),
    }
}

/// `arquivos_do_repo(owner, name, branch)`: (contagem por extensão, truncado?, falhou?).
async fn files_of(
    client: &Client,
    owner: &str,
    name: &str,
    branch: &str,
) -> Result<(Vec<(String, i64)>, bool, bool), AssetsError> {
    if branch.is_empty() {
        return Ok((Vec::new(), false, false));
    }
    let tree = match client
        .get_json(
            &checked(format!("/repos/{owner}/{name}/git/trees/{branch}?recursive=1"))?,
            Retry::LangStats { retry_denied: true },
        )
        .await
    {
        Ok(tree) => tree,
        Err(error) if handled(&error) => {
            eprintln!(
                "aviso: não consegui ler a árvore de {name} ({error}) — ele conta no total, mas sem tipos de arquivo."
            );
            return Ok((Vec::new(), false, true));
        }
        Err(error) => return Err(crash(format!("árvore de {name}: {error}"))),
    };
    let Value::Object(tree) = tree else { return Ok((Vec::new(), false, false)) };
    let mut counts = Vec::new();
    let nodes = match tree.get("tree") {
        Some(Value::Array(nodes)) => nodes.as_slice(),
        Some(other) if json_truthy(other) => {
            return Err(crash(format!("AttributeError: \"tree\" de {name} não é lista de objetos")));
        }
        _ => &[],
    };
    for node in nodes {
        let Value::Object(node) = node else {
            return Err(crash(format!("AttributeError: nó da árvore de {name} não é objeto")));
        };
        if node.get("type").and_then(Value::as_str) != Some("blob") {
            continue;
        }
        let path = match node.get("path") {
            None => "",
            Some(Value::String(path)) => path.as_str(),
            Some(_) => return Err(crash(format!("AttributeError: caminho da árvore de {name} não é texto"))),
        };
        if let Some(ext) = lang_stats::extension(path) {
            add(&mut counts, &ext, 1);
        }
    }
    Ok((counts, tree.get("truncated").is_some_and(json_truthy), false))
}

/// O `collect()` do Python: a linha de contagem e os dados do SVG.
async fn collect(options: &Options, now: NaiveDateTime) -> Result<(String, LangStats), AssetsError> {
    let mut settings = Settings::new(LANG_STATS_AGENT);
    settings.api_version_header = true;
    settings.timeout = LANG_STATS_TIMEOUT;
    let client = Client::new(settings)?.with_token(options.token.clone());
    let listed = list_repos(&client, &options.user).await?;
    let excluded = excluded(&options.root)?;
    let is_excluded = |repo: &Value| {
        let field = |key| repo.get(key).map_or_else(String::new, py_str);
        excluded.contains(&field("name")) || excluded.contains(&field("full_name"))
    };

    let (mut per_lang, mut per_ext) = (Vec::new(), Vec::new());
    let (mut seen, mut private, mut without_language, mut truncated, mut failed, mut files) = (0, 0, 0, 0, 0, 0);
    for repo in listed.iter().filter(|repo| !is_excluded(repo)) {
        if repo.get("fork").is_some_and(json_truthy) && !options.include_forks {
            continue;
        }
        let owner = repo.get("owner").and_then(|owner| owner.get("login")).map(py_str);
        let (Some(owner), Some(name)) = (owner, repo.get("name").map(py_str)) else {
            return Err(crash("KeyError: repositório sem owner.login ou name"));
        };
        seen += 1;
        if repo.get("private").is_some_and(json_truthy) {
            private += 1;
        }

        let mut repo_failed = false;
        let languages = match client
            .get_json(&checked(format!("/repos/{owner}/{name}/languages"))?, Retry::LangStats { retry_denied: true })
            .await
        {
            Ok(value) => value,
            Err(error) if handled(&error) => {
                eprintln!("aviso: não consegui ler as linguagens de {name} ({error}).");
                repo_failed = true;
                Value::Null
            }
            Err(error) => return Err(crash(format!("linguagens de {name}: {error}"))),
        };
        match &languages {
            Value::Object(map) if !map.is_empty() => {
                for (language, size) in map {
                    add(&mut per_lang, language, bytes_of(language, size)?);
                }
            }
            other if json_truthy(other) => {
                return Err(crash(format!("AttributeError: linguagens de {name} não são um objeto")));
            }
            _ if !repo_failed => without_language += 1,
            _ => {}
        }

        if !options.skip_files {
            let branch =
                repo.get("default_branch").filter(|branch| json_truthy(branch)).map_or_else(String::new, py_str);
            let (exts, cut, tree_failed) = files_of(&client, &owner, &name, &branch).await?;
            truncated += i64::from(cut);
            repo_failed |= tree_failed;
            for (ext, count) in exts {
                add(&mut per_ext, &ext, count);
                files += count;
            }
        }
        failed += i64::from(repo_failed);
    }

    let line = format!(
        "repos={seen} privados={private} sem_linguagem={without_language} arquivos={files} \
         arvores_truncadas={truncated} repos_com_falha={failed}\n"
    );
    let total_bytes = per_lang.iter().map(|(_, bytes)| bytes).sum();
    let data = LangStats {
        per_lang: by_value(per_lang),
        per_ext: by_value(per_ext),
        repo_count: seen,
        arquivos_total: files,
        total_bytes,
        generated: now,
    };
    Ok((line, data))
}

/// `SVG_OUT.read_text()`: modo texto do Python (quebras universais).
fn read_text(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|text| text.replace("\r\n", "\n").replace('\r', "\n"))
}

fn write(path: &Path, text: &str) -> Result<(), AssetsError> {
    std::fs::write(path, text).map_err(|error| crash(format!("{}: {error}", path.display())))
}

/// Grava se algo além do carimbo mudou; devolve se gravou.
fn write_if_changed(path: &Path, text: &str) -> Result<bool, AssetsError> {
    let changed = read_text(path).is_none_or(|old| !lang_stats::same_content(text, &old));
    if changed {
        write(path, text)?;
    }
    Ok(changed)
}

fn py_bool(flag: bool) -> &'static str {
    if flag { "True" } else { "False" }
}

/// `render lang-stats`.
pub async fn run_lang_stats(options: &Options) -> Result<Outcome, AssetsError> {
    let now = clock(options.now.as_deref())?;
    let (mut stdout, data) = collect(options, now).await?;
    if data.per_lang.is_empty() {
        eprintln!("Nenhuma linguagem coletada — abortando sem alterar arquivos.");
        return Ok(Outcome { stdout, code: 1 });
    }
    let assets = options.root.join("assets");
    std::fs::create_dir_all(&assets).map_err(|error| crash(format!("{}: {error}", assets.display())))?;
    let svg_changed = write_if_changed(&assets.join("lang-stats.svg"), &lang_stats::build_svg(&data))?;
    let top_changed = write_if_changed(&assets.join("profile-top-langs.svg"), &lang_stats::build_top_langs_svg(&data))?;
    stdout.push_str(&format!(
        "linguagens={} repos={} bytes={} svg_changed={} top_langs_changed={}\n",
        data.per_lang.len(),
        data.repo_count,
        data.total_bytes,
        py_bool(svg_changed),
        py_bool(top_changed)
    ));
    Ok(Outcome { stdout, code: 0 })
}

/// `strftime` com o ano sem zeros à esquerda (glibc).
fn day(when: NaiveDateTime, suffix: &str) -> String {
    format!("{}-{:02}-{:02}{suffix}", when.year(), when.month(), when.day())
}

/// `data[key]` de um objeto JSON, ou o `KeyError`/`TypeError` do Python.
fn item<'a>(value: &'a Value, key: &str) -> Result<&'a Value, AssetsError> {
    match value {
        Value::Object(map) => map.get(key).ok_or_else(|| crash(format!("KeyError: '{key}'"))),
        other => Err(crash(format!("TypeError: {other} não é subscritável por {key:?}"))),
    }
}

/// `request_data()`: a consulta e o dicionário `data`.
async fn request_data(options: &Options, now: NaiveDateTime) -> Result<Map<String, Value>, AssetsError> {
    let token = options.token.clone().filter(|token| !token.is_empty());
    let Some(token) = token else { return Err(crash("RuntimeError: GITHUB_TOKEN ausente")) };
    let variables = json!({
        "login": options.user,
        "from": day(now - TimeDelta::days(365), "T00:00:00Z"),
        "to": day(now, "T23:59:59Z"),
    });
    let client = Client::new(Settings::new(CARDS_AGENT))?.with_token(Some(token));
    let result = match client.graphql(CARDS_QUERY, variables).await {
        Ok(result) => result,
        Err(ApiError::Http { status, .. }) => return Err(crash(format!("RuntimeError: GitHub GraphQL HTTP {status}"))),
        Err(error) => return Err(crash(format!("GraphQL: {error}"))),
    };
    let Value::Object(result) = result else { return Err(crash("AttributeError: resposta GraphQL não é objeto")) };
    if let Some(errors) = result.get("errors").filter(|errors| json_truthy(errors)) {
        let Value::Array(errors) = errors else { return Err(crash("TypeError: \"errors\" não é lista")) };
        let messages: Vec<String> = errors
            .iter()
            .map(|error| error.get("message").map_or_else(|| "GraphQL error".to_string(), py_str))
            .collect();
        return Err(crash(format!("RuntimeError: {}", messages.join("; "))));
    }
    let user = match result.get("data").filter(|data| json_truthy(data)) {
        Some(Value::Object(data)) => data.get("user").cloned().unwrap_or(Value::Null),
        Some(_) => return Err(crash("AttributeError: \"data\" não é objeto")),
        None => Value::Null,
    };
    if !json_truthy(&user) {
        return Err(crash("RuntimeError: usuário não encontrado na API GraphQL"));
    }
    let collection = item(&user, "contributionsCollection")?;
    let login = item(&user, "login")?.clone();
    let name = user.get("name").filter(|name| json_truthy(name)).cloned().unwrap_or_else(|| login.clone());
    let mut data = Map::new();
    data.insert("login".into(), login);
    data.insert("name".into(), name);
    data.insert("contributions".into(), item(item(collection, "contributionCalendar")?, "totalContributions")?.clone());
    for (key, field) in [
        ("commits", "totalCommitContributions"),
        ("issues", "totalIssueContributions"),
        ("pull_requests", "totalPullRequestContributions"),
        ("reviews", "totalPullRequestReviewContributions"),
        ("repository_contributions", "totalRepositoryContributions"),
        ("restricted", "restrictedContributionsCount"),
    ] {
        data.insert(key.into(), item(collection, field)?.clone());
    }
    data.insert("repositories".into(), item(item(&user, "repositories")?, "totalCount")?.clone());
    data.insert("generated".into(), Value::String(lang_stats::stamp(now)));
    Ok(data)
}

/// `render cards`.
pub async fn run_cards(options: &Options) -> Result<Outcome, AssetsError> {
    let now = clock(options.now.as_deref())?;
    let data = request_data(options, now).await?;
    let assets = options.root.join("assets");
    std::fs::create_dir_all(&assets).map_err(|error| crash(format!("{}: {error}", assets.display())))?;
    for (file, svg) in cards::FILES.iter().zip(cards::all(&data)) {
        write(&assets.join(file), &svg)?;
    }
    let stdout = format!("{}\ngerados={}\n", py_dumps(&Value::Object(data), true), cards::FILES.join(","));
    Ok(Outcome { stdout, code: 0 })
}
