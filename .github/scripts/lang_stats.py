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
PROFILE_TOP_LANGS_OUT = ROOT / "assets" / "profile-top-langs.svg"

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
GREEN = "#3ddc84"
RED = "#e07a5f"
PURPLE = "#a68dad"

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
FAMILY_COLORS = {
    "código": "#e8c07a",
    "dado": "#a68dad",
    "imagem": "#3ddc84",
    "modelo 3D": "#e07a5f",
    "documento": "#8fa6c4",
    "estilo e marcação": "#d4a24e",
    "áudio e vídeo": "#c4926b",
    "fonte": "#9b8fc4",
    "outros": "#77694f",
}


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
            # 404 = não existe. 409 = existe e está VAZIO: é o que o GitHub
            # responde em git/trees de repositório sem nenhum commit. Os dois
            # são resposta legítima ("não há nada aqui"), não falha — antes o
            # 409 subia como exceção e derrubava a análise inteira por causa
            # de um único repositório vazio.
            if exc.code in (404, 409) and retry_denied:
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


def arquivos_do_repo(owner: str, name: str, branch: str) -> tuple[dict, bool, bool]:
    """({extensão: contagem}, truncado?, falhou?) lendo a árvore do branch padrão.

    Uma chamada por repositório. `truncated` vem do próprio GitHub quando a
    árvore passa do limite: aí a contagem é PARCIAL, e isso é reportado em vez
    de virar um número que parece completo.

    Repositório vazio ou sem branch padrão devolve contagem zerada e `falhou`
    falso: zero arquivo é o número certo, não um erro. `falhou` fica reservado
    para o que a API recusou por outro motivo — aí o número é incompleto e o
    relatório precisa dizer isso."""
    if not branch:
        return {}, False, False
    try:
        arv = api(f"/repos/{owner}/{name}/git/trees/{branch}",
                  {"recursive": "1"})
    except (urllib.error.HTTPError, urllib.error.URLError, TimeoutError) as exc:
        # Um repositório problemático não pode derrubar a análise dos outros 20.
        print(f"aviso: não consegui ler a árvore de {name} ({exc}) — "
              "ele conta no total, mas sem tipos de arquivo.", file=sys.stderr)
        return {}, False, True
    if not isinstance(arv, dict):
        return {}, False, False
    contagem: dict[str, int] = {}
    for no in arv.get("tree") or []:
        if no.get("type") != "blob":
            continue
        ext = extensao(no.get("path", ""))
        if ext:
            contagem[ext] = contagem.get(ext, 0) + 1
    return contagem, bool(arv.get("truncated")), False


