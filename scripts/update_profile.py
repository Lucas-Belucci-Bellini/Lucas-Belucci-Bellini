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

# Este script também é carregado por importlib nos testes, sem o diretório
# scripts/ no sys.path; o bootstrap abaixo faz o import funcionar nos dois casos.
sys.path.insert(0, str(Path(__file__).resolve().parent))

from project_catalog import (  # noqa: E402
    COLOR_GOLD,
    Presentation,
    _badge,
    build_catalog,
    check_websites,
    cta_buttons,
    cta_cell,
    status_pill,
    discover_project_website,
    normalize_site_overrides,
    resolve_presentation,
    write_catalog_if_changed,
)

OWNER = "Lucas-Belucci-Bellini"
ROOT = Path(__file__).resolve().parents[1]
README = ROOT / "README.md"
SITES_FILE = ROOT / "docs" / "README_SITES.json"
FEATURED_FILE = ROOT / "docs" / "README_FEATURED.json"
STACK_FILE = ROOT / "docs" / "README_STACK.json"
EXCLUDED_FILE = ROOT / "docs" / "README_EXCLUDED.json"
SNAPSHOT_SVG = ROOT / "assets" / "profile-snapshot.svg"
CATALOG_FILE = ROOT / "docs" / "project-catalog.json"

LANGUAGE_DISPLAY = {
    "Batchfile": "Batch",
    "Dockerfile": "Dockerfile",
    "PLpgSQL": "PL/pgSQL",
}

