#!/usr/bin/env python3
"""Refresh the profile README from GitHub metadata without reading private repository files.

The generator uses repository metadata and language byte maps only. Private repositories may
appear by name, metadata description, visibility, status, and link; their file contents are
never fetched or published.
"""
from __future__ import annotations

import argparse
import html
import json
import os
import re
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import Request, urlopen

OWNER = "Lucas-Belucci-Bellini"
ROOT = Path(__file__).resolve().parents[1]
README = ROOT / "README.md"
SITES_FILE = ROOT / "docs" / "README_SITES.json"
FEATURED_FILE = ROOT / "docs" / "README_FEATURED.json"
STACK_FILE = ROOT / "docs" / "README_STACK.json"
EXCLUDED_FILE = ROOT / "docs" / "README_EXCLUDED.json"
SNAPSHOT_SVG = ROOT / "assets" / "profile-snapshot.svg"

LANGUAGE_DISPLAY = {
    "Batchfile": "Batch",
    "Dockerfile": "Dockerfile",
    "PLpgSQL": "PL/pgSQL",
}

FEATURED_PRIORITY = {
    "Projeto-Baluarte": 120,
    "Veritas": 115,
    "Ark-Initiative": 105,
    "AEGIS": 100,
    "baluarte-obra-segura": 100,
    "Project-Vanguard": 95,
    "Digital-Logic-Sim-CE": 90,
    "CHIPS-Digital-Logic-Sim-Lucas-Belucci": 88,
    "taxforge": 86,
    "DailyPlanner": 82,
    "Projeto-Baluarte-World-Game": 80,
    "Recycle-game": 78,
}

FEATURED_SUMMARIES = {
    "Projeto-Baluarte": "Plataforma narrativa, tática e técnica; o site público expõe o núcleo online, J.A.R.V.I.S., Git Nexus e módulos de conteúdo.",
    "Veritas": "Calculadora de tabelas verdade e ferramenta local-first para projetar circuitos lógicos, com editor visual, simulação e MCP documentados.",
    "Ark-Initiative": "Conceito ARCA de infraestrutura de resiliência climática e ambiental, com visão pública de dados, simulação e resposta.",
    "baluarte-obra-segura": "Hub de engenharia para gestão de obras, editor de painéis elétricos, calculadoras e base WikiBuild, conforme a descrição pública.",
    "Project-Vanguard": "GPS topográfico tático e computador de tiro em JavaScript/Vite/MapLibre GL, conforme o README público.",
    "Digital-Logic-Sim-CE": "Fork público da Community Edition de Digital Logic Sim, com recursos de simulação de lógica digital documentados no README.",
    "CHIPS-Digital-Logic-Sim-Lucas-Belucci": "Coleção pública de chips e testes de lógica digital.",
    "DailyPlanner": "Agenda diária em TypeScript/Vite para cadastrar, editar, concluir, excluir, buscar e filtrar atividades no navegador.",
    "Projeto-Baluarte-World-Game": "Conceito e protótipo de jogo de sobrevivência, construção e consequência situado no universo Baluarte.",
    "Recycle-game": "Jogo educativo de reciclagem e automação com protótipo jogável documentado.",
}


