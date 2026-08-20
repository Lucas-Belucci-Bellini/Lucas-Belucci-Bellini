#!/usr/bin/env python3
"""Gera cards locais do perfil a partir da API GraphQL do GitHub.

Os cards são SVGs versionados no repositório para que o README não dependa de
serviços públicos de terceiros sujeitos a pausa, cobrança ou rate limit.
"""

from __future__ import annotations

import html
import json
import os
import urllib.error
import urllib.request
from datetime import datetime, timedelta, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / "assets"
USER = os.environ.get("GH_USER", "Lucas-Belucci-Bellini")
TOKEN = os.environ.get("GITHUB_TOKEN", "")
GRAPHQL_URL = "https://api.github.com/graphql"

QUERY = """
query($login:String!, $from:DateTime!, $to:DateTime!) {
  user(login:$login) {
    login
    name
    contributionsCollection(from:$from, to:$to) {
      totalCommitContributions
      totalIssueContributions
      totalPullRequestContributions
      totalPullRequestReviewContributions
      restrictedContributionsCount
      contributionCalendar { totalContributions }
    }
    repositories(first:1, privacy:PUBLIC, ownerAffiliations:OWNER) { totalCount }
  }
}
"""

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


def esc(value: object) -> str:
    return html.escape(str(value), quote=True)


