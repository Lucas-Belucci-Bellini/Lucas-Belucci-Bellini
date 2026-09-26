//! Os 13 blocos gerados do README, cada um a função `render_*` de mesmo nome
//! no `scripts/update_profile.py`.

use std::collections::HashSet;

use ecosystem_domain::classify::classify_py_v1;
use ecosystem_domain::discovery::{json_truthy, py_str};
use ecosystem_domain::lifecycle::status_py_v1;
use ecosystem_domain::presentation::{FEATURED_SUMMARIES, Presentation, describe};
use ecosystem_domain::priority::featured_score_py_v1;
use ecosystem_domain::pyjson::py_int;
use ecosystem_domain::repo::RepoFacts;
use ecosystem_domain::text::py_split_join;
use serde_json::{Map, Value};

use crate::badges::{COLOR_GOLD, badge, cta_buttons, cta_cell, status_pill};
use crate::languages::{color, format_bytes, stack_for};
use crate::pytext::{md_cell, py_len, py_prefix, py_rstrip, quote};
use crate::{OWNER, Profile, RenderError};

/// Os blocos, na ordem em que o `main()` do Python os substitui. A ordem
/// importa: cada substituição vale sobre o texto que a anterior deixou.
pub const MARKERS: [&str; 13] = [
    "PROFILE-DASHBOARD",
    "WHAT-I-BUILD",
    "PRODUCT-CARDS",
    "FEATURED-PROJECTS",
    "WEBSITE-DIRECTORY",
    "ARSENAL-STACK",
    "LANGUAGE-BADGES",
    "LANGUAGE-STATS",
    "PUBLIC-PROJECTS",
    "PRIVATE-PROJECTS",
    "LIVE-PROJECTS",
    "ECOSYSTEM-MAP",
    "PROJECT-MAP",
];

/// Domínios da vitrine (`DOMAINS`): ícone, título, resumo e os rótulos de
/// `classify()` que caem em cada um.
pub const DOMAINS: [(&str, &str, &str, &[&str]); 6] = [
    ("🌐", "WEB & SAAS", "Plataformas, ferramentas e produtos web publicados.", &["Ecossistema Baluarte", "Web"]),
    ("🤖", "AI & AUTOMATION", "Agentes, automação e sistemas de conhecimento.", &["IA & Automação"]),
    ("⚙", "HARDWARE & LOGIC", "Lógica digital, CPUs do zero e eletrônica.", &["Digital Logic / Hardware"]),
    ("🎮", "GAMES & WORLDS", "Jogos, simulações e mundos jogáveis.", &["Games"]),
    (
        "🛠",
        "TOOLS & SYSTEMS",
        "Utilitários, scripts e infraestrutura de apoio.",
        &["Software & Ferramentas", "Infraestrutura / Backend / Dados"],
    ),
    ("🎓", "ACADEMIC & LABS", "Trabalhos de curso, estudos dirigidos e experimentos.", &["Academia", "Experimentos"]),
];

/// Ordem fixa das categorias do arsenal.
const ARSENAL_ORDER: [(&str, &str); 4] = [
    ("Frameworks & Web", "🧩"),
    ("Infraestrutura & DevOps", "🛡"),
    ("IA & Conhecimento", "🤖"),
    ("Hardware & Simulação", "⚙"),
];

/// Um bloco pelo nome do marcador.
pub fn render(profile: &Profile, marker: &str) -> Result<String, RenderError> {
    Ok(match marker {
        "PROFILE-DASHBOARD" => dashboard(profile),
        "WHAT-I-BUILD" => what_i_build(profile),
        "PRODUCT-CARDS" => product_cards(profile),
        "FEATURED-PROJECTS" => featured_projects(profile)?,
        "WEBSITE-DIRECTORY" => website_directory(profile),
        "ARSENAL-STACK" => arsenal_stack(profile)?,
        "LANGUAGE-BADGES" => language_badges(profile),
        "LANGUAGE-STATS" => language_stats(profile),
        "PUBLIC-PROJECTS" => public_projects(profile),
        "PRIVATE-PROJECTS" => private_projects(profile),
        "LIVE-PROJECTS" => live_projects(profile),
        "ECOSYSTEM-MAP" => ecosystem_map(profile),
        "PROJECT-MAP" => project_map(profile),
        other => return Err(RenderError::MarkerNotFound(other.into())),
    })
}