def api_get(url: str, token: str | None) -> Any:
    headers = {"Accept": "application/vnd.github+json", "User-Agent": "profile-readme-refresh"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = Request(url, headers=headers)
    with urlopen(req, timeout=30) as response:
        return json.loads(response.read().decode("utf-8"))


def fetch_repositories(token: str | None) -> list[dict[str, Any]]:
    repos: list[dict[str, Any]] = []
    page = 1
    while True:
        if token:
            url = (
                "https://api.github.com/user/repos?"
                "affiliation=owner,collaborator,organization_member&per_page=100&page="
                + str(page)
            )
        else:
            # Unauthenticated fallback: only public repositories owned by this profile.
            url = f"https://api.github.com/users/{OWNER}/repos?type=owner&per_page=100&page={page}"
        batch = api_get(url, token)
        if not batch:
            break
        repos.extend(batch)
        if len(batch) < 100:
            break
        page += 1
    return repos


def fetch_languages(full_name: str, token: str | None) -> dict[str, int]:
    try:
        data = api_get(f"https://api.github.com/repos/{full_name}/languages", token)
        return {str(key): int(value) for key, value in data.items()}
    except (HTTPError, URLError, TimeoutError, ValueError):
        return {}


def check_url(url: str) -> tuple[bool, int, str]:
    if not url or not re.match(r"^https?://", url):
        return False, 0, ""
    request = Request(url, headers={"User-Agent": "profile-readme-link-check/1.0"}, method="GET")
    try:
        with urlopen(request, timeout=20) as response:
            return 200 <= response.status < 400, response.status, response.geturl()
    except HTTPError as error:
        # Some deployments reject HEAD/GET but still expose a meaningful redirect target.
        return False, int(error.code), getattr(error, "url", url)
    except (URLError, TimeoutError, ValueError):
        return False, 0, url


def load_local_repositories(path: Path) -> list[dict[str, Any]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, list):
        raise ValueError("repository input must be a JSON array")
    return data


def load_local_languages(directory: Path, repos: list[dict[str, Any]]) -> dict[str, dict[str, int]]:
    result: dict[str, dict[str, int]] = {}
    for repo in repos:
        full_name = str(repo["full_name"])
        safe = full_name.replace("/", "__")
        path = directory / f"{safe}.json"
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
            result[full_name] = {str(k): int(v) for k, v in data.items()}
        except (OSError, ValueError, TypeError):
            result[full_name] = {}
    return result


def normalize_name(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", " ", value.lower()).strip()


def classify(repo: dict[str, Any]) -> str:
    name = str(repo.get("name", ""))
    text = f"{name} {repo.get('description') or ''}".lower()
    n = normalize_name(name)
    if any(token in text for token in ("veritas", "digital logic", "chips", "umbra lima")):
        return "Digital Logic / Hardware"
    if any(token in text for token in ("baluarte", "llbr", "vanguard")):
        return "Ecossistema Baluarte"
    if any(token in text for token in ("academic", "atividade", "decision", "flowgorithm", "java", "python", "pseudocode", "teste aula")):
        return "Academia"
    if any(token in text for token in ("game", "games", "g-mod", "black mesa", "fallout", "mod-pack", "catacombs", "ossuary", "recycle")):
        return "Games"
    if any(token in text for token in ("ai", "artificial", "jarvis", "claude", "kizeo")):
        return "IA & Automação"
    if any(token in text for token in ("sujok", "banco de dados", "backend", "local de trabalho", "backup")):
        return "Infraestrutura / Backend / Dados"
    if repo.get("homepage") or any(token in text for token in ("portfolio", "site", "furniture", "construction", "invitation")):
        return "Web"
    if repo.get("fork"):
        return "Experimentos"
    if not repo.get("description") and not name:
        return "Experimentos"
    return "Software & Ferramentas"


def status_for(repo: dict[str, Any], category: str, now: datetime) -> str:
    name = str(repo.get("name", ""))
    if repo.get("private"):
        return "🔒 Private"
    if repo.get("archived"):
        return "⚪ Archived"
    if category == "Academia":
        return "🟣 Academic"
    if name in {"Projeto-Baluarte", "Ark-Initiative", "CHIPS-Digital-Logic-Sim-Lucas-Belucci"}:
        return "🟡 In Development"
    # The public baluarte-* domain READMEs explicitly mark the extracted domains as backlog.
    if name.startswith("baluarte-") and name != "baluarte-obra-segura":
        return "🟡 In Development"
    if repo.get("fork") and not repo.get("pushed_at"):
        return "🔵 Experimental"
    pushed = repo.get("pushed_at") or repo.get("updated_at")
    try:
        timestamp = datetime.fromisoformat(str(pushed).replace("Z", "+00:00"))
        age_days = max(0, (now - timestamp).days)
    except (TypeError, ValueError):
        age_days = 9999
    if age_days <= 60:
        return "🟢 Active"
    if age_days <= 365:
        return "🟡 In Development"
    return "🔵 Experimental"


def language_rows(repos: list[dict[str, Any]], languages: dict[str, dict[str, int]], public_only: bool = True) -> list[dict[str, Any]]:
    totals: dict[str, int] = {}
    repo_counts: dict[str, int] = {}
    for repo in repos:
        if public_only and repo.get("private"):
            continue
        full_name = str(repo["full_name"])
        for language, byte_count in languages.get(full_name, {}).items():
            totals[language] = totals.get(language, 0) + int(byte_count)
            repo_counts[language] = repo_counts.get(language, 0) + 1
    total_bytes = sum(totals.values()) or 1
    rows = []
    for language, byte_count in sorted(totals.items(), key=lambda item: (-item[1], item[0].lower())):
        rows.append({
            "language": language,
            "display": LANGUAGE_DISPLAY.get(language, language),
            "bytes": byte_count,
            "repositories": repo_counts[language],
            "share": byte_count / total_bytes * 100,
        })
    return rows


def format_bytes(value: int) -> str:
    if value < 1024:
        return f"{value} B"
    if value < 1024 * 1024:
        return f"{value / 1024:.1f} KB"
    return f"{value / (1024 * 1024):.2f} MB"


def featured_score(repo: dict[str, Any], now: datetime) -> float:
    score = float(FEATURED_PRIORITY.get(str(repo.get("name", "")), 0))
    score += min(float(repo.get("size") or 0) / 1000.0, 30.0)
    score += 8.0 if repo.get("description") else 0.0
    score += 8.0 if repo.get("homepage") else 0.0
    score += 5.0 if not repo.get("fork") else 0.0
    pushed = repo.get("pushed_at") or repo.get("updated_at")
    try:
        timestamp = datetime.fromisoformat(str(pushed).replace("Z", "+00:00"))
        age_days = max(0, (now - timestamp).days)
        score += 20.0 if age_days <= 90 else 10.0 if age_days <= 365 else 0.0
    except (TypeError, ValueError):
        pass
    return score


def site_for(repo: dict[str, Any], verified_sites: dict[str, dict[str, Any]]) -> str:
    full_name = str(repo["full_name"])
    site = verified_sites.get(full_name)
    if not site:
        return "—"
    url = site.get("url") or repo.get("homepage")
    return f"[Site / Demo]({url})" if url else "—"


def repo_link(repo: dict[str, Any]) -> str:
    return f"[GitHub](https://github.com/{repo['full_name']})"


def stack_for(repo: dict[str, Any], languages: dict[str, dict[str, int]]) -> str:
    items = [LANGUAGE_DISPLAY.get(k, k) for k in languages.get(str(repo["full_name"]), {}).keys()]
    if not items:
        return "—"
    return " ".join(f"`{item}`" for item in items[:5])


def replace_block(text: str, marker: str, body: str) -> str:
    pattern = re.compile(rf"<!-- {re.escape(marker)}:START -->.*?<!-- {re.escape(marker)}:END -->", re.S)
    replacement = f"<!-- {marker}:START -->\n{body.rstrip()}\n<!-- {marker}:END -->"
    if not pattern.search(text):
        raise ValueError(f"README marker not found: {marker}")
    return pattern.sub(replacement, text, count=1)


def render_dashboard(repos: list[dict[str, Any]], languages: list[dict[str, Any]], verified_sites: dict[str, dict[str, Any]], now: datetime) -> str:
    categorized = [(repo, classify(repo)) for repo in repos]
    active = sum(status_for(repo, category, now) == "🟢 Active" for repo, category in categorized)
    public = sum(not repo.get("private") for repo in repos)
    private = sum(bool(repo.get("private")) for repo in repos)
    academic = sum(category == "Academia" for _, category in categorized)
    excluded = len(load_excluded_names())
    return "\n".join([
        "> `GITHUB SNAPSHOT // FIELD REPORT` · inventário autenticado, métricas públicas e governança editorial.",
        "",
        "![GitHub snapshot](./assets/profile-snapshot.svg)",
        "",
        "<div align=\"center\">",
        "",
        "| REPOSITÓRIOS | PÚBLICOS | PRIVADOS VISÍVEIS | DEPLOYMENTS |",
        "|:---:|:---:|:---:|:---:|",
        f"| **{len(repos)}** | **{public}** | **{private}** | **{len(verified_sites)}** |",
        "",
        "| ATIVOS | ACADÊMICOS | LINGUAGENS | EXCLUSÕES EDITORIAIS |",
        "|:---:|:---:|:---:|:---:|",
        f"| **{active}** | **{academic}** | **{len(languages)}** | **{excluded}** |",
        "",
        "</div>",
        "",
        "> `STATUS: ONLINE` · `PRIVACY: SAFE` · contagens geradas pelo inventário autenticado do GitHub; nenhum conteúdo de arquivo privado é publicado.",
    ])


def render_language_badges(rows: list[dict[str, Any]]) -> str:
    shields = []
    for row in rows:
        language = str(row["display"])
        badge_label = quote(language, safe="")
        badge_message = quote(f"{row['repositories']} repos", safe="")
        color = "3178C6" if language == "TypeScript" else "F2C94C" if language == "JavaScript" else "555555"
        shields.append(
            f"[![{badge_label}](https://img.shields.io/badge?label={badge_label}&message={badge_message}&color={color}&style=flat-square)](https://github.com/{OWNER}?tab=repositories&q=&language={quote(language, safe='')})"
        )
    return "\n".join([
        "> **17 linguagens em uso** · badges gerados a partir dos repositórios públicos auditados.",
        "> A lista abaixo mostra a amplitude do portfólio; a tabela de análise informa peso em bytes e participação relativa.",
        "",
        " ".join(shields[:9]),
        "",
        " ".join(shields[9:]),
    ])


def render_language_stats(repos: list[dict[str, Any]], rows: list[dict[str, Any]], generated_at: str) -> str:
    total = sum(row["bytes"] for row in rows)
    lines = [
        f"> **{len(rows)} linguagens** · **{sum(not repo.get('private') for repo in repos)} repositórios públicos** · **{format_bytes(total)} de código detectado** · atualizado em `{generated_at}`",
        "",
        "| # | Linguagem | Peso | Participação | Repositórios |",
        "|:--:|:---|---:|---:|---:|",
    ]
    for index, row in enumerate(rows, 1):
        lines.append(f"| {index} | **{row['display']}** | `{format_bytes(row['bytes'])}` | `{row['share']:.2f}%` | {row['repositories']} |")
    lines.extend([
        "",
        "> A tabela acima considera somente repositórios públicos. Repositórios privados podem contribuir para métricas agregadas futuras, mas seus arquivos, nomes de arquivos e estrutura interna não são publicados.",
        "",
        "![Public language distribution](./assets/lang-stats.svg)",
    ])
    return "\n".join(lines)


def render_curated_featured(repos: list[dict[str, Any]], verified_sites: dict[str, dict[str, Any]], manifest: dict[str, Any], now: datetime) -> str:
    by_name = {str(repo.get("name")): repo for repo in repos}
    lines = [
        f"> {manifest.get('intro', 'Seleção editorial de projetos públicos.')}",
        "",
        "| # | Missão | Foco confirmado | Status | Acesso |",
        "|:--:|:---|:---|:---|:---|",
    ]
    entries = sorted(manifest.get("projects", []), key=lambda item: int(item.get("order", 999)))
    index = 0
    for entry in entries:
        name = str(entry.get("name", ""))
        repo = by_name.get(name)
        if not repo or repo.get("private"):
            continue
        index += 1
        label = str(entry.get("label", "MISSÃO")).replace("|", "\\|")
        focus = str(entry.get("focus", repo.get("description") or "Descrição pública não informada.")).replace("|", "\\|").replace("\n", " ")
        category = classify(repo)
        access = repo_link(repo)
        site = verified_sites.get(str(repo["full_name"]))
        if site:
            access += f" · [Site]({site.get('url')})"
        lines.append(f"| {index} | **{label}** · {name} | {focus} | {status_for(repo, category, now)} | {access} |")
    if index == 0:
        lines.append("| — | Nenhuma missão pública encontrada | O manifesto será revisado no próximo refresh. | — | — |")
    return "\n".join(lines)


def render_arsenal_stack(rows: list[dict[str, Any]], manifest: dict[str, Any]) -> str:
    lines = [
        f"> **{len(rows)} linguagens detectadas** no inventário público. O peso e a quantidade de repositórios são calculados automaticamente pelo GitHub.",
        "",
        "| Linguagem | Repositórios | Participação |",
        "|:---|---:|---:|",
    ]
    for row in rows:
        lines.append(f"| **{row['display']}** | {row['repositories']} | `{row['share']:.2f}%` |")
    lines.extend([
        "",
        "### Ferramentas, plataformas e ambientes",
        "",
        "| Ferramenta | Papel | Evidência pública |",
        "|:---|:---|:---|",
    ])
    for tool in manifest.get("tools", []):
        name = str(tool.get("name", "")).replace("|", "\\|")
        family = str(tool.get("family", "")).replace("|", "\\|")
        evidence = str(tool.get("evidence", "")).replace("|", "\\|")
        lines.append(f"| **{name}** | {family} | {evidence} |")
    lines.append("")
    lines.append("> A lista de ferramentas é editorial e baseada em READMEs, manifests, configurações e seções visuais preservadas; ela não substitui o mapa automático de linguagens.")
    return "\n".join(lines)


def render_featured_projects(repos: list[dict[str, Any]], verified_sites: dict[str, dict[str, Any]], now: datetime) -> str:
    selected = sorted(repos, key=lambda repo: featured_score(repo, now), reverse=True)[:10]
    lines = [
        "| Projeto | O que a evidência pública confirma | Status | Acesso |",
        "|:---|:---|:---|:---|",
    ]
    for repo in selected:
        name = str(repo["name"])
        category = classify(repo)
        if repo.get("private"):
            summary = "Repositório privado identificado no inventário autenticado. Nenhum detalhe interno é publicado."
        else:
            summary = FEATURED_SUMMARIES.get(name) or str(repo.get("description") or "Descrição pública não informada.")
        access = repo_link(repo)
        if str(repo["full_name"]) in verified_sites:
            access += f" · [Site]({verified_sites[str(repo['full_name'])]['url']})"
        lines.append(f"| **{name}** | {summary} | {status_for(repo, category, now)} | {access} |")
    return "\n".join(lines)


def render_public_projects(repos: list[dict[str, Any]], now: datetime) -> str:
    lines = [
        "<details>",
        "<summary><b>🌐 Public repository catalog</b></summary>",
        "",
        "| Projeto | Categoria | Status | GitHub |",
        "|:---|:---|:---|:---|",
    ]
    for repo in sorted((r for r in repos if not r.get("private")), key=lambda r: str(r["name"]).lower()):
        category = classify(repo)
        lines.append(f"| **{repo['name']}** | {category} | {status_for(repo, category, now)} | {repo_link(repo)} |")
    lines.extend(["", "</details>"])
    return "\n".join(lines)


def render_private_projects(repos: list[dict[str, Any]], now: datetime) -> str:
    lines = [
        "<details>",
        "<summary><b>🔒 Private repository catalog</b></summary>",
        "",
        "| Projeto | Categoria | Descrição pública | Status | GitHub |",
        "|:---|:---|:---|:---|:---|",
    ]
    for repo in sorted((r for r in repos if r.get("private")), key=lambda r: str(r["name"]).lower()):
        category = classify(repo)
        description = str(repo.get("description") or "Descrição pública não informada").replace("|", "\\|").replace("\n", " ")
        lines.append(f"| **{repo['name']}** | {category} | {description} | {status_for(repo, category, now)} | {repo_link(repo)} |")
    lines.extend(["", "</details>", "", "> 🔒 Private repository · nenhuma linha desta seção expõe código, secrets, `.env`, tokens, credenciais ou estrutura interna."])
    return "\n".join(lines)


def render_live_projects(repos: list[dict[str, Any]], verified_sites: dict[str, dict[str, Any]]) -> str:
    lines = [
        "| Projeto | GitHub | Website | Status |",
        "|:---|:---|:---|:---|",
    ]
    for repo in sorted(repos, key=lambda r: str(r["name"]).lower()):
        site = verified_sites.get(str(repo["full_name"]))
        if not site:
            continue
        url = site.get("url") or repo.get("homepage")
        lines.append(f"| **{repo['name']}** | {repo_link(repo)} | [Site / Demo]({url}) | {site.get('status', 'reachable')} |")
    if len(lines) == 3:
        lines.append("| — | — | Site não verificado | — |")
    return "\n".join(lines)


def render_project_map(repos: list[dict[str, Any]], languages: dict[str, dict[str, int]], verified_sites: dict[str, dict[str, Any]], now: datetime) -> str:
    lines = [
        "<details>",
        "<summary><b>⌁ Complete project map</b></summary>",
        "",
        "| Projeto | Categoria | Stack | Status | GitHub | Site |",
        "|:---|:---|:---|:---|:---|:---|",
    ]
    for repo in sorted(repos, key=lambda r: str(r["name"]).lower()):
        category = classify(repo)
        lines.append(
            f"| **{repo['name']}** | {category} | {stack_for(repo, languages)} | {status_for(repo, category, now)} | {repo_link(repo)} | {site_for(repo, verified_sites)} |"
        )
    lines.extend(["", "</details"])
    # Correct the closing tag after keeping the table construction visually simple above.
    lines[-1] = "</details>"
    return "\n".join(lines)


def render_snapshot_svg(repos: list[dict[str, Any]], languages: list[dict[str, Any]], verified_sites: dict[str, dict[str, Any]], now: datetime, generated_at: str, destination: Path) -> None:
    categorized = [(repo, classify(repo)) for repo in repos]
    values = [
        ("REPOSITORIES", str(len(repos)), "inventory"),
        ("PUBLIC", str(sum(not repo.get("private") for repo in repos)), "visible"),
        ("PRIVATE", str(sum(bool(repo.get("private")) for repo in repos)), "metadata"),
        ("DEPLOYMENTS", str(len(verified_sites)), "HTTP 200"),
        ("ACTIVE", str(sum(status_for(repo, category, now) == "🟢 Active" for repo, category in categorized)), "status"),
        ("ACADEMIC", str(sum(category == "Academia" for _, category in categorized)), "portfolio"),
        ("LANGUAGES", str(len(languages)), "public"),
        ("EXCLUDED", str(len(load_excluded_names())), "editorial"),
    ]
    width, height = 1100, 300
    background, surface, border = "#0e0c16", "#1d1729", "#4b3a5c"
    light, muted, gold, green = "#f4ecdd", "#a89f91", "#d4a24e", "#3ddc84"
    svg = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        f'<rect width="{width}" height="{height}" rx="18" fill="{background}"/>',
        f'<rect x="18" y="18" width="{width - 36}" height="{height - 36}" rx="14" fill="none" stroke="{border}" stroke-width="2"/>',
        f'<text x="48" y="58" fill="{gold}" font-family="sans-serif" font-size="22" font-weight="700">&gt;&gt; GITHUB SNAPSHOT // FIELD REPORT &lt;&lt;</text>',
        f'<text x="48" y="84" fill="{muted}" font-family="sans-serif" font-size="13">authenticated inventory | public-safe metrics | generated {html.escape(generated_at)}</text>',
    ]
    card_w, card_h, gap = 245, 78, 16
    start_x, start_y = 48, 105
    for index, (label, value, note) in enumerate(values):
        row, col = divmod(index, 4)
        x, y = start_x + col * (card_w + gap), start_y + row * (card_h + gap)
        value_color = green if label in {"DEPLOYMENTS", "ACTIVE"} else gold
        svg.extend([
            f'<rect x="{x}" y="{y}" width="{card_w}" height="{card_h}" rx="8" fill="{surface}" stroke="{border}"/>',
            f'<text x="{x + 16}" y="{y + 29}" fill="{value_color}" font-family="sans-serif" font-size="25" font-weight="700">{html.escape(value)}</text>',
            f'<text x="{x + 16}" y="{y + 50}" fill="{light}" font-family="sans-serif" font-size="12" font-weight="700">{html.escape(label)}</text>',
            f'<text x="{x + 16}" y="{y + 66}" fill="{muted}" font-family="sans-serif" font-size="10">{html.escape(note)}</text>',
        ])
    svg.append("</svg>")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text("\n".join(svg) + "\n", encoding="utf-8")


def render_svg(rows: list[dict[str, Any]], destination: Path) -> None:
    width, height = 1100, 620
    background = "#0e0c16"
    gold = "#d4a24e"
    light = "#f4ecdd"
    muted = "#a89f91"
    green = "#3ddc84"
    max_bytes = max((row["bytes"] for row in rows), default=1)
    visible = rows[:12]
    svg: list[str] = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        f'<rect width="{width}" height="{height}" rx="18" fill="{background}"/>',
        f'<text x="48" y="58" fill="{light}" font-family="monospace" font-size="25" font-weight="700">LANGUAGE MATRIX // TOP LANGUAGES</text>',
        f'<text x="48" y="88" fill="{muted}" font-family="monospace" font-size="15">source: GitHub language bytes | public repositories only</text>',
    ]
    bar_x, bar_w, start_y, row_h = 270, 680, 125, 36
    for index, row in enumerate(visible):
        y = start_y + index * row_h
        label = html.escape(row["display"])
        bar = max(3, int(bar_w * row["bytes"] / max_bytes))
        fill = green if index == 0 else gold
        svg.append(f'<text x="48" y="{y + 19}" fill="{light}" font-family="monospace" font-size="16">{index + 1:>2} {label}</text>')
        svg.append(f'<rect x="{bar_x}" y="{y + 4}" width="{bar_w}" height="20" rx="10" fill="#1d1729"/>')
        svg.append(f'<rect x="{bar_x}" y="{y + 4}" width="{bar}" height="20" rx="10" fill="{fill}"/>')
        svg.append(f'<text x="{bar_x + bar_w + 16}" y="{y + 19}" fill="{light}" font-family="monospace" font-size="14">{row["share"]:.1f}%</text>')
    svg.extend([
        f'<text x="48" y="{height - 36}" fill="{muted}" font-family="monospace" font-size="14">Generated by scripts/update_profile.py · no private file contents published</text>',
        "</svg>",
    ])
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text("\n".join(svg) + "\n", encoding="utf-8")