def collect() -> dict:
    repos = list_repos()
    per_lang: dict[str, int] = {}
    per_lang_repos: dict[str, list[tuple[str, int, bool]]] = {}
    per_ext: dict[str, int] = {}
    per_ext_repos: dict[str, set[str]] = {}

    vistos = privados = sem_linguagem = truncados = falhados = 0
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

        falhou = False
        try:
            langs = api(f"/repos/{owner}/{name}/languages") or {}
        except (urllib.error.HTTPError, urllib.error.URLError, TimeoutError) as exc:
            print(f"aviso: não consegui ler as linguagens de {name} ({exc}).",
                  file=sys.stderr)
            langs = {}
            falhou = True

        if langs:
            for lang, size in langs.items():
                per_lang[lang] = per_lang.get(lang, 0) + size
                per_lang_repos.setdefault(lang, []).append((name, size, priv))
        elif not falhou:
            # Repositório sem linguagem detectada ainda É um repositório. Antes
            # ele sumia da contagem, e o total de repos ficava menor que o real.
            # Só entra aqui quando a API respondeu de verdade: chamada que
            # falhou não é "sem linguagem", é dado que faltou.
            sem_linguagem += 1

        if not SEM_ARQUIVOS:
            exts, truncado, falhou_arv = arquivos_do_repo(
                owner, name, repo.get("default_branch") or "")
            truncados += 1 if truncado else 0
            falhou = falhou or falhou_arv
            for ext, n in exts.items():
                per_ext[ext] = per_ext.get(ext, 0) + n
                per_ext_repos.setdefault(ext, set()).add(name)
                arquivos_total += n

        falhados += 1 if falhou else 0

    for lang in per_lang_repos:
        per_lang_repos[lang].sort(key=lambda t: t[1], reverse=True)

    print(f"repos={vistos} privados={privados} sem_linguagem={sem_linguagem} "
          f"arquivos={arquivos_total} arvores_truncadas={truncados} "
          f"repos_com_falha={falhados}")

    return {
        "per_lang": dict(sorted(per_lang.items(), key=lambda kv: kv[1], reverse=True)),
        "per_lang_repos": per_lang_repos,
        "per_ext": dict(sorted(per_ext.items(), key=lambda kv: kv[1], reverse=True)),
        "per_ext_repos": per_ext_repos,
        "repo_count": vistos,
        "privados": privados,
        "sem_linguagem": sem_linguagem,
        "truncados": truncados,
        "falhados": falhados,
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
    """Gera um dashboard compacto com KPIs, rankings e famílias."""
    width, height = 880, 660
    total = data["total_bytes"] or 1
    arq_total = data["arquivos_total"] or 1
    per_ext = data.get("per_ext") or {}
    langs = list(data["per_lang"].items())[:8]
    ext_order = sorted(per_ext.items(), key=lambda kv: (-kv[1], kv[0]))

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" font-family="DejaVu Sans Mono,monospace" role="img" aria-label="Painel visual de linguagens, arquivos e famílias do perfil">',
        '<defs>',
        f'<linearGradient id="ecosystem-bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{BG}"/><stop offset="1" stop-color="#0b0910"/></linearGradient>',
        f'<linearGradient id="ecosystem-scan" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="{GOLD}" stop-opacity="0"/><stop offset="0.5" stop-color="{GOLD_LIGHT}" stop-opacity="0.9"/><stop offset="1" stop-color="{GOLD}" stop-opacity="0"/></linearGradient>',
        '</defs>',
        f'<rect width="{width}" height="{height}" rx="14" fill="url(#ecosystem-bg)"/>',
        f'<rect x="5" y="5" width="{width-10}" height="{height-10}" rx="11" fill="none" stroke="{GOLD}" stroke-opacity="0.35"/>',
        f'<g stroke="{GOLD_LIGHT}" stroke-width="2" fill="none" stroke-linecap="round"><path d="M22 40 V22 H40"/><path d="M{width-40} 22 H{width-22} V40"/><path d="M22 {height-40} V{height-22} H40"/><path d="M{width-22} {height-40} V{height-22} H{width-40}"/></g>',
        f'<text x="38" y="42" fill="{GOLD_LIGHT}" font-size="15" font-weight="700" letter-spacing="2">&#x2B21; ARSENAL // AN&#xC1;LISE DO ECOSSISTEMA</text>',
        f'<text x="38" y="59" fill="{DIM}" font-size="10" letter-spacing="1.2">linguagens · arquivos · famílias · leitura rápida do portfólio</text>',
        f'<line x1="24" y1="71" x2="{width-24}" y2="71" stroke="{GOLD}" stroke-opacity="0.25"/>',
        f'<rect x="24" y="72" width="200" height="2" fill="url(#ecosystem-scan)" opacity="0.85"><animate attributeName="x" values="24;{width-224};24" dur="8s" repeatCount="indefinite"/></rect>',
    ]

    stats = [
        (str(len(data["per_lang"])), "LINGUAGENS", GOLD_LIGHT),
        (str(len(per_ext)), "TIPOS", GOLD),
        (str(data["repo_count"]), "REPOSITÓRIOS", GREEN),
        (human(data["total_bytes"]), "CÓDIGO", PURPLE),
        (f'{data["arquivos_total"]:,}'.replace(',', '.'), "ARQUIVOS", RED),
    ]
    card_x = [24, 194, 364, 534, 704]
    card_w = [160, 160, 160, 160, 152]
    for x, w, (value, label, color) in zip(card_x, card_w, stats):
        parts.append(
            f'<rect x="{x}" y="86" width="{w}" height="58" rx="8" fill="{SURFACE}" stroke="{GOLD}" stroke-opacity="0.22"/>'
            f'<text x="{x+14}" y="114" fill="{color}" font-size="20" font-weight="700">{esc(value)}</text>'
            f'<text x="{x+14}" y="132" fill="{MUTED}" font-size="9" letter-spacing="1">{label}</text>'
        )

    panel_y, panel_h = 164, 330
    for x, title, subtitle in [(24, "LINGUAGENS DOMINANTES", "peso no código · top 8"), (452, "TIPOS DE ARQUIVO", "contagem · top 10")]:
        parts.append(
            f'<rect x="{x}" y="{panel_y}" width="404" height="{panel_h}" rx="10" fill="{SURFACE}" stroke="{GOLD}" stroke-opacity="0.22"/>'
            f'<text x="{x+18}" y="{panel_y+28}" fill="{GOLD_LIGHT}" font-size="12" font-weight="700" letter-spacing="1.4">{title}</text>'
            f'<text x="{x+18}" y="{panel_y+45}" fill="{DIM}" font-size="9" letter-spacing="1">{subtitle}</text>'
            f'<line x1="{x+16}" y1="{panel_y+56}" x2="{x+388}" y2="{panel_y+56}" stroke="{GOLD}" stroke-opacity="0.18"/>'
        )

    for i, (lang, size) in enumerate(langs):
        y = panel_y + 80 + i * 29
        pct = size / total * 100
        bar_w = max(3, int(176 * size / total))
        col = color_for(lang, i)
        parts.append(
            f'<text x="42" y="{y}" fill="{DIM}" font-size="9">{i+1:02d}</text>'
            f'<text x="64" y="{y}" fill="{PARCHMENT}" font-size="10">{esc(lang)}</text>'
            f'<rect x="178" y="{y-10}" width="176" height="10" rx="5" fill="#0e0c16"/>'
            f'<rect x="178" y="{y-10}" width="{bar_w}" height="10" rx="5" fill="{col}"/>'
            f'<text x="370" y="{y}" fill="{MUTED}" font-size="9" text-anchor="end">{pct:.1f}%</text>'
            f'<text x="410" y="{y}" fill="{DIM}" font-size="9" text-anchor="end">{human(size)}</text>'
        )

    top_ext_max = ext_order[0][1] if ext_order else 1
    for i, (ext, count) in enumerate(ext_order[:10]):
        y = panel_y + 80 + i * 24
        pct = count / arq_total * 100
        family = EXT_FAMILIA.get(ext, "outros")
        col = FAMILY_COLORS.get(family, DIM)
        bar_w = max(3, int(176 * count / top_ext_max))
        parts.append(
            f'<circle cx="466" cy="{y-4}" r="4" fill="{col}"/>'
            f'<text x="480" y="{y}" fill="{PARCHMENT}" font-size="10">.{esc(ext)}</text>'
            f'<rect x="548" y="{y-10}" width="176" height="10" rx="5" fill="#0e0c16"/>'
            f'<rect x="548" y="{y-10}" width="{bar_w}" height="10" rx="5" fill="{col}"/>'
            f'<text x="740" y="{y}" fill="{MUTED}" font-size="9" text-anchor="end">{pct:.1f}%</text>'
            f'<text x="840" y="{y}" fill="{DIM}" font-size="9" text-anchor="end">{count:,}'.replace(',', '.') + '</text>'
        )

    por_fam: dict[str, int] = {}
    for ext, count in per_ext.items():
        family = EXT_FAMILIA.get(ext, "outros")
        por_fam[family] = por_fam.get(family, 0) + count
    families = sorted(por_fam.items(), key=lambda kv: kv[1], reverse=True)[:4]
    family_y = 518
    for i, (family, count) in enumerate(families):
        x = 24 + i * 214
        pct = count / arq_total * 100
        col = FAMILY_COLORS.get(family, DIM)
        parts.append(
            f'<rect x="{x}" y="{family_y}" width="198" height="70" rx="8" fill="{SURFACE}" stroke="{GOLD}" stroke-opacity="0.22"/>'
            f'<circle cx="{x+18}" cy="{family_y+23}" r="5" fill="{col}"/>'
            f'<text x="{x+31}" y="{family_y+26}" fill="{PARCHMENT}" font-size="10" font-weight="700">{esc(family.upper())}</text>'
            f'<text x="{x+18}" y="{family_y+48}" fill="{GOLD_LIGHT}" font-size="15" font-weight="700">{count:,}'.replace(',', '.') + '</text>'
            f'<text x="{x+72}" y="{family_y+48}" fill="{MUTED}" font-size="9">arquivos · {pct:.1f}%</text>'
            f'<rect x="{x+18}" y="{family_y+57}" width="162" height="4" rx="2" fill="#0e0c16"/>'
            f'<rect x="{x+18}" y="{family_y+57}" width="{max(3, int(162 * pct / 100))}" height="4" rx="2" fill="{col}"/>'
        )

    stamp = data["generated"].strftime("%Y-%m-%d %H:%M UTC")
    parts.append(
        f'<text x="38" y="634" fill="{DIM}" font-size="9" letter-spacing="1">atualizado automaticamente · {stamp} · detalhes abaixo para auditoria completa</text>'
        f'<circle cx="824" cy="630" r="4" fill="{GREEN}"><animate attributeName="opacity" values="1;0.3;1" dur="2s" repeatCount="indefinite"/></circle>'
        f'<text x="836" y="634" fill="{GREEN}" font-size="9">LIVE</text>'
        '</svg>'
    )
    return "\n".join(parts) + "\n"