/// `replace_block(text, marker, body)`: troca o conteúdo do **primeiro** par
/// de marcadores. O corpo entra literalmente (uma `\` numa descrição do
/// GitHub não é escape de nada).
pub fn replace_block(text: &str, marker: &str, body: &str) -> Result<String, RenderError> {
    let start_tag = format!("<!-- {marker}:START -->");
    let end_tag = format!("<!-- {marker}:END -->");
    // A regex do Python é `START.*?END` com DOTALL: a primeira ocorrência de
    // START que tem um END depois dela, e o END mais próximo. Se a primeira
    // START não tem END depois, nenhuma outra tem.
    let not_found = || RenderError::MarkerNotFound(marker.into());
    let start = text.find(&start_tag).ok_or_else(not_found)?;
    let after = start + start_tag.len();
    let end = text[after..].find(&end_tag).ok_or_else(not_found)? + after;
    Ok(format!("{}{start_tag}\n{}\n{end_tag}{}", &text[..start], py_rstrip(body), &text[end + end_tag.len()..]))
}

fn crash(kind: &'static str, detail: String) -> RenderError {
    RenderError::PythonCrash { kind, detail }
}

/// Nome do tipo que o Python veria para um valor do `json.loads`.
fn py_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "NoneType",
        Value::Bool(_) => "bool",
        Value::Number(number) if number.is_f64() => "float",
        Value::Number(_) => "int",
        Value::String(_) => "str",
        Value::Array(_) => "list",
        Value::Object(_) => "dict",
    }
}

/// Percorre `manifest.get(key, [])` chamando `.get` em cada item, como o
/// Python: lista de objetos passa; lista com outra coisa, ou objeto/texto não
/// vazio, é `AttributeError`; `null`, número e booleano, `TypeError`.
fn objects<'m>(
    manifest: &'m Map<String, Value>,
    key: &str,
    file: &str,
) -> Result<Vec<&'m Map<String, Value>>, RenderError> {
    let attribute = |value: &Value| {
        crash("AttributeError", format!("'{}' object has no attribute 'get' ({file}: \"{key}\")", py_type_name(value)))
    };
    match manifest.get(key) {
        None => Ok(Vec::new()),
        Some(Value::Array(items)) => items.iter().map(|item| item.as_object().ok_or_else(|| attribute(item))).collect(),
        Some(Value::Object(map)) if map.is_empty() => Ok(Vec::new()),
        Some(Value::String(text)) if text.is_empty() => Ok(Vec::new()),
        // Objeto e texto iteram chaves e caracteres: `str` sem `.get`.
        Some(Value::Object(_) | Value::String(_)) => Err(attribute(&Value::String(String::new()))),
        Some(other) => {
            Err(crash("TypeError", format!("'{}' object is not iterable ({file}: \"{key}\")", py_type_name(other))))
        }
    }
}

/// `str(entry.get(key, default))`.
fn get_str(entry: &Map<String, Value>, key: &str, default: &str) -> String {
    entry.get(key).map_or_else(|| default.to_string(), py_str)
}

fn summary(name: &str) -> Option<&'static str> {
    FEATURED_SUMMARIES.iter().find(|(key, _)| *key == name).map(|(_, text)| *text)
}

/// `(-p.marketing_priority, p.name.lower())`.
fn by_priority(a: &&Presentation, b: &&Presentation) -> std::cmp::Ordering {
    b.marketing_priority.cmp(&a.marketing_priority).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "projeto" } else { "projetos" }
}