def load_json_object(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise ValueError(f"invalid JSON manifest: {path}: {error}") from error
    if not isinstance(data, dict):
        raise ValueError(f"JSON manifest must be an object: {path}")
    return data


def load_excluded_names() -> set[str]:
    data = load_json_object(EXCLUDED_FILE)
    values = data.get("repositories", [])
    if not isinstance(values, list):
        raise ValueError(f"repositories must be a list: {EXCLUDED_FILE}")
    return {str(value) for value in values}


def load_site_overrides() -> dict[str, str]:
    if not SITES_FILE.exists():
        return {}
    try:
        data = json.loads(SITES_FILE.read_text(encoding="utf-8"))
        return {str(k): str(v) for k, v in data.items()}
    except (OSError, ValueError, TypeError):
        return {}


def build_data(args: argparse.Namespace) -> tuple[list[dict[str, Any]], dict[str, dict[str, int]]]:
    # PROFILE_GITHUB_TOKEN is optional for read-only/public previews, but write mode
    # must not replace a private-aware README with a public-only inventory.
    inventory_token = args.github_token or os.environ.get("PROFILE_GITHUB_TOKEN")
    api_token = inventory_token or os.environ.get("GITHUB_TOKEN")
    if args.write and not args.input_repos and not inventory_token:
        raise ValueError("refusing write mode without PROFILE_GITHUB_TOKEN; this could erase private-project entries")
    excluded_names = load_excluded_names()
    if args.input_repos:
        repos = load_local_repositories(Path(args.input_repos))
    else:
        repos = fetch_repositories(inventory_token)
    repos = [
        repo for repo in repos
        if str(repo.get("name", "")) not in excluded_names
        and str(repo.get("full_name", "")) not in excluded_names
    ]
    if args.input_repos:
        languages = load_local_languages(Path(args.languages_dir), repos) if args.languages_dir else {str(r["full_name"]): {} for r in repos}
    else:
        languages = {str(repo["full_name"]): fetch_languages(str(repo["full_name"]), api_token) for repo in repos}
    return repos, languages


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-repos", help="local repos.json from an authenticated audit")
    parser.add_argument("--languages-dir", help="local directory containing one JSON language map per repository")
    parser.add_argument("--github-token", help="token for GitHub API access; prefer environment variables in CI")
    parser.add_argument("--write", action="store_true", help="write README and generated assets; otherwise validate/render only")
    args = parser.parse_args()

    try:
        repos, languages = build_data(args)
    except (HTTPError, URLError, OSError, ValueError) as error:
        print(f"profile refresh failed: {error}", file=sys.stderr)
        return 2
    if not repos:
        print("profile refresh failed: GitHub returned no repositories", file=sys.stderr)
        return 2

    now = datetime.now(timezone.utc)
    verified_sites: dict[str, dict[str, Any]] = {}
    manual_overrides = load_site_overrides()
    featured_manifest = load_json_object(FEATURED_FILE)
    stack_manifest = load_json_object(STACK_FILE)
    for repo in repos:
        # Private repositories are never included in the public live-project catalog.
        if repo.get("private"):
            continue
        homepage = str(repo.get("homepage") or "").strip()
        full_name = str(repo["full_name"])
        if not homepage and full_name in manual_overrides:
            homepage = manual_overrides[full_name]
        if homepage:
            reachable, status, effective = check_url(homepage)
            if reachable:
                verified_sites[full_name] = {"url": effective or homepage, "status": f"HTTP {status}"}

    rows = language_rows(repos, languages, public_only=True)
    # Use a source-derived timestamp so unchanged inventories do not create timestamp-only commits.
    source_times = []
    for repo in repos:
        value = repo.get("pushed_at") or repo.get("updated_at")
        try:
            source_times.append(datetime.fromisoformat(str(value).replace("Z", "+00:00")))
        except (TypeError, ValueError):
            continue
    generated_at = max(source_times, default=now).strftime("%Y-%m-%d %H:%M UTC")
    text = README.read_text(encoding="utf-8")
    text = replace_block(text, "PROFILE-DASHBOARD", render_dashboard(repos, rows, verified_sites, now))
    text = replace_block(text, "FEATURED-PROJECTS", render_featured_projects(repos, verified_sites, now))
    text = replace_block(text, "CURATED-FEATURED", render_curated_featured(repos, verified_sites, featured_manifest, now))
    text = replace_block(text, "ARSENAL-STACK", render_arsenal_stack(rows, stack_manifest))
    text = replace_block(text, "LANGUAGE-BADGES", render_language_badges(rows))
    text = replace_block(text, "LANGUAGE-STATS", render_language_stats(repos, rows, generated_at))
    text = replace_block(text, "PUBLIC-PROJECTS", render_public_projects(repos, now))
    text = replace_block(text, "PRIVATE-PROJECTS", render_private_projects(repos, now))
    text = replace_block(text, "LIVE-PROJECTS", render_live_projects(repos, verified_sites))
    text = replace_block(text, "PROJECT-MAP", render_project_map(repos, languages, verified_sites, now))

    if args.write:
        README.write_text(text, encoding="utf-8")
        render_svg(rows, ROOT / "assets" / "lang-stats.svg")
        render_snapshot_svg(repos, rows, verified_sites, now, generated_at, SNAPSHOT_SVG)
    else:
        print(text[:500])
    print(json.dumps({
        "repositories": len(repos),
        "public_repositories": sum(not repo.get("private") for repo in repos),
        "private_repositories": sum(bool(repo.get("private")) for repo in repos),
        "excluded_repositories": sorted(load_excluded_names()),
        "public_languages": len(rows),
        "verified_sites": len(verified_sites),
        "write": bool(args.write),
    }, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
