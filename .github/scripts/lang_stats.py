#!/usr/bin/env python3
"""
Bot de análise de linguagens e arquivos — ⬡ Projeto Baluarte / perfil.

Varre TODOS os repositórios do perfil — **incluindo os privados** — e gera:

  1. assets/lang-stats.svg  — card visual no tema "Ouro de Fábula"
  2. bloco no README.md entre os marcadores LANG-STATS: linguagens por peso,
     tipos de arquivo por contagem, e em quais repositórios cada um aparece.

⚠️ **Privado entra, e isso tem consequência.** Este README é público. Incluir
repositório privado nas estatísticas é decisão do dono do perfil (pedida
explicitamente); o efeito colateral é que o NOME dele aparece na seção de
detalhe. Quem não quiser isso liga `OCULTAR_NOMES_PRIVADOS=1`: os números
continuam completos e o nome vira `repositório privado`.

Para enxergar privado é preciso um **PAT com escopo `repo`** em
`LANG_STATS_TOKEN`. O `GITHUB_TOKEN` do Actions é token de instalação, sem
contexto de usuário — ele responde 401/403 em `/user/repos` e a análise cai no
endpoint público, que só lista os abertos. Sem o PAT, nada quebra: o relatório
diz quantos ficaram de fora, em vez de fingir que o número está completo.

Uso:
    GITHUB_TOKEN=... python3 .github/scripts/lang_stats.py

Variáveis de ambiente:
    GITHUB_TOKEN            token de leitura (PAT com `repo` para ver privados)
    GH_USER                 login do dono (padrão: Lucas-Belucci-Bellini)
    INCLUDE_FORKS           "1" para incluir forks (padrão: 0)
    OCULTAR_NOMES_PRIVADOS  "1" anonimiza o nome dos privados (padrão: 0)
    SEM_ARQUIVOS            "1" pula a varredura de tipos de arquivo (padrão: 0)
"""

from __future__ import annotations

import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

API = "https://api.github.com"
USER = os.environ.get("GH_USER", "Lucas-Belucci-Bellini")
TOKEN = os.environ.get("GITHUB_TOKEN", "")
INCLUDE_FORKS = os.environ.get("INCLUDE_FORKS", "0") == "1"
OCULTAR_PRIV = os.environ.get("OCULTAR_NOMES_PRIVADOS", "0") == "1"
SEM_ARQUIVOS = os.environ.get("SEM_ARQUIVOS", "0") == "1"

# Quantos tipos de arquivo listar na tabela. O resto vira uma linha "outros" —
# a cauda de extensões únicas é longa e não informa nada.
TOP_EXT = 24

# Extensão -> família, para a leitura ficar por assunto e não por acaso
# alfabético. Só agrupa o que é inequívoco; o que não está aqui cai em "outros"
# em vez de ser chutado para uma família plausível.
FAMILIA_EXT = {
    "código": {"js", "mjs", "cjs", "jsx", "ts", "tsx", "py", "sqf", "lua", "c",
               "h", "cpp", "hpp", "cs", "java", "go", "rs", "rb", "php", "kt",
               "swift", "sh", "bat", "ps1", "vue", "svelte", "dart", "ino"},
    "estilo e marcação": {"css", "scss", "sass", "less", "html", "htm", "xml",
                          "svg", "xsl"},
    "dado": {"json", "jsonl", "csv", "tsv", "yml", "yaml", "toml", "ini", "cfg",
             "sql", "db", "sqlite", "parquet", "rpt", "hpp_cfg"},
    "documento": {"md", "mdx", "txt", "rst", "pdf", "doc", "docx", "adoc"},
    "imagem": {"png", "jpg", "jpeg", "webp", "gif", "bmp", "ico", "tga", "paa",
               "psd", "avif"},
    "áudio e vídeo": {"mp3", "wav", "ogg", "flac", "mp4", "webm", "mov", "mkv"},
    "modelo 3D": {"glb", "gltf", "obj", "fbx", "stl", "p3d", "blend", "dae"},
    "fonte": {"ttf", "otf", "woff", "woff2", "eot"},
}
EXT_FAMILIA = {e: f for f, exts in FAMILIA_EXT.items() for e in exts}