/// `render_dashboard`.
pub fn dashboard(profile: &Profile) -> String {
    let repos = profile.inputs.repos;
    let now = profile.inputs.now;
    let active = repos.iter().filter(|repo| status_py_v1(repo, classify_py_v1(repo), now) == "🟢 Active").count();
    let public = repos.iter().filter(|repo| !repo.private).count();
    let private = repos.len() - public;
    let academic = repos.iter().filter(|repo| classify_py_v1(repo) == "Academia").count();
    [
        "> `GITHUB SNAPSHOT // FIELD REPORT` · inventário autenticado, métricas públicas e governança editorial."
            .into(),
        String::new(),
        "![GitHub snapshot](./assets/profile-snapshot.svg)".into(),
        String::new(),
        "<div align=\"center\">".into(),
        String::new(),
        "| REPOSITÓRIOS | PÚBLICOS | PRIVADOS VISÍVEIS | DEPLOYMENTS |".into(),
        "|:---:|:---:|:---:|:---:|".into(),
        format!("| **{}** | **{public}** | **{private}** | **{}** |", repos.len(), profile.verified_sites()),
        String::new(),
        "| ATIVOS | ACADÊMICOS | LINGUAGENS | EXCLUSÕES EDITORIAIS |".into(),
        "|:---:|:---:|:---:|:---:|".into(),
        format!("| **{active}** | **{academic}** | **{}** | **{}** |", profile.rows.len(), profile.inputs.excluded),
        String::new(),
        "</div>".into(),
        String::new(),
        "> `STATUS: ONLINE` · `PRIVACY: SAFE` · contagens geradas pelo inventário autenticado do GitHub; nenhum \
         conteúdo de arquivo privado é publicado."
            .into(),
    ]
    .join("\n")
}

/// `render_language_badges`.
pub fn language_badges(profile: &Profile) -> String {
    let rows = &profile.rows;
    let shields: Vec<String> = rows
        .iter()
        .map(|row| {
            let badge_url = format!(
                "https://img.shields.io/badge/{}-{}-{}?style=flat-square&labelColor=0e0c16",
                quote(&row.display),
                quote(&format!("{} repos", row.repositories)),
                color(&row.language, &row.display)
            );
            format!(
                "[![{}]({badge_url})](https://github.com/{OWNER}?tab=repositories&q=&language={})",
                row.display,
                quote(&row.language)
            )
        })
        .collect();
    let split_at = shields.len().div_ceil(2);
    let badge_lines = |items: &[String]| -> Vec<String> { items.chunks(5).map(|chunk| chunk.join(" ")).collect() };

    let mut lines = vec![
        format!(
            "> **{} linguagens públicas detectadas** · badges gerados a partir dos repositórios auditados.",
            rows.len()
        ),
        "> Os nomes permanecem legíveis mesmo quando a URL precisa escapar caracteres como `#` e `/`; a matriz abaixo \
         informa peso em bytes e participação relativa."
            .into(),
        String::new(),
    ];
    let section = |lines: &mut Vec<String>, items: &[String]| {
        for (index, line) in badge_lines(items).into_iter().enumerate() {
            if index > 0 {
                lines.push("<br>".into());
            }
            lines.push(line);
        }
    };
    if !shields.is_empty() {
        lines.extend(["**PRINCIPAIS POR VOLUME**".into(), String::new()]);
        section(&mut lines, &shields[..split_at]);
    }
    if !shields[split_at..].is_empty() {
        lines.extend([String::new(), String::new(), "**EXTENSÕES DO PORTFÓLIO**".into(), String::new()]);
        section(&mut lines, &shields[split_at..]);
    }
    lines.join("\n")
}

/// `render_language_stats`.
pub fn language_stats(profile: &Profile) -> String {
    let rows = &profile.rows;
    let total: i64 = rows.iter().map(|row| row.bytes).sum();
    let public = profile.inputs.repos.iter().filter(|repo| !repo.private).count();
    let mut lines = vec![
        format!(
            "> **{} linguagens** · **{public} repositórios públicos** · **{} de código detectado** · atualizado em `{}`",
            rows.len(),
            format_bytes(total),
            profile.inputs.generated_at
        ),
        String::new(),
        "| # | Linguagem | Peso | Participação | Repositórios |".into(),
        "|:--:|:---|---:|---:|---:|".into(),
    ];
    for (index, row) in rows.iter().enumerate() {
        lines.push(format!(
            "| {} | **{}** | `{}` | `{:.2}%` | {} |",
            index + 1,
            row.display,
            format_bytes(row.bytes),
            row.share,
            row.repositories
        ));
    }
    lines.extend([
        String::new(),
        "> A tabela acima considera somente repositórios públicos. Repositórios privados podem contribuir para \
         métricas agregadas futuras, mas seus arquivos, nomes de arquivos e estrutura interna não são publicados."
            .into(),
        String::new(),
        "![Public language distribution](./assets/lang-stats.svg)".into(),
    ]);
    lines.join("\n")
}

