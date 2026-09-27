//! `assets/lang-stats.svg` e `assets/profile-top-langs.svg` — as funções de
//! desenho do `.github/scripts/lang_stats.py` (`build_svg`,
//! `build_profile_top_langs_svg`, `mesmo_conteudo`).
//!
//! A coleta (repositórios, linguagens, árvores git) fica no `profile-core`;
//! aqui entra o que ela produz e sai o SVG com os mesmos bytes. A prova é o
//! fixture `tests/fixtures/parity/assets.json`, gerado chamando as funções
//! Python.

use std::sync::LazyLock;

use chrono::{Datelike, NaiveDateTime, Timelike};

/// Paleta "Ouro de Fábula".
const BG: &str = "#0e0c16";
const SURFACE: &str = "#1d1729";
const GOLD: &str = "#d4a24e";
const GOLD_LIGHT: &str = "#e8c07a";
const PARCHMENT: &str = "#f4ecdd";
const MUTED: &str = "#a89a80";
const DIM: &str = "#77694f";
const GREEN: &str = "#3ddc84";
const RED: &str = "#e07a5f";
const PURPLE: &str = "#a68dad";

/// `LANG_COLORS`.
pub const LANG_COLORS: [(&str, &str); 33] = [
    ("JavaScript", "#e8c07a"),
    ("TypeScript", "#d4a24e"),
    ("Python", "#c9a227"),
    ("HTML", "#e07a5f"),
    ("CSS", "#a68dad"),
    ("C", "#8fa6c4"),
    ("C++", "#7f9ec4"),
    ("C#", "#9a8fc4"),
    ("Java", "#c48f6b"),
    ("Shell", "#8fc49a"),
    ("Batchfile", "#7ba88a"),
    ("PowerShell", "#8f9ec4"),
    ("Lua", "#8f8fc4"),
    ("Ruby", "#c47f8f"),
    ("Go", "#8fc4c4"),
    ("Rust", "#c4926b"),
    ("PHP", "#9b8fc4"),
    ("Vue", "#8fc4a3"),
    ("Svelte", "#d99a6b"),
    ("Dart", "#8fbcc4"),
    ("Kotlin", "#b98fc4"),
    ("Swift", "#d4926b"),
    ("Objective-C", "#8fa8c4"),
    ("Makefile", "#a3a3a3"),
    ("Dockerfile", "#8fb0c4"),
    ("JSON", "#b9a77f"),
    ("Jupyter Notebook", "#d4a24e"),
    ("GLSL", "#c48fa8"),
    ("ShaderLab", "#c49a8f"),
    ("HLSL", "#b08fc4"),
    ("SCSS", "#c48fb0"),
    ("Assembly", "#a89a80"),
    ("Arduino", "#7fbfae"),
];

/// `FALLBACK_CYCLE`: cor de quem não está em [`LANG_COLORS`], pela posição.
pub const FALLBACK_CYCLE: [&str; 6] = [GOLD, GOLD_LIGHT, "#b9a77f", "#c9a227", "#a89a80", "#8fa6c4"];

/// `FAMILY_COLORS`.
pub const FAMILY_COLORS: [(&str, &str); 9] = [
    ("código", "#e8c07a"),
    ("dado", "#a68dad"),
    ("imagem", "#3ddc84"),
    ("modelo 3D", "#e07a5f"),
    ("documento", "#8fa6c4"),
    ("estilo e marcação", "#d4a24e"),
    ("áudio e vídeo", "#c4926b"),
    ("fonte", "#9b8fc4"),
    ("outros", "#77694f"),
];