def build_profile_top_langs_svg(data: dict) -> str:
    """Gera o card compacto local que substitui o serviço Top Languages."""
    langs = list(data["per_lang"].items())[:8]
    total = data["total_bytes"] or 1
    width, height = 880, 360
    bar_x, bar_w = 250, 500
    row_h, top_y = 27, 116
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" font-family="\'DejaVu Sans Mono\',monospace" role="img" aria-label="Top Languages do perfil">',
        '<defs>',
        f'<linearGradient id="topbg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{BG}"/><stop offset="1" stop-color="#0b0910"/></linearGradient>',
        f'<linearGradient id="topscan" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="{GOLD}" stop-opacity="0"/><stop offset="0.5" stop-color="{GOLD_LIGHT}" stop-opacity="0.9"/><stop offset="1" stop-color="{GOLD}" stop-opacity="0"/></linearGradient>',
        '</defs>',
        f'<rect width="{width}" height="{height}" rx="14" fill="url(#topbg)"/>',
        f'<rect x="5" y="5" width="{width-10}" height="{height-10}" rx="11" fill="none" stroke="{GOLD}" stroke-opacity="0.35"/>',
        f'<text x="38" y="44" fill="{GOLD_LIGHT}" font-size="15" font-weight="700" letter-spacing="2">⬡ LANGUAGE MATRIX // TOP LANGUAGES</text>',
        f'<text x="38" y="61" fill="{DIM}" font-size="10" letter-spacing="1.2">fonte local · análise automática dos repositórios públicos</text>',
        f'<line x1="24" y1="74" x2="{width-24}" y2="74" stroke="{GOLD}" stroke-opacity="0.25"/>',
        f'<rect x="24" y="75" width="200" height="2" fill="url(#topscan)" opacity="0.85"><animate attributeName="x" values="24;{width-224};24" dur="8s" repeatCount="indefinite"/></rect>',
        f'<text x="38" y="101" fill="{GOLD}" font-size="13" font-weight="700">{len(data["per_lang"])} linguagens</text>',
        f'<text x="215" y="101" fill="{MUTED}" font-size="11">· {human(data["total_bytes"])} de código · {data["repo_count"]} repositórios</text>',
    ]
    y = top_y
    for i, (lang, size) in enumerate(langs):
        pct = size / total * 100
        w = max(3, int(bar_w * size / total))
        col = color_for(lang, i)
        parts.append(
            f'<text x="38" y="{y+13}" fill="{PARCHMENT}" font-size="12">{esc(lang)}</text>'
            f'<rect x="{bar_x}" y="{y+3}" width="{bar_w}" height="12" rx="6" fill="{SURFACE}"/>'
            f'<rect x="{bar_x}" y="{y+3}" width="{w}" height="12" rx="6" fill="{col}"><animate attributeName="width" from="0" to="{w}" dur="1.1s" fill="freeze"/></rect>'
            f'<text x="{bar_x+bar_w+14}" y="{y+13}" fill="{MUTED}" font-size="11">{pct:.1f}% &#183; {human(size)}</text>'
        )
        y += row_h
    stamp = data["generated"].strftime("%Y-%m-%d %H:%M UTC")
    parts.append(f'<text x="38" y="{height-14}" fill="{DIM}" font-size="10" letter-spacing="1">atualizado automaticamente &#183; {stamp}</text>')
    parts.append(f'<circle cx="{width-52}" cy="{height-18}" r="4" fill="#3ddc84"><animate attributeName="opacity" values="1;0.3;1" dur="2s" repeatCount="indefinite"/></circle><text x="{width-40}" y="{height-14}" fill="#3ddc84" font-size="10">LIVE</text>')
    parts.append('</svg>')
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
    """Mantém o README limpo: resumo visual aberto, auditoria em detalhes."""
    per_lang = data["per_lang"]
    per_ext = data["per_ext"]
    total = data["total_bytes"] or 1
    stamp = data["generated"].strftime("%d/%m/%Y %H:%M UTC")

    resumo = (f"> **{len(per_lang)} linguagens** · **{len(per_ext)} tipos de arquivo** · "
              f"**{data['repo_count']} repositórios** · **{human(data['total_bytes'])}** de código · "
              f"**{data['arquivos_total']:,}** arquivos".replace(',', '.'))
    if data["privados"]:
        resumo += f" · **{data['privados']}** privados"
    resumo += f" · atualizado em `{stamp}`"

    out = [
        START,
        "",
        resumo,
        "",
        '<div align="center">',
        "",
        "![Análise visual do ecossistema](./assets/lang-stats.svg)",
        "",
        "</div>",
        "",
        "> **Leitura rápida:** o painel acima prioriza o que importa — volume, linguagens dominantes, formatos de arquivo e famílias do portfólio. A auditoria completa fica recolhida para manter o README elegante e rápido de ler.",
        "",
    ]

    if data["sem_linguagem"]:
        out += [f"> {data['sem_linguagem']} repositório(s) sem linguagem detectada pelo GitHub — contam no total, mas não na tabela detalhada.", ""]
    if data.get("falhados"):
        out += [f"> ⚠️ {data['falhados']} repositório(s) não responderam nesta rodada; os números são parciais e serão reavaliados na próxima execução.", ""]

    out += [
        "<details>",
        "<summary><b>⌁ Auditoria de linguagens e repositórios</b></summary>",
        "",
        "### Linguagens por peso",
        "",
        "| # | Linguagem | Peso | Participação | Repositórios |",
        "| :--: | :--- | ---: | ---: | ---: |",
    ]
    for i, (lang, size) in enumerate(per_lang.items(), 1):
        pct = size / total * 100
        n_repos = len(data["per_lang_repos"].get(lang, []))
        out.append(f"| {i} | **{lang}** | `{human(size)}` | `{pct:.2f}%` | {n_repos} |")
    out += ["", "#### Onde cada linguagem foi usada", ""]
    for lang, size in per_lang.items():
        repos = data["per_lang_repos"].get(lang, [])
        out.append(f"<details><summary><b>{lang}</b> · `{human(size)}` · {len(repos)} repositórios</summary>")
        out += ["", "| Repositório | Peso |", "| :--- | ---: |"]
        for name, rsize, priv in repos[:12]:
            out.append(f"| {rotulo_repo(name, priv)} | `{human(rsize)}` |")
        if len(repos) > 12:
            out.append(f"| _… +{len(repos) - 12} repositórios_ | |")
        out += ["", "</details>", ""]
    out += ["</details>", ""]

    if per_ext:
        arq_total = data["arquivos_total"] or 1
        ordenado = sorted(per_ext.items(), key=lambda kv: (-kv[1], kv[0]))
        out += [
            "<details>",
            "<summary><b>⌘ Auditoria de tipos de arquivo e famílias</b></summary>",
            "",
            "### Formatos mais frequentes",
            "",
            "| # | Tipo | Arquivos | Participação | Família | Repositórios |",
            "| :--: | :--- | ---: | ---: | :--- | ---: |",
        ]
        for i, (ext, n) in enumerate(ordenado[:TOP_EXT], 1):
            pct = n / arq_total * 100
            fam = EXT_FAMILIA.get(ext, "outros")
            nrep = len(data["per_ext_repos"].get(ext, ()))
            out.append(f"| {i} | `.{ext}` | `{n}` | `{pct:.2f}%` | {fam} | {nrep} |")
        resto = ordenado[TOP_EXT:]
        if resto:
            n_resto = sum(n for _e, n in resto)
            out.append(f"|  | _… +{len(resto)} outros tipos_ | `{n_resto}` | `{n_resto / arq_total * 100:.2f}%` |  |  |")

        por_fam: dict[str, int] = {}
        for ext, n in per_ext.items():
            fam = EXT_FAMILIA.get(ext, "outros")
            por_fam[fam] = por_fam.get(fam, 0) + n
        out += ["", "### Famílias de arquivo", "", "| Família | Arquivos | Participação |", "| :--- | ---: | ---: |"]
        for fam, n in sorted(por_fam.items(), key=lambda kv: kv[1], reverse=True):
            out.append(f"| {fam} | `{n}` | `{n / arq_total * 100:.2f}%` |")
        if data["truncados"]:
            out += ["", f"> ⚠️ {data['truncados']} repositório(s) têm árvore grande demais para uma leitura só; a contagem de arquivos deles é parcial."]
        out += ["", "</details>"]

    out += ["", END]
    return "\n".join(out)