ROOT = Path(__file__).resolve().parents[2]
README = ROOT / "README.md"
SVG_OUT = ROOT / "assets" / "lang-stats.svg"

START = "<!-- LANG-STATS:START -->"
END = "<!-- LANG-STATS:END -->"

# Paleta "Ouro de Fábula" (docs/DESIGN-SYSTEM.md do Projeto Baluarte)
BG = "#0e0c16"
SURFACE = "#1d1729"
GOLD = "#d4a24e"
GOLD_LIGHT = "#e8c07a"
PARCHMENT = "#f4ecdd"
MUTED = "#a89a80"
DIM = "#77694f"

# Cores por linguagem (tons que convivem com o dourado)
LANG_COLORS = {
    "JavaScript": "#e8c07a", "TypeScript": "#d4a24e", "Python": "#c9a227",
    "HTML": "#e07a5f", "CSS": "#a68dad", "C": "#8fa6c4", "C++": "#7f9ec4",
    "C#": "#9a8fc4", "Java": "#c48f6b", "Shell": "#8fc49a", "Batchfile": "#7ba88a",
    "PowerShell": "#8f9ec4", "Lua": "#8f8fc4", "Ruby": "#c47f8f",
    "Go": "#8fc4c4", "Rust": "#c4926b", "PHP": "#9b8fc4", "Vue": "#8fc4a3",
    "Svelte": "#d99a6b", "Dart": "#8fbcc4", "Kotlin": "#b98fc4",
    "Swift": "#d4926b", "Objective-C": "#8fa8c4", "Makefile": "#a3a3a3",
    "Dockerfile": "#8fb0c4", "JSON": "#b9a77f", "Jupyter Notebook": "#d4a24e",
    "GLSL": "#c48fa8", "ShaderLab": "#c49a8f", "HLSL": "#b08fc4",
    "SCSS": "#c48fb0", "Assembly": "#a89a80", "Arduino": "#7fbfae",
}
FALLBACK_CYCLE = [GOLD, GOLD_LIGHT, "#b9a77f", "#c9a227", "#a89a80", "#8fa6c4"]


def color_for(lang: str, idx: int) -> str:
    return LANG_COLORS.get(lang, FALLBACK_CYCLE[idx % len(FALLBACK_CYCLE)])


# ----------------------------------------------------------------- HTTP

def api(path: str, params: dict | None = None, retry_denied: bool = True) -> object:
    url = API + path
    if params:
        url += "?" + "&".join(f"{k}={v}" for k, v in params.items())
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "baluarte-lang-bot",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if TOKEN:
        headers["Authorization"] = f"Bearer {TOKEN}"

    for attempt in range(4):
        try:
            req = urllib.request.Request(url, headers=headers)
            with urllib.request.urlopen(req, timeout=45) as resp:
                return json.load(resp)
        except urllib.error.HTTPError as exc:
            # 403/429 costumam ser rate-limit secundário — espera e tenta de novo.
            # Com retry_denied=False o chamador quer decidir sozinho (ex.: fallback).
            if exc.code in (403, 429) and retry_denied and attempt < 3:
                time.sleep(2 ** (attempt + 1))
                continue
            if exc.code == 404 and retry_denied:
                return None
            raise
        except (urllib.error.URLError, TimeoutError):
            if attempt < 3:
                time.sleep(2 ** (attempt + 1))
                continue
            raise
    return None


def _paginate(path: str, params: dict, retry_denied: bool = True) -> list[dict]:
    out: list[dict] = []
    page = 1
    while True:
        batch = api(path, {**params, "per_page": "100", "page": str(page)},
                    retry_denied=retry_denied)
        if not batch:
            break
        out.extend(batch)
        if len(batch) < 100:
            break
        page += 1
    return out


