#!/usr/bin/env python3
"""
Bot de análise de linguagens — ⬡ Projeto Baluarte / perfil.

Varre TODOS os repositórios do perfil, soma os bytes por linguagem
(via API /repos/{owner}/{repo}/languages) e gera:

  1. assets/lang-stats.svg  — card visual no tema "Ouro de Fábula"
  2. bloco no README.md entre os marcadores LANG-STATS — total de linguagens,
     quanto cada uma pesa e EM QUAIS repositórios foi usada.

Uso:
    GITHUB_TOKEN=... python3 .github/scripts/lang_stats.py

Variáveis de ambiente:
    GITHUB_TOKEN   token de leitura (o GITHUB_TOKEN do Actions basta p/ repos públicos)
    GH_USER        login do dono (padrão: Lucas-Belucci-Bellini)
    INCLUDE_FORKS  "1" para incluir forks (padrão: 0)
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

def collect() -> dict:
    repos = list_repos()
    per_lang: dict[str, int] = {}
    per_lang_repos: dict[str, list[tuple[str, int]]] = {}
    repo_rows: list[dict] = []
    scanned = 0

    for repo in repos:
        if repo.get("fork") and not INCLUDE_FORKS:
            continue
        owner = repo["owner"]["login"]
        name = repo["name"]
        langs = api(f"/repos/{owner}/{name}/languages") or {}
        if not langs:
            continue
        scanned += 1
        total = sum(langs.values())
        for lang, size in langs.items():
            per_lang[lang] = per_lang.get(lang, 0) + size
            per_lang_repos.setdefault(lang, []).append((name, size))
        top = max(langs.items(), key=lambda kv: kv[1])[0] if langs else "—"
        repo_rows.append({
            "name": name, "url": repo["html_url"], "bytes": total,
            "top": top, "private": repo.get("private", False),
            "langs": len(langs),
        })

    for lang in per_lang_repos:
        per_lang_repos[lang].sort(key=lambda t: t[1], reverse=True)
    repo_rows.sort(key=lambda r: r["bytes"], reverse=True)

    return {
        "per_lang": dict(sorted(per_lang.items(), key=lambda kv: kv[1], reverse=True)),
        "per_lang_repos": per_lang_repos,
        "repos": repo_rows,
        "repo_count": scanned,
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

    # stats
    stats = [
        (str(n_langs), "LINGUAGENS"),
        (str(data["repo_count"]), "REPOSITÓRIOS"),
        (human(data["total_bytes"]), "CÓDIGO TOTAL"),
    ]
    sx = 38
    for val, label in stats:
        parts.append(
            f'<text x="{sx}" y="93" fill="{GOLD}" font-size="22" font-weight="700">{esc(val)}</text>'
            f'<text x="{sx}" y="110" fill="{DIM}" font-size="10" letter-spacing="1.5">{label}</text>'
        )
        sx += 250

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

def build_markdown(data: dict) -> str:
    per_lang = data["per_lang"]
    total = data["total_bytes"] or 1
    stamp = data["generated"].strftime("%d/%m/%Y %H:%M UTC")

    out = [
        START,
        "",
        f"> **{len(per_lang)} linguagens** detectadas em **{data['repo_count']} repositórios** ·"
        f" **{human(data['total_bytes'])}** de código · atualizado em `{stamp}`",
        "",
        '<div align="center">',
        "",
        "![Análise de linguagens](./assets/lang-stats.svg)",
        "",
        "</div>",
        "",
        "| # | Linguagem | Peso | % | Repositórios |",
        "| :-- | :--- | ---: | ---: | ---: |",
    ]

    for i, (lang, size) in enumerate(per_lang.items(), 1):
        pct = size / total * 100
        n_repos = len(data["per_lang_repos"].get(lang, []))
        out.append(f"| {i} | **{lang}** | `{human(size)}` | `{pct:.2f}%` | {n_repos} |")

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
        for name, rsize in repos[:12]:
            url = f"https://github.com/{USER}/{name}"
            out.append(f"| [{name}]({url}) | `{human(rsize)}` |")
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