LANGUAGE_COLORS = {
    "JavaScript": "F7DF1E",
    "TypeScript": "3178C6",
    "HTML": "E34F26",
    "CSS": "1572B6",
    "Java": "ED8B00",
    "Python": "3776AB",
    "CSharp": "239120",
    "C#": "239120",
    "PLpgSQL": "336791",
    "PL/pgSQL": "336791",
    "Rust": "DEA584",
    "Shell": "4EAA25",
    "GDScript": "478CBF",
    "PowerShell": "5391FE",
    "Portugol": "6A5ACD",
    "Batchfile": "5C2D91",
    "Batch": "5C2D91",
    "ShaderLab": "A48EFA",
    "Dockerfile": "2496ED",
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


def describe(repo: dict[str, Any]) -> str:
    """Descrição pública do repositório, em uma linha e sem escape.

    O valor cru é o que vai para o catálogo JSON; escapar para Markdown é
    trabalho de `md_cell`, na hora de renderizar a célula.
    """
    text = str(repo.get("description") or "Descrição pública não informada.")
    return " ".join(text.split())


def md_cell(text: str) -> str:
    """Escapa o que quebraria uma célula de tabela Markdown."""
    return text.replace("|", "\\|")


def featured_order(curadoria: dict[str, dict[str, Any]], repo: dict[str, Any]) -> int | None:
    """Posição do repositório na curadoria, ou None quando não está nela."""
    entry = curadoria.get(str(repo.get("name")))
    return int(entry.get("order", 999)) if entry else None


def featured_priority(curadoria: dict[str, dict[str, Any]], repo: dict[str, Any]) -> int | None:
    """`priority` explícita do manifesto, quando o operador tiver declarado uma."""
    entry = curadoria.get(str(repo.get("name")))
    if not entry or entry.get("priority") is None:
        return None
    try:
        return int(entry["priority"])
    except (TypeError, ValueError):
        return None


def build_presentations(
    repos: list[dict[str, Any]],
    *,
    sites: dict[str, tuple[str | None, str]],
    checks: dict[str, Any],
    featured_manifest: dict[str, Any],
    now: datetime,
) -> dict[str, Presentation]:
    """Resolve como cada repositório aparece na vitrine.

    Toda decisão de CTA passa por `resolve_presentation`; este laço só reúne
    os insumos (site descoberto, verificação, categoria, status, curadoria).
    """
    curadoria = {
        str(entry.get("name", "")): entry
        for entry in featured_manifest.get("projects", [])
        if entry.get("name")
    }
    presentations: dict[str, Presentation] = {}
    for repo in repos:
        full_name = str(repo["full_name"])
        url, source = sites.get(full_name, (None, "none"))
        category = classify(repo)
        presentations[full_name] = resolve_presentation(
            repo,
            check=checks.get(url) if url else None,
            website=url,
            website_source=source,
            category=category,
            status=status_for(repo, category, now),
            description=describe(repo),
            featured_order=featured_order(curadoria, repo),
            featured_priority=featured_priority(curadoria, repo),
        )
    return presentations


def live_site_map(presentations: dict[str, Presentation]) -> dict[str, dict[str, Any]]:
    """Mapa compacto dos sites no ar, para os contadores e o SVG."""
    return {
        name: {"url": item.website, "status": f"HTTP {item.website_http_status}"}
        for name, item in presentations.items()
        if item.has_live_website
    }


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
    # Função, não string: o corpo vem de descrições do GitHub e pode conter `\`,
    # que como string de substituição o `re` leria como escape ou grupo.
    return pattern.sub(lambda _match: replacement, text, count=1)


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
        # Keep the accessible Markdown label human-readable; encode only the URL parts.
        language = str(row["display"])
        badge_label = quote(language, safe="")
        badge_message = quote(f"{row['repositories']} repos", safe="")
        color = LANGUAGE_COLORS.get(str(row["language"]), LANGUAGE_COLORS.get(language, "6E6E6E"))
        badge_url = (
            f"https://img.shields.io/badge/{badge_label}-{badge_message}-{color}"
            "?style=flat-square&labelColor=0e0c16"
        )
        search_language = quote(str(row["language"]), safe="")
        shields.append(
            f"[![{language}]({badge_url})]"
            f"(https://github.com/{OWNER}?tab=repositories&q=&language={search_language})"
        )

    split_at = (len(shields) + 1) // 2

    def badge_lines(items: list[str], per_line: int = 5) -> list[str]:
        return [" ".join(items[index:index + per_line]) for index in range(0, len(items), per_line)]

    lines = [
        f"> **{len(rows)} linguagens públicas detectadas** · badges gerados a partir dos repositórios auditados.",
        "> Os nomes permanecem legíveis mesmo quando a URL precisa escapar caracteres como `#` e `/`; a matriz abaixo informa peso em bytes e participação relativa.",
        "",
    ]
    if shields:
        lines.extend(["**PRINCIPAIS POR VOLUME**", ""])
        for index, line in enumerate(badge_lines(shields[:split_at])):
            if index:
                lines.append("<br>")
            lines.append(line)
    if shields[split_at:]:
        lines.extend(["", "", "**EXTENSÕES DO PORTFÓLIO**", ""])
        for index, line in enumerate(badge_lines(shields[split_at:])):
            if index:
                lines.append("<br>")
            lines.append(line)
    return "\n".join(lines)


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


def render_arsenal_stack(rows: list[dict[str, Any]], manifest: dict[str, Any]) -> str:
    principais = " · ".join(f"`{row['display']}`" for row in rows[:8])
    lines = [
        f"> **{len(rows)} linguagens detectadas** no inventário público, por peso em bytes: {principais}.",
        "",
        "<details>",
        "<summary><b>▶ Ver a participação de cada linguagem</b></summary>",
        "",
        "| Linguagem | Repositórios | Participação |",
        "|:---|---:|---:|",
    ]
    for row in rows:
        lines.append(f"| **{row['display']}** | {row['repositories']} | `{row['share']:.2f}%` |")
    lines.extend(["", "</details>"])

    category_order = [
        ("Frameworks & Web", "🧩"),
        ("Infraestrutura & DevOps", "🛡"),
        ("IA & Conhecimento", "🤖"),
        ("Hardware & Simulação", "⚙"),
    ]
    tools = manifest.get("tools", [])
    by_category: dict[str, list[dict[str, Any]]] = {}
    for tool in tools:
        category = str(tool.get("category", "Outras ferramentas"))
        by_category.setdefault(category, []).append(tool)

    for category, icon in category_order:
        category_tools = by_category.get(category, [])
        if not category_tools:
            continue
        lines.extend([
            "",
            f"### {icon} {category}",
            "",
            "| Ferramenta | Papel | Evidência pública |",
            "|:---|:---|:---|",
        ])
        for tool in category_tools:
            name = str(tool.get("name", "")).replace("|", "\\|")
            family = str(tool.get("family", "")).replace("|", "\\|")
            evidence = str(tool.get("evidence", "")).replace("|", "\\|")
            lines.append(f"| **{name}** | {family} | {evidence} |")

    remaining_categories = [category for category in by_category if category not in {item[0] for item in category_order}]
    for category in sorted(remaining_categories):
        lines.extend([
            "",
            f"### 🧰 {category}",
            "",
            "| Ferramenta | Papel | Evidência pública |",
            "|:---|:---|:---|",
        ])
        for tool in by_category[category]:
            name = str(tool.get("name", "")).replace("|", "\\|")
            family = str(tool.get("family", "")).replace("|", "\\|")
            evidence = str(tool.get("evidence", "")).replace("|", "\\|")
            lines.append(f"| **{name}** | {family} | {evidence} |")

    lines.extend([
        "",
        "> A separação é editorial: linguagens vêm dos mapas públicos do GitHub; ferramentas e categorias vêm de READMEs, manifests, configurações e evidências visuais públicas. Nenhum bloco publica conteúdo privado.",
    ])
    return "\n".join(lines)


def render_featured_projects(
    repos: list[dict[str, Any]],
    presentations: dict[str, Presentation],
    manifest: dict[str, Any],
    now: datetime,
    *,
    maximum: int = 10,
) -> str:
    """A tabela única de destaques: curadoria na frente, heurística completando.

    Antes existiam duas tabelas quase iguais — uma vinda do manifesto e outra do
    `featured_score`. Repetir os mesmos projetos com rótulos diferentes só custava
    rolagem, então viraram uma: quem está no manifesto entra primeiro, com o
    rótulo e o foco que o operador escreveu; o resto do espaço vai para os
    projetos públicos de maior prioridade que ainda não apareceram.
    """
    # The hero section is public-facing; keep private repository metadata out of it.
    by_name = {str(repo.get("name")): repo for repo in repos if not repo.get("private")}
    lines = [
        f"> {manifest.get('intro', 'Seleção editorial de projetos públicos.')}",
        "",
        "| # | Missão | O que a evidência pública confirma | Status | Acesso |",
        "|:--:|:---|:---|:---|:---|",
    ]

    escolhidos: list[tuple[str, str, Presentation]] = []
    vistos: set[str] = set()

    for entry in sorted(manifest.get("projects", []), key=lambda item: int(item.get("order", 999))):
        name = str(entry.get("name", ""))
        repo = by_name.get(name)
        if not repo:
            continue
        # Uma vez na curadoria, a decisão vale: se `website_required` tirou a
        # entrada, ela não pode voltar pela porta dos fundos da heurística.
        vistos.add(name)
        item = presentations[str(repo["full_name"])]
        if entry.get("website_required") and not item.has_live_website:
            continue
        rotulo = str(entry.get("label", "MISSÃO")).replace("|", "\\|")
        foco = str(entry.get("focus") or FEATURED_SUMMARIES.get(name) or describe(repo))
        escolhidos.append((rotulo, md_cell(" ".join(foco.split())), item))

    restantes = sorted(
        (repo for repo in by_name.values() if str(repo.get("name")) not in vistos),
        key=lambda repo: featured_score(repo, now),
        reverse=True,
    )
    for repo in restantes[: max(0, maximum - len(escolhidos))]:
        name = str(repo["name"])
        item = presentations[str(repo["full_name"])]
        resumo = FEATURED_SUMMARIES.get(name) or describe(repo)
        escolhidos.append((classify(repo).upper(), md_cell(resumo), item))

    if not escolhidos:
        lines.append(
            "| — | Nenhuma missão pública encontrada | O manifesto será revisado no próximo refresh. | — | — |"
        )
        return "\n".join(lines)

    for indice, (rotulo, foco, item) in enumerate(escolhidos, 1):
        lines.append(
            f"| {indice} | **{rotulo}** · {item.name} | {foco} | {item.status} | {cta_cell(item)} |"
        )
    return "\n".join(lines)


# Domínios da vitrine: o agrupamento largo que o visitante entende em dois
# segundos, montado sobre os rótulos editoriais que o README já usa.
DOMAINS = (
    ("🌐", "WEB & SAAS", "Plataformas, ferramentas e produtos web publicados.",
     ("Ecossistema Baluarte", "Web")),
    ("🤖", "AI & AUTOMATION", "Agentes, automação e sistemas de conhecimento.",
     ("IA & Automação",)),
    ("⚙", "HARDWARE & LOGIC", "Lógica digital, CPUs do zero e eletrônica.",
     ("Digital Logic / Hardware",)),
    ("🎮", "GAMES & WORLDS", "Jogos, simulações e mundos jogáveis.",
     ("Games",)),
    ("🛠", "TOOLS & SYSTEMS", "Utilitários, scripts e infraestrutura de apoio.",
     ("Software & Ferramentas", "Infraestrutura / Backend / Dados")),
    ("🎓", "ACADEMIC & LABS", "Trabalhos de curso, estudos dirigidos e experimentos.",
     ("Academia", "Experimentos")),
)

# Todo rótulo de `classify()` precisa cair em algum domínio. Uma categoria nova
# sem domínio sumiria da vitrine em silêncio — o teste cobre exatamente isso.
DOMAIN_LABELS = frozenset(rotulo for _, _, _, rotulos in DOMAINS for rotulo in rotulos)


def render_what_i_build(
    repos: list[dict[str, Any]],
    presentations: dict[str, Presentation],
    *,
    columns: int = 3,
) -> str:
    """Os quatro domínios de trabalho, com números que saem do inventário.

    A amplitude é o argumento desta seção, então ela mostra contagem real em vez
    de adjetivo. Um domínio sem nenhum projeto público some em vez de aparecer
    zerado — prometer menos é melhor do que exibir um vazio.
    """
    publicos = [
        presentations[str(repo["full_name"])]
        for repo in repos
        if not repo.get("private")
    ]
    presentes = []
    for icone, titulo, resumo, rotulos in DOMAINS:
        itens = [p for p in publicos if p.category_label in rotulos]
        if itens:
            presentes.append((icone, titulo, resumo, itens))
    if not presentes:
        return "> Nenhum projeto público para agrupar nesta auditoria."

    largura = f"{100 // columns}%"
    linhas = ["<table>"]
    for inicio in range(0, len(presentes), columns):
        fatia = presentes[inicio : inicio + columns]
        linhas.append("<tr>")
        for icone, titulo, resumo, itens in fatia:
            vivos = sum(1 for p in itens if p.has_live_website)
            destaque = max(itens, key=lambda p: (p.marketing_priority, p.name.lower()))
            plural = "projeto" if len(itens) == 1 else "projetos"
            linhas.extend([
                f'<td width="{largura}" valign="top" align="center">',
                "",
                f"### {icone}",
                "",
                f"**{titulo}**",
                "",
                f"<sub>{resumo}</sub>",
                "",
                f"`{len(itens)} {plural}` · `{vivos} com site`",
                "",
                f"<sub>ex.: {destaque.name}</sub>",
                "",
                "</td>",
            ])
        for _ in range(columns - len(fatia)):
            linhas.append(f'<td width="{largura}"></td>')
        linhas.append("</tr>")
    linhas.append("</table>")
    return "\n".join(linhas)


def render_website_directory(
    repos: list[dict[str, Any]],
    presentations: dict[str, Presentation],
) -> str:
    """Diretório enxuto: todos os sites no ar, um clique cada.

    É a seção que responde «o que dá para abrir agora» sem tabela e sem rolagem.
    Só entra o que passou pela verificação.
    """
    vivos = sorted(
        (
            presentations[str(repo["full_name"])]
            for repo in repos
            if presentations[str(repo["full_name"])].has_live_website
        ),
        key=lambda p: p.name.lower(),
    )
    if not vivos:
        return "> Nenhum site respondeu na última auditoria."
    linhas = [
        f"> **{len(vivos)} sites no ar** · cada link foi conferido por requisição HTTP nesta auditoria.",
        "",
        "<div align=\"center\">",
        "",
    ]
    linhas.extend(
        f"[![{item.name}]({_badge(f'🌐 {item.name.upper()}', COLOR_GOLD, style='flat-square')})]({item.website})"
        for item in vivos
    )
    linhas.extend(["", "</div>"])
    return "\n".join(linhas)


def card_summary(presentation: Presentation, limit: int = 150) -> str:
    """Descrição do card: curta, orientada a valor, sem cortar no meio da palavra."""
    text = FEATURED_SUMMARIES.get(presentation.name) or presentation.description
    if len(text) <= limit:
        return text
    corte = text[:limit].rsplit(" ", 1)[0].rstrip(" .,;:")
    return f"{corte}…"


def render_product_cards(
    repos: list[dict[str, Any]],
    presentations: dict[str, Presentation],
    *,
    columns: int = 2,
    maximum: int = 6,
) -> str:
    """A vitrine propriamente dita: o projeto como produto, não como repositório.

    Prioriza quem tem site no ar, porque é o que o visitante consegue abrir. Se
    não houver seis, completa com os de maior prioridade — mostrando o estado
    real de cada um em vez de deixar buraco na grade.
    """
    publicos = [
        presentations[str(repo["full_name"])]
        for repo in repos
        if not repo.get("private")
    ]
    ordenados = sorted(publicos, key=lambda p: (-p.marketing_priority, p.name.lower()))
    com_site = [p for p in ordenados if p.has_live_website]
    escolhidos = com_site[:maximum]
    if len(escolhidos) < maximum:
        restantes = [p for p in ordenados if p not in escolhidos]
        escolhidos += restantes[: maximum - len(escolhidos)]
    if not escolhidos:
        return "> Nenhum projeto público disponível nesta auditoria."

    largura = f"{100 // columns}%"
    linhas = ["<table>"]
    for inicio in range(0, len(escolhidos), columns):
        fatia = escolhidos[inicio : inicio + columns]
        linhas.append("<tr>")
        for item in fatia:
            linhas.extend([
                f'<td width="{largura}" valign="top">',
                "",
                f"<sub>`{item.category_label.upper()}`</sub>",
                "",
                f"### {item.name}",
                "",
                card_summary(item),
                "",
                status_pill(item),
                "",
                cta_buttons(item),
                "",
                "</td>",
            ])
        # Célula vazia mantém a grade retangular quando a última linha é ímpar.
        for _ in range(columns - len(fatia)):
            linhas.append(f'<td width="{largura}"></td>')
        linhas.append("</tr>")
    linhas.append("</table>")
    return "\n".join(linhas)


def render_ecosystem_map(
    repos: list[dict[str, Any]],
    presentations: dict[str, Presentation],
    *,
    por_ramo: int = 4,
) -> str:
    """Árvore do ecossistema por categoria canônica.

    Cada ramo traz quantos projetos existem e quantos têm site no ar — é a
    leitura que responde «onde há produto», e não só «onde há código».
    """
    ramos: dict[str, list[Presentation]] = {}
    for repo in repos:
        if repo.get("private"):
            continue
        item = presentations[str(repo["full_name"])]
        ramos.setdefault(item.category_label, []).append(item)

    # Ordem: quantidade de sites no ar, depois tamanho, depois a taxonomia.
    ordem = sorted(
        ramos.items(),
        key=lambda kv: (
            -sum(1 for p in kv[1] if p.has_live_website),
            -len(kv[1]),
            kv[0].lower(),
        ),
    )
    if not ordem:
        return "> Nenhum projeto público para mapear nesta auditoria."

    largura = max(len(nome) for nome, _ in ordem)
    linhas = ["```text", "ECOSSISTEMA", "│"]
    for indice, (categoria, itens) in enumerate(ordem):
        ultimo = indice == len(ordem) - 1
        tronco = "└──" if ultimo else "├──"
        haste = "   " if ultimo else "│  "
        vivos = sum(1 for p in itens if p.has_live_website)
        pontos = "." * max(3, largura - len(categoria) + 3)
        linhas.append(
            f"{tronco} {categoria} {pontos} {len(itens):>2} "
            f"{'projeto ' if len(itens) == 1 else 'projetos'} · {vivos} com site"
        )
        destaques = sorted(itens, key=lambda p: (-p.marketing_priority, p.name.lower()))[:por_ramo]
        nomes = [f"{p.name}{' ●' if p.has_live_website else ''}" for p in destaques]
        sobra = len(itens) - len(destaques)
        if sobra > 0:
            nomes.append(f"+{sobra}")
        linhas.append(f"{haste} └─ " + " · ".join(nomes))
        if not ultimo:
            linhas.append("│")
    linhas.extend([
        "```",
        "",
        "> `●` marca projeto com site verificado nesta auditoria. Os ramos usam os rótulos "
        "editoriais que o README já exibe; o equivalente canônico, para reuso externo, está "
        "em [`docs/project-catalog.json`](docs/project-catalog.json). Repositórios privados "
        "não entram no mapa público.",
    ])
    return "\n".join(linhas)


def render_public_projects(repos: list[dict[str, Any]], presentations: dict[str, Presentation], now: datetime) -> str:
    lines = [
        "<details>",
        "<summary><b>🌐 Public repository catalog</b></summary>",
        "",
        "| Projeto | Categoria | Status | Acesso |",
        "|:---|:---|:---|:---|",
    ]
    for repo in sorted((r for r in repos if not r.get("private")), key=lambda r: str(r["name"]).lower()):
        item = presentations[str(repo["full_name"])]
        lines.append(f"| **{repo['name']}** | {item.category_label} | {item.status} | {cta_cell(item)} |")
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


def render_live_projects(repos: list[dict[str, Any]], presentations: dict[str, Presentation]) -> str:
    # A ordem das colunas é a regra da vitrine: o site vem antes do código.
    lines = [
        "| Projeto | Website | Verificação | Código |",
        "|:---|:---|:---:|:---|",
    ]
    live = [
        presentations[str(repo["full_name"])]
        for repo in repos
        if presentations[str(repo["full_name"])].has_live_website
    ]
    for item in sorted(live, key=lambda p: p.name.lower()):
        lines.append(
            f"| **{item.name}** | **[▸ Abrir site]({item.website})** "
            f"| `HTTP {item.website_http_status}` | [código]({item.github}) |"
        )
    if not live:
        lines.append("| — | Nenhum site verificado nesta auditoria | — | — |")
    return "\n".join(lines)


def render_project_map(repos: list[dict[str, Any]], languages: dict[str, dict[str, int]], presentations: dict[str, Presentation], now: datetime) -> str:
    lines = [
        "<details>",
        "<summary><b>⌁ Complete project map</b></summary>",
        "",
        "| Projeto | Categoria | Stack | Status | Acesso |",
        "|:---|:---|:---|:---|:---|",
    ]
    for repo in sorted(repos, key=lambda r: str(r["name"]).lower()):
        item = presentations[str(repo["full_name"])]
        lines.append(
            f"| **{repo['name']}** | {item.category_label} | {stack_for(repo, languages)} | {item.status} | {cta_cell(item)} |"
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


def load_site_overrides() -> dict[str, dict[str, Any]]:
    """Lê `docs/README_SITES.json` e normaliza as duas formas aceitas.

    O arquivo histórico mapeia `"owner/repo": "https://..."`. A forma
    estendida aceita um objeto por repositório. Ambas continuam válidas —
    trocar o formato do manifesto não é requisito para nada aqui.
    """
    if not SITES_FILE.exists():
        return {}
    try:
        return normalize_site_overrides(json.loads(SITES_FILE.read_text(encoding="utf-8")))
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
    parser.add_argument("--skip-site-check", action="store_true", help="skip HTTP verification; renders every site as unverified")
    parser.add_argument("--site-timeout", type=float, default=15.0, help="per-request timeout for website verification (seconds)")
    parser.add_argument("--site-workers", type=int, default=6, help="maximum concurrent website checks")
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
    manual_overrides = load_site_overrides()
    featured_manifest = load_json_object(FEATURED_FILE)
    stack_manifest = load_json_object(STACK_FILE)

    # Descoberta: homepage do GitHub, depois o manifesto. Nunca deduzida do nome.
    # Private repositories are never included in the public live-project catalog.
    sites: dict[str, tuple[str | None, str]] = {}
    for repo in repos:
        if repo.get("private"):
            continue
        sites[str(repo["full_name"])] = discover_project_website(repo, manual_overrides)

    candidates = [url for url, _ in sites.values() if url]
    checks = {} if args.skip_site_check else check_websites(
        candidates, max_workers=args.site_workers, timeout=args.site_timeout
    )
    presentations = build_presentations(
        repos, sites=sites, checks=checks, featured_manifest=featured_manifest, now=now
    )
    verified_sites = live_site_map(presentations)

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
    text = replace_block(text, "WHAT-I-BUILD", render_what_i_build(repos, presentations))
    text = replace_block(text, "PRODUCT-CARDS", render_product_cards(repos, presentations))
    text = replace_block(text, "FEATURED-PROJECTS", render_featured_projects(repos, presentations, featured_manifest, now))
    text = replace_block(text, "WEBSITE-DIRECTORY", render_website_directory(repos, presentations))
    text = replace_block(text, "ARSENAL-STACK", render_arsenal_stack(rows, stack_manifest))
    text = replace_block(text, "LANGUAGE-BADGES", render_language_badges(rows))
    text = replace_block(text, "LANGUAGE-STATS", render_language_stats(repos, rows, generated_at))
    text = replace_block(text, "PUBLIC-PROJECTS", render_public_projects(repos, presentations, now))
    text = replace_block(text, "PRIVATE-PROJECTS", render_private_projects(repos, now))
    text = replace_block(text, "LIVE-PROJECTS", render_live_projects(repos, presentations))
    text = replace_block(text, "ECOSYSTEM-MAP", render_ecosystem_map(repos, presentations))
    text = replace_block(text, "PROJECT-MAP", render_project_map(repos, languages, presentations, now))

    catalog = build_catalog(list(presentations.values()))
    catalog_written = False
    if args.write:
        README.write_text(text, encoding="utf-8")
        render_svg(rows, ROOT / "assets" / "lang-stats.svg")
        render_snapshot_svg(repos, rows, verified_sites, now, generated_at, SNAPSHOT_SVG)
        # Conteúdo igual não é gravado: o catálogo não deve produzir commit vazio.
        catalog_written = write_catalog_if_changed(catalog, CATALOG_FILE)
    else:
        print(text[:500])
    print(json.dumps({
        "repositories": len(repos),
        "public_repositories": sum(not repo.get("private") for repo in repos),
        "private_repositories": sum(bool(repo.get("private")) for repo in repos),
        "excluded_repositories": sorted(load_excluded_names()),
        "public_languages": len(rows),
        "declared_sites": len([url for url, _ in sites.values() if url]),
        "verified_sites": len(verified_sites),
        "unreachable_sites": sorted(
            item.name for item in presentations.values()
            if item.website_status == "unreachable"
        ),
        "catalog_projects": len(catalog["projects"]),
        "catalog_written": catalog_written,
        "site_check_skipped": bool(args.skip_site_check),
        "write": bool(args.write),
    }, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