def list_repos() -> list[dict]:
    """Todos os repos do dono do perfil.

    Com um PAT de usuário, /user/repos traz também os privados. O GITHUB_TOKEN
    do Actions é um token de instalação — sem contexto de usuário, ele responde
    401/403 nesse endpoint; nesse caso caímos no endpoint público, que funciona.
    """
    if TOKEN:
        try:
            return _paginate("/user/repos",
                             {"affiliation": "owner", "sort": "pushed"},
                             retry_denied=False)
        except urllib.error.HTTPError as exc:
            if exc.code not in (401, 403, 404):
                raise
            print("aviso: /user/repos indisponível para este token "
                  f"(HTTP {exc.code}); usando o endpoint público — "
                  "repositórios privados ficam de fora. Defina o secret "
                  "LANG_STATS_TOKEN (PAT com escopo `repo`) para incluí-los.",
                  file=sys.stderr)
    return _paginate(f"/users/{USER}/repos", {"type": "owner"})


# ----------------------------------------------------------------- coleta

def extensao(caminho: str) -> str | None:
    """Extensão em minúsculas, ou None quando não há uma que sirva de tipo.

    `Makefile` e `LICENSE` não têm extensão; `.gitignore` é só um ponto-nome, e
    tratá-lo como extensão `gitignore` encheria a tabela de ruído de
    configuração. Os dois casos viram None de propósito."""
    nome = caminho.rsplit("/", 1)[-1]
    if "." not in nome or nome.startswith("."):
        return None
    ext = nome.rsplit(".", 1)[1].lower()
    # extensão absurda quase sempre é nome com ponto, não tipo de arquivo
    if not ext or len(ext) > 12 or not ext.isalnum():
        return None
    return ext


def arquivos_do_repo(owner: str, name: str, branch: str) -> tuple[dict, bool]:
    """({extensão: contagem}, truncado?) lendo a árvore do branch padrão.

    Uma chamada por repositório. `truncated` vem do próprio GitHub quando a
    árvore passa do limite: aí a contagem é PARCIAL, e isso é reportado em vez
    de virar um número que parece completo."""
    if not branch:
        return {}, False
    arv = api(f"/repos/{owner}/{name}/git/trees/{branch}",
              {"recursive": "1"})
    if not isinstance(arv, dict):
        return {}, False
    contagem: dict[str, int] = {}
    for no in arv.get("tree") or []:
        if no.get("type") != "blob":
            continue
        ext = extensao(no.get("path", ""))
        if ext:
            contagem[ext] = contagem.get(ext, 0) + 1
    return contagem, bool(arv.get("truncated"))


def collect() -> dict:
    repos = list_repos()
    per_lang: dict[str, int] = {}
    per_lang_repos: dict[str, list[tuple[str, int, bool]]] = {}
    per_ext: dict[str, int] = {}
    per_ext_repos: dict[str, set[str]] = {}

    vistos = privados = sem_linguagem = truncados = 0
    arquivos_total = 0

    for repo in repos:
        if repo.get("fork") and not INCLUDE_FORKS:
            continue
        owner = repo["owner"]["login"]
        name = repo["name"]
        priv = bool(repo.get("private"))
        vistos += 1
        if priv:
            privados += 1

        langs = api(f"/repos/{owner}/{name}/languages") or {}
        if langs:
            for lang, size in langs.items():
                per_lang[lang] = per_lang.get(lang, 0) + size
                per_lang_repos.setdefault(lang, []).append((name, size, priv))
        else:
            # Repositório sem linguagem detectada ainda É um repositório. Antes
            # ele sumia da contagem, e o total de repos ficava menor que o real.
            sem_linguagem += 1

        if not SEM_ARQUIVOS:
            exts, truncado = arquivos_do_repo(
                owner, name, repo.get("default_branch") or "")
            truncados += 1 if truncado else 0
            for ext, n in exts.items():
                per_ext[ext] = per_ext.get(ext, 0) + n
                per_ext_repos.setdefault(ext, set()).add(name)
                arquivos_total += n

    for lang in per_lang_repos:
        per_lang_repos[lang].sort(key=lambda t: t[1], reverse=True)

    print(f"repos={vistos} privados={privados} sem_linguagem={sem_linguagem} "
          f"arquivos={arquivos_total} arvores_truncadas={truncados}")

    return {
        "per_lang": dict(sorted(per_lang.items(), key=lambda kv: kv[1], reverse=True)),
        "per_lang_repos": per_lang_repos,
        "per_ext": dict(sorted(per_ext.items(), key=lambda kv: kv[1], reverse=True)),
        "per_ext_repos": per_ext_repos,
        "repo_count": vistos,
        "privados": privados,
        "sem_linguagem": sem_linguagem,
        "truncados": truncados,
        "arquivos_total": arquivos_total,
        "total_bytes": sum(per_lang.values()),
        "generated": datetime.now(timezone.utc),
    }