def request_data() -> dict:
    if not TOKEN:
        raise RuntimeError("GITHUB_TOKEN ausente")
    now = datetime.now(timezone.utc)
    variables = {
        "login": USER,
        "from": (now - timedelta(days=365)).strftime("%Y-%m-%dT00:00:00Z"),
        "to": now.strftime("%Y-%m-%dT23:59:59Z"),
    }
    payload = json.dumps({"query": QUERY, "variables": variables}).encode()
    req = urllib.request.Request(
        GRAPHQL_URL,
        data=payload,
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {TOKEN}",
            "Content-Type": "application/json",
            "User-Agent": "Lucas-Belucci-Bellini-profile-cards",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as response:
            result = json.load(response)
    except urllib.error.HTTPError as exc:
        raise RuntimeError(f"GitHub GraphQL HTTP {exc.code}") from exc
    errors = result.get("errors") or []
    if errors:
        raise RuntimeError("; ".join(str(error.get("message", "GraphQL error")) for error in errors))
    user = (result.get("data") or {}).get("user")
    if not user:
        raise RuntimeError("usuário não encontrado na API GraphQL")
    collection = user["contributionsCollection"]
    return {
        "login": user["login"],
        "name": user.get("name") or user["login"],
        "contributions": collection["contributionCalendar"]["totalContributions"],
        "commits": collection["totalCommitContributions"],
        "issues": collection["totalIssueContributions"],
        "pull_requests": collection["totalPullRequestContributions"],
        "reviews": collection["totalPullRequestReviewContributions"],
        "restricted": collection["restrictedContributionsCount"],
        "repositories": user["repositories"]["totalCount"],
        "generated": datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC"),
    }


def frame(width: int, height: int, title: str, subtitle: str = "") -> list[str]:
    return [
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" role="img" aria-label="{esc(title)}">',
        '<defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#0e0c16"/><stop offset="1" stop-color="#0b0910"/></linearGradient><linearGradient id="scan" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#d4a24e" stop-opacity="0"/><stop offset="0.5" stop-color="#e8c07a" stop-opacity="0.9"/><stop offset="1" stop-color="#d4a24e" stop-opacity="0"/></linearGradient></defs>',
        '<rect width="100%" height="100%" rx="14" fill="url(#bg)"/>',
        f'<rect x="5" y="5" width="{width-10}" height="{height-10}" rx="11" fill="none" stroke="{GOLD}" stroke-opacity="0.35"/>',
        f'<text x="28" y="35" fill="{GOLD_LIGHT}" font-family="DejaVu Sans Mono,monospace" font-size="14" font-weight="700" letter-spacing="2">⬡ {esc(title)}</text>',
        f'<text x="28" y="54" fill="{DIM}" font-family="DejaVu Sans Mono,monospace" font-size="10" letter-spacing="1.2">{esc(subtitle)}</text>',
        f'<line x1="24" y1="66" x2="{width-24}" y2="66" stroke="{GOLD}" stroke-opacity="0.25"/>',
        f'<rect x="24" y="67" width="180" height="2" fill="url(#scan)" opacity="0.85"/>',
    ]


def metric(x: int, y: int, w: int, label: str, value: object, color: str = GOLD_LIGHT) -> str:
    return "".join([
        f'<rect x="{x}" y="{y}" width="{w}" height="76" rx="8" fill="{SURFACE}" stroke="{GOLD}" stroke-opacity="0.22"/>',
        f'<text x="{x+16}" y="{y+31}" fill="{color}" font-family="DejaVu Sans Mono,monospace" font-size="25" font-weight="700">{esc(value)}</text>',
        f'<text x="{x+16}" y="{y+55}" fill="{MUTED}" font-family="DejaVu Sans Mono,monospace" font-size="10" letter-spacing="1">{esc(label)}</text>',
    ])


def stats_svg(data: dict) -> str:
    lines = frame(880, 260, "RELATÓRIO DE CAMPO // GITHUB SNAPSHOT", "métrica oficial via GitHub GraphQL · janela móvel de 365 dias")
    lines.append(metric(24, 86, 198, "CONTRIBUIÇÕES", data["contributions"], GOLD_LIGHT))
    lines.append(metric(238, 86, 198, "COMMITS", data["commits"], GOLD))
    lines.append(metric(452, 86, 198, "PULL REQUESTS", data["pull_requests"], GREEN))
    lines.append(metric(666, 86, 190, "ISSUES", data["issues"], PURPLE))
    lines.append(metric(24, 174, 198, "REVIEWS", data["reviews"], GOLD_LIGHT))
    lines.append(metric(238, 174, 198, "REPOSITÓRIOS PÚBLICOS", data["repositories"], GOLD))
    lines.append(metric(452, 174, 198, "RESTRITOS", data["restricted"], MUTED))
    lines.append(metric(666, 174, 190, "STATUS", "ONLINE", GREEN))
    lines.append(f'<text x="24" y="246" fill="{DIM}" font-family="DejaVu Sans Mono,monospace" font-size="9">atualizado automaticamente · {esc(data["generated"])}</text>')
    lines.append('</svg>')
    return "\n".join(lines) + "\n"


def streak_svg(data: dict) -> str:
    lines = frame(495, 195, "CONTINUIDADE // FIELD STREAK", "contagem oficial de contribuições · últimos 365 dias")
    lines.append(metric(24, 84, 142, "TOTAL", data["contributions"], GOLD_LIGHT))
    lines.append(metric(176, 84, 142, "COMMITS", data["commits"], GOLD))
    lines.append(metric(328, 84, 143, "PRs", data["pull_requests"], GREEN))
    lines.append(f'<text x="24" y="180" fill="{DIM}" font-family="DejaVu Sans Mono,monospace" font-size="9">fonte: GitHub GraphQL · {esc(data["generated"])}</text>')
    lines.append('</svg>')
    return "\n".join(lines) + "\n"


def projects_svg() -> str:
    """Gera o painel local dos quatro projetos destacados do perfil."""
    lines = frame(880, 236, "ARSENAL // PROJETOS EM DESTAQUE", "cards locais · links diretos preservados no README")
    projects = [
        ("Projeto Baluarte", "J.A.R.V.I.S. · Vite · Electron", GOLD_LIGHT),
        ("Digital Logic Sim CE", "CPUs de 8 → 64 bits · Unity", GREEN),
        ("Stock Analyzer", "IA · RSI · MACD · alertas", PURPLE),
        ("Baluarte Obra Segura", "web · Electron · segurança", RED),
    ]
    positions = [(24, 84), (452, 84), (24, 158), (452, 158)]
    for (name, description, color), (x, y) in zip(projects, positions):
        lines.append(f'<rect x="{x}" y="{y}" width="404" height="58" rx="8" fill="{SURFACE}" stroke="{GOLD}" stroke-opacity="0.22"/>')
        lines.append(f'<circle cx="{x+22}" cy="{y+29}" r="7" fill="{color}"/><text x="{x+42}" y="{y+25}" fill="{PARCHMENT}" font-family="DejaVu Sans Mono,monospace" font-size="13" font-weight="700">{esc(name)}</text>')
        lines.append(f'<text x="{x+42}" y="{y+43}" fill="{MUTED}" font-family="DejaVu Sans Mono,monospace" font-size="10">{esc(description)}</text>')
    lines.append('</svg>')
    return "\n".join(lines) + "\n"


def trophies_svg(data: dict) -> str:
    lines = frame(880, 190, "TROFÉUS // MISSÃO EM CAMPO", "marcos derivados da atividade oficial do GitHub · sem serviço externo")
    labels = [
        ("COMMIT ENGINE", data["commits"], GOLD_LIGHT),
        ("PR COMMANDER", data["pull_requests"], GREEN),
        ("ISSUE SCOUT", data["issues"], PURPLE),
        ("REVIEW SENTINEL", data["reviews"], GOLD),
        ("REPO ARCHITECT", data["repositories"], RED),
    ]
    widths = [158, 158, 158, 158, 158]
    x = 24
    for (label, value, color), width in zip(labels, widths):
        lines.append(metric(x, 84, width, label, value, color))
        x += width + 16
    lines.append('</svg>')
    return "\n".join(lines) + "\n"


def main() -> int:
    data = request_data()
    ASSETS.mkdir(parents=True, exist_ok=True)
    outputs = {
        "profile-stats.svg": stats_svg(data),
        "profile-streak.svg": streak_svg(data),
        "profile-trophies.svg": trophies_svg(data),
        "profile-projects.svg": projects_svg(),
    }
    for name, content in outputs.items():
        (ASSETS / name).write_text(content, encoding="utf-8")
    print(json.dumps(data, ensure_ascii=False, sort_keys=True))
    print("gerados=" + ",".join(outputs))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