fn tool_table(lines: &mut Vec<String>, title: String, tools: &[&Map<String, Value>]) {
    lines.extend([
        String::new(),
        title,
        String::new(),
        "| Ferramenta | Papel | Evidência pública |".into(),
        "|:---|:---|:---|".into(),
    ]);
    for tool in tools {
        let field = |key| md_cell(&get_str(tool, key, ""));
        lines.push(format!("| **{}** | {} | {} |", field("name"), field("family"), field("evidence")));
    }
}

/// `render_arsenal_stack`.
pub fn arsenal_stack(profile: &Profile) -> Result<String, RenderError> {
    let rows = &profile.rows;
    let principais = rows.iter().take(8).map(|row| format!("`{}`", row.display)).collect::<Vec<_>>().join(" · ");
    let mut lines = vec![
        format!("> **{} linguagens detectadas** no inventário público, por peso em bytes: {principais}.", rows.len()),
        String::new(),
        "<details>".into(),
        "<summary><b>▶ Ver a participação de cada linguagem</b></summary>".into(),
        String::new(),
        "| Linguagem | Repositórios | Participação |".into(),
        "|:---|---:|---:|".into(),
    ];
    for row in rows {
        lines.push(format!("| **{}** | {} | `{:.2}%` |", row.display, row.repositories, row.share));
    }
    lines.extend([String::new(), "</details>".into()]);

    // by_category: um dict, na ordem em que cada categoria aparece.
    let mut by_category: Vec<(String, Vec<&Map<String, Value>>)> = Vec::new();
    for tool in objects(profile.inputs.stack, "tools", "README_STACK.json")? {
        let category = get_str(tool, "category", "Outras ferramentas");
        match by_category.iter_mut().find(|(name, _)| *name == category) {
            Some((_, tools)) => tools.push(tool),
            None => by_category.push((category, vec![tool])),
        }
    }
    for (category, icon) in ARSENAL_ORDER {
        if let Some((_, tools)) = by_category.iter().find(|(name, _)| name == category) {
            tool_table(&mut lines, format!("### {icon} {category}"), tools);
        }
    }
    let mut remaining: Vec<&(String, Vec<&Map<String, Value>>)> =
        by_category.iter().filter(|(name, _)| !ARSENAL_ORDER.iter().any(|(known, _)| known == name)).collect();
    remaining.sort_by(|a, b| a.0.cmp(&b.0));
    for (category, tools) in remaining {
        tool_table(&mut lines, format!("### 🧰 {category}"), tools);
    }
    lines.extend([
        String::new(),
        "> A separação é editorial: linguagens vêm dos mapas públicos do GitHub; ferramentas e categorias vêm de \
         READMEs, manifests, configurações e evidências visuais públicas. Nenhum bloco publica conteúdo privado."
            .into(),
    ]);
    Ok(lines.join("\n"))
}