# ----------------------------------------------------------------- formato

def human(n: int) -> str:
    if n >= 1024 ** 3:
        return f"{n / 1024 ** 3:.2f} GB"
    if n >= 1024 ** 2:
        return f"{n / 1024 ** 2:.2f} MB"
    if n >= 1024:
        return f"{n / 1024:.1f} KB"
    return f"{n} B"


def esc(s: str) -> str:
    return (s.replace("&", "&amp;").replace("<", "&lt;")
             .replace(">", "&gt;").replace('"', "&quot;"))


# ----------------------------------------------------------------- SVG

def build_svg(data: dict) -> str:
    langs = list(data["per_lang"].items())[:10]
    total = data["total_bytes"] or 1
    n_langs = len(data["per_lang"])

    row_h = 26
    top_y = 132
    height = top_y + row_h * len(langs) + 34
    width = 880
    bar_x, bar_w = 250, 500

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" '
        f'width="{width}" height="{height}" font-family="\'IBM Plex Mono\',monospace" '
        f'role="img" aria-label="Análise de linguagens do perfil">',
        '<defs>'
        f'<linearGradient id="bgg" x1="0" y1="0" x2="1" y2="1">'
        f'<stop offset="0" stop-color="{BG}"/><stop offset="1" stop-color="#0b0910"/></linearGradient>'
        f'<linearGradient id="scanl" x1="0" y1="0" x2="1" y2="0">'
        f'<stop offset="0" stop-color="{GOLD}" stop-opacity="0"/>'
        f'<stop offset="0.5" stop-color="{GOLD_LIGHT}" stop-opacity="0.9"/>'
        f'<stop offset="1" stop-color="{GOLD}" stop-opacity="0"/></linearGradient>'
        '</defs>',
        f'<rect width="{width}" height="{height}" rx="14" fill="url(#bgg)"/>',
        f'<rect x="5" y="5" width="{width-10}" height="{height-10}" rx="11" fill="none" '
        f'stroke="{GOLD}" stroke-opacity="0.35"/>',
        # cantos HUD
        f'<g stroke="{GOLD_LIGHT}" stroke-width="2" fill="none" stroke-linecap="round">'
        f'<path d="M22 40 V22 H40"/><path d="M{width-40} 22 H{width-22} V40"/>'
        f'<path d="M22 {height-40} V{height-22} H40"/>'
        f'<path d="M{width-22} {height-40} V{height-22} H{width-40}"/></g>',
        # título
        f'<text x="38" y="44" fill="{GOLD_LIGHT}" font-size="15" font-weight="700" '
        f'letter-spacing="2">&#x2B21; ARSENAL // AN&#xC1;LISE DE LINGUAGENS</text>',
        f'<line x1="24" y1="58" x2="{width-24}" y2="58" stroke="{GOLD}" stroke-opacity="0.25"/>',
        f'<rect x="24" y="59" width="200" height="2" fill="url(#scanl)" opacity="0.85">'
        f'<animate attributeName="x" values="24;{width-224};24" dur="8s" repeatCount="indefinite"/></rect>',
    ]

    # stats — quatro colunas desde que os tipos de arquivo entraram
    repos_lbl = str(data["repo_count"])
    if data.get("privados"):
        repos_lbl += f" ({data['privados']} priv)"
    stats = [
        (str(n_langs), "LINGUAGENS"),
        (str(len(data.get("per_ext") or {})), "TIPOS DE ARQUIVO"),
        (repos_lbl, "REPOSITÓRIOS"),
        (human(data["total_bytes"]), "CÓDIGO TOTAL"),
    ]
    sx = 38
    for val, label in stats:
        parts.append(
            f'<text x="{sx}" y="93" fill="{GOLD}" font-size="20" font-weight="700">{esc(val)}</text>'
            f'<text x="{sx}" y="110" fill="{DIM}" font-size="10" letter-spacing="1.5">{label}</text>'
        )
        sx += 210

    # barras
    y = top_y
    for i, (lang, size) in enumerate(langs):
        pct = size / total * 100
        w = max(3, int(bar_w * size / total))
        col = color_for(lang, i)
        parts.append(
            f'<text x="38" y="{y+13}" fill="{PARCHMENT}" font-size="12">{esc(lang)}</text>'
            f'<rect x="{bar_x}" y="{y+3}" width="{bar_w}" height="12" rx="6" fill="{SURFACE}"/>'
            f'<rect x="{bar_x}" y="{y+3}" width="{w}" height="12" rx="6" fill="{col}">'
            f'<animate attributeName="width" from="0" to="{w}" dur="1.1s" fill="freeze"/></rect>'
            f'<text x="{bar_x+bar_w+14}" y="{y+13}" fill="{MUTED}" font-size="11">'
            f'{pct:.1f}% &#183; {human(size)}</text>'
        )
        y += row_h

    stamp = data["generated"].strftime("%Y-%m-%d %H:%M UTC")
    parts.append(
        f'<text x="38" y="{height-14}" fill="{DIM}" font-size="10" letter-spacing="1">'
        f'atualizado automaticamente &#183; {stamp}</text>'
    )
    parts.append(
        f'<circle cx="{width-52}" cy="{height-18}" r="4" fill="#3ddc84">'
        f'<animate attributeName="opacity" values="1;0.3;1" dur="2s" repeatCount="indefinite"/></circle>'
        f'<text x="{width-40}" y="{height-14}" fill="#3ddc84" font-size="10">LIVE</text>'
    )
    parts.append("</svg>")
    return "\n".join(parts) + "\n"