/// `EXT_FAMILIA`: extensão → família (o que não está aqui é "outros").
pub const EXT_FAMILY: [(&str, &str); 92] = [
    ("adoc", "documento"),
    ("avif", "imagem"),
    ("bat", "código"),
    ("blend", "modelo 3D"),
    ("bmp", "imagem"),
    ("c", "código"),
    ("cfg", "dado"),
    ("cjs", "código"),
    ("cpp", "código"),
    ("cs", "código"),
    ("css", "estilo e marcação"),
    ("csv", "dado"),
    ("dae", "modelo 3D"),
    ("dart", "código"),
    ("db", "dado"),
    ("doc", "documento"),
    ("docx", "documento"),
    ("eot", "fonte"),
    ("fbx", "modelo 3D"),
    ("flac", "áudio e vídeo"),
    ("gif", "imagem"),
    ("glb", "modelo 3D"),
    ("gltf", "modelo 3D"),
    ("go", "código"),
    ("h", "código"),
    ("hpp", "código"),
    ("hpp_cfg", "dado"),
    ("htm", "estilo e marcação"),
    ("html", "estilo e marcação"),
    ("ico", "imagem"),
    ("ini", "dado"),
    ("ino", "código"),
    ("java", "código"),
    ("jpeg", "imagem"),
    ("jpg", "imagem"),
    ("js", "código"),
    ("json", "dado"),
    ("jsonl", "dado"),
    ("jsx", "código"),
    ("kt", "código"),
    ("less", "estilo e marcação"),
    ("lua", "código"),
    ("md", "documento"),
    ("mdx", "documento"),
    ("mjs", "código"),
    ("mkv", "áudio e vídeo"),
    ("mov", "áudio e vídeo"),
    ("mp3", "áudio e vídeo"),
    ("mp4", "áudio e vídeo"),
    ("obj", "modelo 3D"),
    ("ogg", "áudio e vídeo"),
    ("otf", "fonte"),
    ("p3d", "modelo 3D"),
    ("paa", "imagem"),
    ("parquet", "dado"),
    ("pdf", "documento"),
    ("php", "código"),
    ("png", "imagem"),
    ("ps1", "código"),
    ("psd", "imagem"),
    ("py", "código"),
    ("rb", "código"),
    ("rpt", "dado"),
    ("rs", "código"),
    ("rst", "documento"),
    ("sass", "estilo e marcação"),
    ("scss", "estilo e marcação"),
    ("sh", "código"),
    ("sqf", "código"),
    ("sql", "dado"),
    ("sqlite", "dado"),
    ("stl", "modelo 3D"),
    ("svelte", "código"),
    ("svg", "estilo e marcação"),
    ("swift", "código"),
    ("tga", "imagem"),
    ("toml", "dado"),
    ("ts", "código"),
    ("tsv", "dado"),
    ("tsx", "código"),
    ("ttf", "fonte"),
    ("txt", "documento"),
    ("vue", "código"),
    ("wav", "áudio e vídeo"),
    ("webm", "áudio e vídeo"),
    ("webp", "imagem"),
    ("woff", "fonte"),
    ("woff2", "fonte"),
    ("xml", "estilo e marcação"),
    ("xsl", "estilo e marcação"),
    ("yaml", "dado"),
    ("yml", "dado"),
];

fn lookup(table: &'static [(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    table.iter().find(|(name, _)| *name == key).map(|(_, value)| *value)
}

/// `color_for(lang, idx)`.
pub fn color_for(language: &str, index: usize) -> &'static str {
    lookup(&LANG_COLORS, language).unwrap_or(FALLBACK_CYCLE[index % FALLBACK_CYCLE.len()])
}

/// `EXT_FAMILIA.get(ext, "outros")`.
pub fn family(extension: &str) -> &'static str {
    lookup(&EXT_FAMILY, extension).unwrap_or("outros")
}

fn family_color(family: &str) -> &'static str {
    lookup(&FAMILY_COLORS, family).unwrap_or(DIM)
}

/// O que a coleta produz (`collect()` do Python), já na ordem dele.
#[derive(Debug, Clone, PartialEq)]
pub struct LangStats {
    /// `per_lang`: bytes por linguagem, decrescente (empate: ordem de chegada).
    pub per_lang: Vec<(String, i64)>,
    /// `per_ext`: arquivos por extensão, decrescente (empate: ordem de chegada).
    pub per_ext: Vec<(String, i64)>,
    /// Repositórios vistos.
    pub repo_count: i64,
    /// Arquivos contados.
    pub arquivos_total: i64,
    /// Soma de `per_lang`.
    pub total_bytes: i64,
    /// O relógio, no fuso em que foi dado (é o que o `strftime` imprime).
    pub generated: NaiveDateTime,
}

/// `human(n)`: B, KB, MB ou GB.
pub fn human(value: i64) -> String {
    const KB: i64 = 1024;
    if value >= KB * KB * KB {
        format!("{:.2} GB", value as f64 / (KB * KB * KB) as f64)
    } else if value >= KB * KB {
        format!("{:.2} MB", value as f64 / (KB * KB) as f64)
    } else if value >= KB {
        format!("{:.1} KB", value as f64 / KB as f64)
    } else {
        format!("{value} B")
    }
}