/// `render_featured_projects`: a curadoria na frente, a heurística completando
/// até dez linhas.
pub fn featured_projects(profile: &Profile) -> Result<String, RenderError> {
    const MAXIMUM: usize = 10;
    let manifest = profile.inputs.featured;
    // by_name: um dict por nome — posição do primeiro, valor do último.
    let mut by_name: Vec<(&str, &RepoFacts)> = Vec::new();
    for repo in profile.inputs.repos.iter().filter(|repo| !repo.private) {
        match by_name.iter_mut().find(|(name, _)| *name == repo.name) {
            Some(entry) => entry.1 = repo,
            None => by_name.push((&repo.name, repo)),
        }
    }
    let mut lines = vec![
        format!("> {}", get_str(manifest, "intro", "Seleção editorial de projetos públicos.")),
        String::new(),
        "| # | Missão | O que a evidência pública confirma | Status | Acesso |".into(),
        "|:--:|:---|:---|:---|:---|".into(),
    ];

    let mut entries = Vec::new();
    for entry in objects(manifest, "projects", "README_FEATURED.json")? {
        let order = match entry.get("order") {
            None => 999,
            Some(value) => py_int(value).map_err(|error| {
                crash(error.python_name(), format!("README_FEATURED.json: \"order\" não é inteiro: {value}"))
            })?,
        };
        entries.push((order, entry));
    }
    entries.sort_by_key(|(order, _)| *order);

    let mut chosen: Vec<(String, String, &Presentation)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (_, entry) in entries {
        let name = get_str(entry, "name", "");
        let Some((_, repo)) = by_name.iter().find(|(key, _)| *key == name) else { continue };
        seen.insert(name.clone());
        let item = profile.presentation(repo);
        if entry.get("website_required").is_some_and(json_truthy) && !item.has_live_website() {
            continue;
        }
        let label = md_cell(&get_str(entry, "label", "MISSÃO"));
        let focus = match entry.get("focus").filter(|value| json_truthy(value)) {
            Some(value) => py_str(value),
            None => summary(&name).map_or_else(|| describe(repo.description.as_deref()), str::to_string),
        };
        chosen.push((label, md_cell(&py_split_join(&focus)), item));
    }

    let mut rest: Vec<(f64, &RepoFacts)> = by_name
        .iter()
        .filter(|(name, _)| !seen.contains(*name))
        .map(|(_, repo)| (featured_score_py_v1(repo, profile.inputs.now), *repo))
        .collect();
    // sorted(..., reverse=True) é estável: o empate fica na ordem do inventário.
    rest.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, repo) in rest.into_iter().take(MAXIMUM.saturating_sub(chosen.len())) {
        let item = profile.presentation(repo);
        let text = summary(&repo.name).map_or_else(|| describe(repo.description.as_deref()), str::to_string);
        chosen.push((classify_py_v1(repo).to_uppercase(), md_cell(&text), item));
    }

    if chosen.is_empty() {
        lines.push(
            "| — | Nenhuma missão pública encontrada | O manifesto será revisado no próximo refresh. | — | — |".into(),
        );
        return Ok(lines.join("\n"));
    }
    for (index, (label, focus, item)) in chosen.iter().enumerate() {
        lines.push(format!(
            "| {} | **{label}** · {} | {focus} | {} | {} |",
            index + 1,
            item.name,
            item.status,
            cta_cell(item)
        ));
    }
    Ok(lines.join("\n"))
}

/// `max(items, key=...)`: no empate, o **primeiro** (o `max_by` do Rust daria o último).
fn first_max<'p>(items: &[&'p Presentation]) -> &'p Presentation {
    let key = |p: &Presentation| (p.marketing_priority, p.name.to_lowercase());
    let mut best = items[0];
    let mut best_key = key(best);
    for item in &items[1..] {
        let candidate = key(item);
        if candidate > best_key {
            best = item;
            best_key = candidate;
        }
    }
    best
}

/// `render_what_i_build`: os domínios com projeto público, três por linha.
pub fn what_i_build(profile: &Profile) -> String {
    const COLUMNS: usize = 3;
    let public = profile.public();
    let present: Vec<(&str, &str, &str, Vec<&Presentation>)> = DOMAINS
        .iter()
        .filter_map(|(icon, title, text, labels)| {
            let items: Vec<&Presentation> =
                public.iter().copied().filter(|p| labels.contains(&p.category_label.as_str())).collect();
            (!items.is_empty()).then_some((*icon, *title, *text, items))
        })
        .collect();
    if present.is_empty() {
        return "> Nenhum projeto público para agrupar nesta auditoria.".into();
    }
    let width = format!("{}%", 100 / COLUMNS);
    let mut lines = vec!["<table>".to_string()];
    for slice in present.chunks(COLUMNS) {
        lines.push("<tr>".into());
        for (icon, title, text, items) in slice {
            let live = items.iter().filter(|p| p.has_live_website()).count();
            lines.extend([
                format!("<td width=\"{width}\" valign=\"top\" align=\"center\">"),
                String::new(),
                format!("### {icon}"),
                String::new(),
                format!("**{title}**"),
                String::new(),
                format!("<sub>{text}</sub>"),
                String::new(),
                format!("`{} {}` · `{live} com site`", items.len(), plural(items.len())),
                String::new(),
                format!("<sub>ex.: {}</sub>", first_max(items).name),
                String::new(),
                "</td>".into(),
            ]);
        }
        for _ in slice.len()..COLUMNS {
            lines.push(format!("<td width=\"{width}\"></td>"));
        }
        lines.push("</tr>".into());
    }
    lines.push("</table>".into());
    lines.join("\n")
}