# Os dois formatos de carimbo de hora que o bot escreve: `03/08/2026 00:01 UTC`
# no README e `2026-08-03 00:01 UTC` no SVG.
CARIMBO_RE = re.compile(
    r"\d{2}/\d{2}/\d{4} \d{2}:\d{2} UTC|\d{4}-\d{2}-\d{2} \d{2}:\d{2} UTC")


def mesmo_conteudo(novo: str, antigo: str) -> bool:
    """Igual, ignorando o carimbo de hora.

    O carimbo muda a cada execução por definição. Comparando com ele dentro, o
    bot commitava de hora em hora mesmo quando nenhum número tinha mudado —
    ~24 commits por dia de ruído no histórico do perfil. Fora da comparação, o
    carimbo passa a marcar a última vez que os DADOS mudaram, que é a
    informação que ele deveria estar dando desde o começo."""
    return CARIMBO_RE.sub("@", novo) == CARIMBO_RE.sub("@", antigo)


def patch_readme(block: str) -> bool:
    text = README.read_text(encoding="utf-8")
    achado = re.search(re.escape(START) + r".*?" + re.escape(END), text, flags=re.S)
    if not achado:
        print("!! marcadores LANG-STATS não encontrados no README", file=sys.stderr)
        return False
    if mesmo_conteudo(block, achado.group(0)):
        return False
    # Fatiar em vez de re.sub: o bloco tem barras invertidas e `\g` do Markdown
    # seriam lidos como referência de grupo pelo re.
    README.write_text(text[:achado.start()] + block + text[achado.end():],
                      encoding="utf-8")
    return True


def main() -> int:
    data = collect()
    if not data["per_lang"]:
        print("Nenhuma linguagem coletada — abortando sem alterar arquivos.", file=sys.stderr)
        return 1

    SVG_OUT.parent.mkdir(parents=True, exist_ok=True)
    svg = build_svg(data)
    svg_changed = (not SVG_OUT.exists()) or not mesmo_conteudo(
        svg, SVG_OUT.read_text(encoding="utf-8"))
    if svg_changed:
        SVG_OUT.write_text(svg, encoding="utf-8")

    top_langs_svg = build_profile_top_langs_svg(data)
    top_langs_changed = (not PROFILE_TOP_LANGS_OUT.exists()) or not mesmo_conteudo(
        top_langs_svg, PROFILE_TOP_LANGS_OUT.read_text(encoding="utf-8"))
    if top_langs_changed:
        PROFILE_TOP_LANGS_OUT.write_text(top_langs_svg, encoding="utf-8")

    md_changed = patch_readme(build_markdown(data))

    print(f"linguagens={len(data['per_lang'])} repos={data['repo_count']} "
          f"bytes={data['total_bytes']} svg_changed={svg_changed} "
          f"top_langs_changed={top_langs_changed} md_changed={md_changed}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