/// `f"{n:,}"`: milhar com vírgula (o chamador troca por ponto).
pub fn thousands(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    if value < 0 { format!("-{out}") } else { out }
}

/// `esc(s)` do `lang_stats.py`: `&`, `<`, `>` e `"` (o apóstrofo passa).
pub fn esc(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// `strftime("%Y-%m-%d %H:%M UTC")` (o ano sem zeros à esquerda, como o glibc).
pub fn stamp(when: NaiveDateTime) -> String {
    format!("{}-{:02}-{:02} {:02}:{:02} UTC", when.year(), when.month(), when.day(), when.hour(), when.minute())
}

/// `int(x)`: trunca em direção a zero.
fn py_int(value: f64) -> i64 {
    value.trunc() as i64
}

/// `max(3, int(scale * value / top))`.
fn bar(scale: i64, value: i64, top: i64) -> i64 {
    py_int((scale * value) as f64 / top as f64).max(3)
}

fn or_one(value: i64) -> i64 {
    if value == 0 { 1 } else { value }
}

/// `build_svg(data)`: o painel de linguagens, tipos de arquivo e famílias.
pub fn build_svg(data: &LangStats) -> String {
    let (width, height) = (880_i64, 660_i64);
    let total = or_one(data.total_bytes);
    let arq_total = or_one(data.arquivos_total);
    let langs = &data.per_lang[..data.per_lang.len().min(8)];
    let mut ext_order = data.per_ext.clone();
    ext_order.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut parts = vec![
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" font-family="DejaVu Sans Mono,monospace" role="img" aria-label="Painel visual de linguagens, arquivos e famílias do perfil">"#
        ),
        "<defs>".to_string(),
        format!(
            r##"<linearGradient id="ecosystem-bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{BG}"/><stop offset="1" stop-color="#0b0910"/></linearGradient>"##
        ),
        format!(
            r#"<linearGradient id="ecosystem-scan" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="{GOLD}" stop-opacity="0"/><stop offset="0.5" stop-color="{GOLD_LIGHT}" stop-opacity="0.9"/><stop offset="1" stop-color="{GOLD}" stop-opacity="0"/></linearGradient>"#
        ),
        "</defs>".to_string(),
        format!(r#"<rect width="{width}" height="{height}" rx="14" fill="url(#ecosystem-bg)"/>"#),
        format!(
            r#"<rect x="5" y="5" width="{}" height="{}" rx="11" fill="none" stroke="{GOLD}" stroke-opacity="0.35"/>"#,
            width - 10,
            height - 10
        ),
        format!(
            r#"<g stroke="{GOLD_LIGHT}" stroke-width="2" fill="none" stroke-linecap="round"><path d="M22 40 V22 H40"/><path d="M{} 22 H{} V40"/><path d="M22 {} V{} H40"/><path d="M{} {} V{} H{}"/></g>"#,
            width - 40,
            width - 22,
            height - 40,
            height - 22,
            width - 22,
            height - 40,
            height - 22,
            width - 40
        ),
        format!(
            r#"<text x="38" y="42" fill="{GOLD_LIGHT}" font-size="15" font-weight="700" letter-spacing="2">&#x2B21; ARSENAL // AN&#xC1;LISE DO ECOSSISTEMA</text>"#
        ),
        format!(
            r#"<text x="38" y="59" fill="{DIM}" font-size="10" letter-spacing="1.2">linguagens · arquivos · famílias · leitura rápida do portfólio</text>"#
        ),
        format!(r#"<line x1="24" y1="71" x2="{}" y2="71" stroke="{GOLD}" stroke-opacity="0.25"/>"#, width - 24),
        format!(
            r#"<rect x="24" y="72" width="200" height="2" fill="url(#ecosystem-scan)" opacity="0.85"><animate attributeName="x" values="24;{};24" dur="8s" repeatCount="indefinite"/></rect>"#,
            width - 224
        ),
    ];

    let stats = [
        (data.per_lang.len().to_string(), "LINGUAGENS", GOLD_LIGHT),
        (data.per_ext.len().to_string(), "TIPOS", GOLD),
        (data.repo_count.to_string(), "REPOSITÓRIOS", GREEN),
        (human(data.total_bytes), "CÓDIGO", PURPLE),
        (thousands(data.arquivos_total).replace(',', "."), "ARQUIVOS", RED),
    ];
    let card_x = [24, 194, 364, 534, 704];
    let card_w = [160, 160, 160, 160, 152];
    for ((x, w), (value, label, color)) in card_x.iter().zip(card_w).zip(stats) {
        parts.push(format!(
            r#"<rect x="{x}" y="86" width="{w}" height="58" rx="8" fill="{SURFACE}" stroke="{GOLD}" stroke-opacity="0.22"/><text x="{}" y="114" fill="{color}" font-size="20" font-weight="700">{}</text><text x="{}" y="132" fill="{MUTED}" font-size="9" letter-spacing="1">{label}</text>"#,
            x + 14,
            esc(&value),
            x + 14
        ));
    }

    let (panel_y, panel_h) = (164_i64, 330_i64);
    for (x, title, subtitle) in
        [(24, "LINGUAGENS DOMINANTES", "peso no código · top 8"), (452, "TIPOS DE ARQUIVO", "contagem · top 10")]
    {
        parts.push(format!(
            r#"<rect x="{x}" y="{panel_y}" width="404" height="{panel_h}" rx="10" fill="{SURFACE}" stroke="{GOLD}" stroke-opacity="0.22"/><text x="{}" y="{}" fill="{GOLD_LIGHT}" font-size="12" font-weight="700" letter-spacing="1.4">{title}</text><text x="{}" y="{}" fill="{DIM}" font-size="9" letter-spacing="1">{subtitle}</text><line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{GOLD}" stroke-opacity="0.18"/>"#,
            x + 18,
            panel_y + 28,
            x + 18,
            panel_y + 45,
            x + 16,
            panel_y + 56,
            x + 388,
            panel_y + 56
        ));
    }

    for (i, (language, size)) in langs.iter().enumerate() {
        let y = panel_y + 80 + i as i64 * 29;
        let pct = *size as f64 / total as f64 * 100.0;
        parts.push(format!(
            r##"<text x="42" y="{y}" fill="{DIM}" font-size="9">{:02}</text><text x="64" y="{y}" fill="{PARCHMENT}" font-size="10">{}</text><rect x="178" y="{}" width="176" height="10" rx="5" fill="#0e0c16"/><rect x="178" y="{}" width="{}" height="10" rx="5" fill="{}"/><text x="370" y="{y}" fill="{MUTED}" font-size="9" text-anchor="end">{pct:.1}%</text><text x="410" y="{y}" fill="{DIM}" font-size="9" text-anchor="end">{}</text>"##,
            i + 1,
            esc(language),
            y - 10,
            y - 10,
            bar(176, *size, total),
            color_for(language, i),
            human(*size)
        ));
    }

    let top_ext_max = ext_order.first().map_or(1, |(_, count)| *count);
    for (i, (ext, count)) in ext_order.iter().take(10).enumerate() {
        let y = panel_y + 80 + i as i64 * 24;
        let pct = *count as f64 / arq_total as f64 * 100.0;
        let color = family_color(family(ext));
        // No Python o `.replace(',', '.')` pega a linha inteira (literais
        // adjacentes viram um só antes do método), não só a contagem.
        let row = format!(
            r##"<circle cx="466" cy="{}" r="4" fill="{color}"/><text x="480" y="{y}" fill="{PARCHMENT}" font-size="10">.{}</text><rect x="548" y="{}" width="176" height="10" rx="5" fill="#0e0c16"/><rect x="548" y="{}" width="{}" height="10" rx="5" fill="{color}"/><text x="740" y="{y}" fill="{MUTED}" font-size="9" text-anchor="end">{pct:.1}%</text><text x="840" y="{y}" fill="{DIM}" font-size="9" text-anchor="end">{}"##,
            y - 4,
            esc(ext),
            y - 10,
            y - 10,
            bar(176, *count, top_ext_max),
            thousands(*count)
        );
        parts.push(format!("{}</text>", row.replace(',', ".")));
    }

    let mut by_family: Vec<(&str, i64)> = Vec::new();
    for (ext, count) in &data.per_ext {
        let name = family(ext);
        match by_family.iter_mut().find(|(known, _)| *known == name) {
            Some((_, total)) => *total += count,
            None => by_family.push((name, *count)),
        }
    }
    by_family.sort_by(|a, b| b.1.cmp(&a.1));
    let family_y = 518_i64;
    for (i, (name, count)) in by_family.iter().take(4).enumerate() {
        let x = 24 + i as i64 * 214;
        let pct = *count as f64 / arq_total as f64 * 100.0;
        let color = family_color(name);
        let head = format!(
            r#"<rect x="{x}" y="{family_y}" width="198" height="70" rx="8" fill="{SURFACE}" stroke="{GOLD}" stroke-opacity="0.22"/><circle cx="{}" cy="{}" r="5" fill="{color}"/><text x="{}" y="{}" fill="{PARCHMENT}" font-size="10" font-weight="700">{}</text><text x="{}" y="{}" fill="{GOLD_LIGHT}" font-size="15" font-weight="700">{}"#,
            x + 18,
            family_y + 23,
            x + 31,
            family_y + 26,
            esc(&name.to_uppercase()),
            x + 18,
            family_y + 48,
            thousands(*count)
        );
        let tail = format!(
            r##"</text><text x="{}" y="{}" fill="{MUTED}" font-size="9">arquivos · {pct:.1}%</text><rect x="{}" y="{}" width="162" height="4" rx="2" fill="#0e0c16"/><rect x="{}" y="{}" width="{}" height="4" rx="2" fill="{color}"/>"##,
            x + 72,
            family_y + 48,
            x + 18,
            family_y + 57,
            x + 18,
            family_y + 57,
            py_int(162.0 * pct / 100.0).max(3)
        );
        parts.push(format!("{}{tail}", head.replace(',', ".")));
    }

    parts.push(format!(
        r#"<text x="38" y="634" fill="{DIM}" font-size="9" letter-spacing="1">atualizado automaticamente · {} · detalhes abaixo para auditoria completa</text><circle cx="824" cy="630" r="4" fill="{GREEN}"><animate attributeName="opacity" values="1;0.3;1" dur="2s" repeatCount="indefinite"/></circle><text x="836" y="634" fill="{GREEN}" font-size="9">LIVE</text></svg>"#,
        stamp(data.generated)
    ));
    let mut text = parts.join("\n");
    text.push('\n');
    text
}

/// `build_profile_top_langs_svg(data)`: o card compacto das oito primeiras.
pub fn build_top_langs_svg(data: &LangStats) -> String {
    let langs = &data.per_lang[..data.per_lang.len().min(8)];
    let total = or_one(data.total_bytes);
    let (width, height) = (880_i64, 360_i64);
    let (bar_x, bar_w) = (250_i64, 500_i64);
    let (row_h, top_y) = (27_i64, 116_i64);
    let mut parts = vec![
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" font-family="'DejaVu Sans Mono',monospace" role="img" aria-label="Top Languages do perfil">"#
        ),
        "<defs>".to_string(),
        format!(
            r##"<linearGradient id="topbg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{BG}"/><stop offset="1" stop-color="#0b0910"/></linearGradient>"##
        ),
        format!(
            r#"<linearGradient id="topscan" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="{GOLD}" stop-opacity="0"/><stop offset="0.5" stop-color="{GOLD_LIGHT}" stop-opacity="0.9"/><stop offset="1" stop-color="{GOLD}" stop-opacity="0"/></linearGradient>"#
        ),
        "</defs>".to_string(),
        format!(r#"<rect width="{width}" height="{height}" rx="14" fill="url(#topbg)"/>"#),
        format!(
            r#"<rect x="5" y="5" width="{}" height="{}" rx="11" fill="none" stroke="{GOLD}" stroke-opacity="0.35"/>"#,
            width - 10,
            height - 10
        ),
        format!(
            r#"<text x="38" y="44" fill="{GOLD_LIGHT}" font-size="15" font-weight="700" letter-spacing="2">⬡ LANGUAGE MATRIX // TOP LANGUAGES</text>"#
        ),
        format!(
            r#"<text x="38" y="61" fill="{DIM}" font-size="10" letter-spacing="1.2">fonte local · análise automática dos repositórios públicos</text>"#
        ),
        format!(r#"<line x1="24" y1="74" x2="{}" y2="74" stroke="{GOLD}" stroke-opacity="0.25"/>"#, width - 24),
        format!(
            r#"<rect x="24" y="75" width="200" height="2" fill="url(#topscan)" opacity="0.85"><animate attributeName="x" values="24;{};24" dur="8s" repeatCount="indefinite"/></rect>"#,
            width - 224
        ),
        format!(
            r#"<text x="38" y="101" fill="{GOLD}" font-size="13" font-weight="700">{} linguagens</text>"#,
            data.per_lang.len()
        ),
        format!(
            r#"<text x="215" y="101" fill="{MUTED}" font-size="11">· {} de código · {} repositórios</text>"#,
            human(data.total_bytes),
            data.repo_count
        ),
    ];
    let mut y = top_y;
    for (i, (language, size)) in langs.iter().enumerate() {
        let pct = *size as f64 / total as f64 * 100.0;
        let w = bar(bar_w, *size, total);
        parts.push(format!(
            r#"<text x="38" y="{}" fill="{PARCHMENT}" font-size="12">{}</text><rect x="{bar_x}" y="{}" width="{bar_w}" height="12" rx="6" fill="{SURFACE}"/><rect x="{bar_x}" y="{}" width="{w}" height="12" rx="6" fill="{}"><animate attributeName="width" from="0" to="{w}" dur="1.1s" fill="freeze"/></rect><text x="{}" y="{}" fill="{MUTED}" font-size="11">{pct:.1}% &#183; {}</text>"#,
            y + 13,
            esc(language),
            y + 3,
            y + 3,
            color_for(language, i),
            bar_x + bar_w + 14,
            y + 13,
            human(*size)
        ));
        y += row_h;
    }
    parts.push(format!(
        r#"<text x="38" y="{}" fill="{DIM}" font-size="10" letter-spacing="1">atualizado automaticamente &#183; {}</text>"#,
        height - 14,
        stamp(data.generated)
    ));
    parts.push(format!(
        r##"<circle cx="{}" cy="{}" r="4" fill="#3ddc84"><animate attributeName="opacity" values="1;0.3;1" dur="2s" repeatCount="indefinite"/></circle><text x="{}" y="{}" fill="#3ddc84" font-size="10">LIVE</text>"##,
        width - 52,
        height - 18,
        width - 40,
        height - 14
    ));
    parts.push("</svg>".to_string());
    let mut text = parts.join("\n");
    text.push('\n');
    text
}

/// `CARIMBO_RE` — `\d` do Python casa qualquer dígito Unicode, como o do `regex`.
static STAMP: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2} UTC").expect("regex válida"));

/// `mesmo_conteudo(novo, antigo)`: igual, ignorando o carimbo de hora.
pub fn same_content(new: &str, old: &str) -> bool {
    STAMP.replace_all(new, "@") == STAMP.replace_all(old, "@")
}

/// `extensao(caminho)`: extensão em minúsculas, ou `None` quando não serve de tipo.
///
/// O `isalnum()` do Python aceita letras e números Unicode pela categoria
/// geral; o `is_alphanumeric()` do Rust usa as propriedades Alphabetic e
/// Numeric. Diferem só em marcas combinantes — extensão de arquivo real não tem.
pub fn extension(path: &str) -> Option<String> {
    let name = path.rsplit('/').next().unwrap_or(path);
    if !name.contains('.') || name.starts_with('.') {
        return None;
    }
    let ext = name.rsplit_once('.').map_or("", |(_, ext)| ext).to_lowercase();
    if ext.is_empty() || ext.chars().count() > 12 || !ext.chars().all(char::is_alphanumeric) {
        return None;
    }
    Some(ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milhar_e_peso() {
        assert_eq!("0", thousands(0));
        assert_eq!("999", thousands(999));
        assert_eq!("1,000", thousands(1000));
        assert_eq!("-1,234,567", thousands(-1_234_567));
        assert_eq!("1023 B", human(1023));
        assert_eq!("1.0 KB", human(1024));
        assert_eq!("1.00 MB", human(1024 * 1024));
        assert_eq!("2.50 GB", human(5 * 1024 * 1024 * 1024 / 2));
    }

    #[test]
    fn extensoes() {
        assert_eq!(Some("js".into()), extension("src/app.JS"));
        assert_eq!(None, extension("Makefile"));
        assert_eq!(None, extension("dir/.gitignore"));
        assert_eq!(None, extension("a.b-c"));
        assert_eq!(None, extension("arquivo."));
        assert_eq!(None, extension("x.abcdefghijklm"));
        assert_eq!(Some("ção".into()), extension("x.ÇÃO"));
        assert_eq!(Some("gz".into()), extension("dist/pacote.tar.gz"));
    }

    #[test]
    fn carimbo_nao_e_mudanca() {
        assert!(same_content("<t>2026-09-27 08:35 UTC</t><t>19</t>", "<t>2026-09-20 08:35 UTC</t><t>19</t>"));
        assert!(!same_content("<t>2026-09-27 08:35 UTC</t><t>20</t>", "<t>2026-09-20 08:35 UTC</t><t>19</t>"));
    }
}