/// Os que têm site no ar, pelo nome sem caixa (ordenação estável).
fn live_sorted<'p>(profile: &Profile<'p>) -> Vec<&'p Presentation> {
    let mut live: Vec<&Presentation> =
        profile.inputs.repos.iter().map(|repo| profile.presentation(repo)).filter(|p| p.has_live_website()).collect();
    live.sort_by_cached_key(|p| p.name.to_lowercase());
    live
}

/// `render_website_directory`: um badge por site no ar.
pub fn website_directory(profile: &Profile) -> String {
    let live = live_sorted(profile);
    if live.is_empty() {
        return "> Nenhum site respondeu na última auditoria.".into();
    }
    let mut lines = vec![
        format!("> **{} sites no ar** · cada link foi conferido por requisição HTTP nesta auditoria.", live.len()),
        String::new(),
        "<div align=\"center\">".into(),
        String::new(),
    ];
    for item in &live {
        lines.push(format!(
            "[![{}]({})]({})",
            item.name,
            badge(&format!("🌐 {}", item.name.to_uppercase()), COLOR_GOLD, "flat-square"),
            item.website.as_deref().unwrap_or_default()
        ));
    }
    lines.extend([String::new(), "</div>".into()]);
    lines.join("\n")
}

/// `card_summary(presentation)`: até 150 caracteres, sem cortar palavra.
pub fn card_summary(presentation: &Presentation) -> String {
    const LIMIT: usize = 150;
    let text = summary(&presentation.name).unwrap_or(&presentation.description);
    if py_len(text) <= LIMIT {
        return text.to_string();
    }
    let prefix = py_prefix(text, LIMIT);
    let cut = prefix.rfind(' ').map_or(prefix, |index| &prefix[..index]);
    format!("{}…", cut.trim_end_matches([' ', '.', ',', ';', ':']))
}

/// `render_product_cards`: seis cards, primeiro os que têm site no ar.
pub fn product_cards(profile: &Profile) -> String {
    const COLUMNS: usize = 2;
    const MAXIMUM: usize = 6;
    let mut ordered = profile.public();
    ordered.sort_by(by_priority);
    let mut chosen: Vec<&Presentation> =
        ordered.iter().copied().filter(|p| p.has_live_website()).take(MAXIMUM).collect();
    if chosen.len() < MAXIMUM {
        // `p not in escolhidos` compara por valor (dataclass), não por posição.
        let rest: Vec<&Presentation> = ordered.iter().copied().filter(|p| !chosen.contains(p)).collect();
        let missing = MAXIMUM - chosen.len();
        chosen.extend(rest.into_iter().take(missing));
    }
    if chosen.is_empty() {
        return "> Nenhum projeto público disponível nesta auditoria.".into();
    }
    let width = format!("{}%", 100 / COLUMNS);
    let mut lines = vec!["<table>".to_string()];
    for slice in chosen.chunks(COLUMNS) {
        lines.push("<tr>".into());
        for item in slice {
            lines.extend([
                format!("<td width=\"{width}\" valign=\"top\">"),
                String::new(),
                format!("<sub>`{}`</sub>", item.category_label.to_uppercase()),
                String::new(),
                format!("### {}", item.name),
                String::new(),
                card_summary(item),
                String::new(),
                status_pill(item),
                String::new(),
                cta_buttons(item),
                String::new(),
                "</td>".into(),
            ]);
        }
        for _ in slice.len()..COLUMNS {
            lines.push(format!("<td width=\"{width}\"></td>"));
        }
        lines.push("</tr>".into());
    }
    lines.push("</table>".into());
    lines.join("\n")
}