# ----------------------------------------------------------------- Markdown

def rotulo_repo(name: str, priv: bool) -> str:
    """Como o repositório aparece na tabela pública."""
    if priv and OCULTAR_PRIV:
        return "🔒 _repositório privado_"
    if priv:
        return f"🔒 {name}"
    return f"[{name}](https://github.com/{USER}/{name})"


def build_markdown(data: dict) -> str:
    per_lang = data["per_lang"]
    per_ext = data["per_ext"]
    total = data["total_bytes"] or 1
    stamp = data["generated"].strftime("%d/%m/%Y %H:%M UTC")

    resumo = (f"> **{len(per_lang)} linguagens** e **{len(per_ext)} tipos de arquivo** "
              f"em **{data['repo_count']} repositórios**")
    if data["privados"]:
        resumo += f" (**{data['privados']}** privados)"
    resumo += (f" · **{human(data['total_bytes'])}** de código"
               f" · **{data['arquivos_total']:,}** arquivos".replace(",", "."))
    resumo += f" · atualizado em `{stamp}`"

    out = [
        START,
        "",
        resumo,
        "",
        '<div align="center">',
        "",
        "![Análise de linguagens](./assets/lang-stats.svg)",
        "",
        "</div>",
        "",
        "### Linguagens",
        "",
        "| # | Linguagem | Peso | % | Repositórios |",
        "| :-- | :--- | ---: | ---: | ---: |",
    ]

    for i, (lang, size) in enumerate(per_lang.items(), 1):
        pct = size / total * 100
        n_repos = len(data["per_lang_repos"].get(lang, []))
        out.append(f"| {i} | **{lang}** | `{human(size)}` | `{pct:.2f}%` | {n_repos} |")

    if data["sem_linguagem"]:
        out += ["", f"> {data['sem_linguagem']} repositório(s) sem linguagem "
                    f"detectada pelo GitHub — contam no total, mas não na tabela."]

    # ── tipos de arquivo ──────────────────────────────────────────────────
    if per_ext:
        arq_total = data["arquivos_total"] or 1
        # Ordena aqui também: a tabela diz "top", e depender da ordem que o
        # chamador entregou faria um "top" errado sem nenhum sinal.
        ordenado = sorted(per_ext.items(), key=lambda kv: (-kv[1], kv[0]))
        out += [
            "",
            "### Tipos de arquivo",
            "",
            "| # | Tipo | Arquivos | % | Família | Repositórios |",
            "| :-- | :--- | ---: | ---: | :--- | ---: |",
        ]
        for i, (ext, n) in enumerate(ordenado[:TOP_EXT], 1):
            pct = n / arq_total * 100
            fam = EXT_FAMILIA.get(ext, "outros")
            nrep = len(data["per_ext_repos"].get(ext, ()))
            out.append(f"| {i} | `.{ext}` | `{n}` | `{pct:.2f}%` | {fam} | {nrep} |")

        resto = ordenado[TOP_EXT:]
        if resto:
            n_resto = sum(n for _e, n in resto)
            out.append(f"| | _… +{len(resto)} outros tipos_ | `{n_resto}` | "
                       f"`{n_resto / arq_total * 100:.2f}%` | | |")

        # por família — a leitura por assunto
        por_fam: dict[str, int] = {}
        for ext, n in per_ext.items():
            por_fam[EXT_FAMILIA.get(ext, "outros")] = \
                por_fam.get(EXT_FAMILIA.get(ext, "outros"), 0) + n
        out += ["", "| Família | Arquivos | % |", "| :--- | ---: | ---: |"]
        for fam, n in sorted(por_fam.items(), key=lambda kv: kv[1], reverse=True):
            out.append(f"| {fam} | `{n}` | `{n / arq_total * 100:.2f}%` |")

        if data["truncados"]:
            out += ["", f"> ⚠️ {data['truncados']} repositório(s) têm árvore grande "
                        f"demais para uma leitura só — a contagem de arquivos deles "
                        f"é **parcial**. O GitHub trunca; melhor dizer do que "
                        f"apresentar número incompleto como se fosse total."]

    # ── detalhe ───────────────────────────────────────────────────────────
    out += [
        "",
        "<details>",
        "<summary><b>🗺 Onde cada linguagem foi usada / Where each language was used</b></summary>",
        "",
    ]

    for lang, size in per_lang.items():
        repos = data["per_lang_repos"].get(lang, [])
        out.append(f"#### {lang} — `{human(size)}`")
        out.append("")
        out.append("| Repositório | Peso |")
        out.append("| :--- | ---: |")
        for name, rsize, priv in repos[:12]:
            out.append(f"| {rotulo_repo(name, priv)} | `{human(rsize)}` |")
        if len(repos) > 12:
            out.append(f"| _… +{len(repos) - 12} repositórios_ | |")
        out.append("")

    out += ["</details>", "", END]
    return "\n".join(out)


def patch_readme(block: str) -> bool:
    text = README.read_text(encoding="utf-8")
    if START in text and END in text:
        new = re.sub(
            re.escape(START) + r".*?" + re.escape(END),
            lambda _m: block,
            text,
            flags=re.S,
        )
    else:
        print("!! marcadores LANG-STATS não encontrados no README", file=sys.stderr)
        return False
    if new == text:
        return False
    README.write_text(new, encoding="utf-8")
    return True


def main() -> int:
    data = collect()
    if not data["per_lang"]:
        print("Nenhuma linguagem coletada — abortando sem alterar arquivos.", file=sys.stderr)
        return 1

    SVG_OUT.parent.mkdir(parents=True, exist_ok=True)
    svg = build_svg(data)
    svg_changed = (not SVG_OUT.exists()) or SVG_OUT.read_text(encoding="utf-8") != svg
    if svg_changed:
        SVG_OUT.write_text(svg, encoding="utf-8")

    md_changed = patch_readme(build_markdown(data))

    print(f"linguagens={len(data['per_lang'])} repos={data['repo_count']} "
          f"bytes={data['total_bytes']} svg_changed={svg_changed} md_changed={md_changed}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