/// `render_ecosystem_map`: a árvore por categoria.
pub fn ecosystem_map(profile: &Profile) -> String {
    const PER_BRANCH: usize = 4;
    let mut branches: Vec<(&str, Vec<&Presentation>)> = Vec::new();
    for item in profile.public() {
        match branches.iter_mut().find(|(label, _)| *label == item.category_label) {
            Some((_, items)) => items.push(item),
            None => branches.push((&item.category_label, vec![item])),
        }
    }
    let live = |items: &[&Presentation]| items.iter().filter(|p| p.has_live_website()).count();
    branches.sort_by(|(a_label, a), (b_label, b)| {
        live(b)
            .cmp(&live(a))
            .then_with(|| b.len().cmp(&a.len()))
            .then_with(|| a_label.to_lowercase().cmp(&b_label.to_lowercase()))
    });
    if branches.is_empty() {
        return "> Nenhum projeto público para mapear nesta auditoria.".into();
    }
    let width = branches.iter().map(|(label, _)| py_len(label)).max().unwrap_or(0);
    let mut lines: Vec<String> = vec!["```text".into(), "ECOSSISTEMA".into(), "│".into()];
    for (index, (category, items)) in branches.iter().enumerate() {
        let last = index == branches.len() - 1;
        let (trunk, stem) = if last { ("└──", "   ") } else { ("├──", "│  ") };
        let dots = ".".repeat((width + 3).saturating_sub(py_len(category)).max(3));
        let noun = if items.len() == 1 { "projeto " } else { "projetos" };
        lines.push(format!("{trunk} {category} {dots} {:>2} {noun} · {} com site", items.len(), live(items)));
        let mut highlights = items.clone();
        highlights.sort_by(by_priority);
        highlights.truncate(PER_BRANCH);
        let mut names: Vec<String> =
            highlights.iter().map(|p| format!("{}{}", p.name, if p.has_live_website() { " ●" } else { "" })).collect();
        let spare = items.len() - highlights.len();
        if spare > 0 {
            names.push(format!("+{spare}"));
        }
        lines.push(format!("{stem} └─ {}", names.join(" · ")));
        if !last {
            lines.push("│".into());
        }
    }
    lines.extend([
        "```".into(),
        String::new(),
        "> `●` marca projeto com site verificado nesta auditoria. Os ramos usam os rótulos editoriais que o README já \
         exibe; o equivalente canônico, para reuso externo, está em [`docs/project-catalog.json`](docs/project-catalog.json). \
         Repositórios privados não entram no mapa público."
            .into(),
    ]);
    lines.join("\n")
}

/// Repositórios filtrados e ordenados pelo nome sem caixa (estável).
fn sorted_repos(repos: &[RepoFacts], keep: impl Fn(&RepoFacts) -> bool) -> Vec<&RepoFacts> {
    let mut selected: Vec<&RepoFacts> = repos.iter().filter(|repo| keep(repo)).collect();
    selected.sort_by_cached_key(|repo| repo.name.to_lowercase());
    selected
}

/// `render_public_projects`.
pub fn public_projects(profile: &Profile) -> String {
    let mut lines: Vec<String> = vec![
        "<details>".into(),
        "<summary><b>🌐 Public repository catalog</b></summary>".into(),
        String::new(),
        "| Projeto | Categoria | Status | Acesso |".into(),
        "|:---|:---|:---|:---|".into(),
    ];
    for repo in sorted_repos(profile.inputs.repos, |repo| !repo.private) {
        let item = profile.presentation(repo);
        lines.push(format!("| **{}** | {} | {} | {} |", repo.name, item.category_label, item.status, cta_cell(item)));
    }
    lines.extend([String::new(), "</details>".into()]);
    lines.join("\n")
}

/// `render_private_projects`: nome, categoria, descrição pública e status — nada do conteúdo.
pub fn private_projects(profile: &Profile) -> String {
    let mut lines: Vec<String> = vec![
        "<details>".into(),
        "<summary><b>🔒 Private repository catalog</b></summary>".into(),
        String::new(),
        "| Projeto | Categoria | Descrição pública | Status | GitHub |".into(),
        "|:---|:---|:---|:---|:---|".into(),
    ];
    for repo in sorted_repos(profile.inputs.repos, |repo| repo.private) {
        let category = classify_py_v1(repo);
        let description = match repo.description.as_deref() {
            Some(text) if !text.is_empty() => text,
            _ => "Descrição pública não informada",
        };
        lines.push(format!(
            "| **{}** | {category} | {} | {} | [GitHub](https://github.com/{}) |",
            repo.name,
            md_cell(description).replace('\n', " "),
            status_py_v1(repo, category, profile.inputs.now),
            repo.full_name
        ));
    }
    lines.extend([
        String::new(),
        "</details>".into(),
        String::new(),
        "> 🔒 Private repository · nenhuma linha desta seção expõe código, secrets, `.env`, tokens, credenciais ou \
         estrutura interna."
            .into(),
    ]);
    lines.join("\n")
}

/// `render_live_projects`: o site antes do código.
pub fn live_projects(profile: &Profile) -> String {
    let mut lines: Vec<String> =
        vec!["| Projeto | Website | Verificação | Código |".into(), "|:---|:---|:---:|:---|".into()];
    let live = live_sorted(profile);
    for item in &live {
        lines.push(format!(
            "| **{}** | **[▸ Abrir site]({})** | `HTTP {}` | [código]({}) |",
            item.name,
            item.website.as_deref().unwrap_or_default(),
            item.website_http_status,
            item.github
        ));
    }
    if live.is_empty() {
        lines.push("| — | Nenhum site verificado nesta auditoria | — | — |".into());
    }
    lines.join("\n")
}

/// `render_project_map`: todos os repositórios, privados inclusive (A20: a
/// pilha dos privados aparece — defeito registrado, corrigido só como versão nova).
pub fn project_map(profile: &Profile) -> String {
    let mut lines: Vec<String> = vec![
        "<details>".into(),
        "<summary><b>⌁ Complete project map</b></summary>".into(),
        String::new(),
        "| Projeto | Categoria | Stack | Status | Acesso |".into(),
        "|:---|:---|:---|:---|:---|".into(),
    ];
    for repo in sorted_repos(profile.inputs.repos, |_| true) {
        let item = profile.presentation(repo);
        lines.push(format!(
            "| **{}** | {} | {} | {} | {} |",
            repo.name,
            item.category_label,
            stack_for(repo, profile.inputs.languages),
            item.status,
            cta_cell(item)
        ));
    }
    lines.extend([String::new(), "</details>".into()]);
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitui_so_o_primeiro_par_e_preserva_barra_invertida() {
        let text = "a\n<!-- X:START -->\nvelho\n<!-- X:END -->\nb\n<!-- X:START -->\n<!-- X:END -->";
        let replaced = replace_block(text, "X", "novo \\1 \\g<0>\n\n").unwrap();
        assert_eq!(
            "a\n<!-- X:START -->\nnovo \\1 \\g<0>\n<!-- X:END -->\nb\n<!-- X:START -->\n<!-- X:END -->",
            replaced
        );
    }

    #[test]
    fn marcador_sem_fim_e_erro() {
        let error = replace_block("<!-- X:END --><!-- X:START -->", "X", "").unwrap_err();
        assert_eq!("ValueError: README marker not found: X", error.to_string());
    }

    #[test]
    fn resumo_do_card_corta_na_palavra() {
        let mut p = Presentation {
            repository: "o/x".into(),
            name: "x".into(),
            description: format!("{} fim", "palavra, ".repeat(20)),
            category: "",
            category_label: String::new(),
            website: None,
            website_declared: None,
            website_status: "none",
            website_http_status: 0,
            website_source: "none".into(),
            website_final_url: String::new(),
            github: String::new(),
            primary_cta: "github",
            secondary_cta: None,
            status: String::new(),
            private: false,
            featured: false,
            marketing_priority: 0,
        };
        let cut = card_summary(&p);
        assert!(cut.ends_with("palavra…"), "{cut}");
        assert!(py_len(&cut) <= 151);
        p.description = "curta".into();
        assert_eq!("curta", card_summary(&p));
    }
}
