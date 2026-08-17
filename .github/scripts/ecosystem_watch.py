#!/usr/bin/env python3
"""Daily, low-noise health/index pass over the public project ecosystem.

The profile repository is an index, not a mirror. This script records the latest
commit SHA of each public repository and counts commits added since the previous
scan. It writes one state snapshot and one human-readable report per scan, but
only the files that actually changed are committed by Actions.
"""
from __future__ import annotations

import json
import os
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
STATE = ROOT / "docs" / "ECOSYSTEM-COMMIT-STATE.json"
REPORT = ROOT / "docs" / "ECOSYSTEM-COMMIT-MONITOR.md"
API = "https://api.github.com"
USER = os.environ.get("GH_USER", "Lucas-Belucci-Bellini")
TOKEN = os.environ.get("GITHUB_TOKEN", "")


def api(path: str):
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "baluarte-ecosystem-watch",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if TOKEN:
        headers["Authorization"] = f"Bearer {TOKEN}"
    req = urllib.request.Request(API + path, headers=headers)
    with urllib.request.urlopen(req, timeout=30) as response:
        return json.load(response)


def repos():
    out = []
    page = 1
    while True:
        data = api(f"/users/{urllib.parse.quote(USER)}/repos?type=owner&per_page=100&page={page}")
        if not data:
            return out
        out.extend(r for r in data if not r.get("fork"))
        if len(data) < 100:
            return out
        page += 1


def latest_commit(owner: str, name: str, branch: str):
    commits = api(f"/repos/{owner}/{urllib.parse.quote(name)}/commits?sha={urllib.parse.quote(branch)}&per_page=1")
    if not commits:
        return None
    c = commits[0]
    return {
        "sha": c["sha"],
        "date": c.get("commit", {}).get("committer", {}).get("date"),
        "message": c.get("commit", {}).get("message", "").splitlines()[0][:140],
        "url": c.get("html_url"),
    }


def compare_count(owner: str, name: str, base: str, head: str):
    if not base or base == head:
        return 0
    try:
        data = api(f"/repos/{owner}/{urllib.parse.quote(name)}/compare/{base}...{head}")
        return int(data.get("ahead_by", 0))
    except Exception:
        # History may have been rewritten or the comparison may have expired.
        return None


def main():
    previous = {}
    if STATE.exists():
        try:
            previous = json.loads(STATE.read_text(encoding="utf-8")).get("repositories", {})
        except Exception:
            previous = {}

    current = {}
    changes = []
    for repo in repos():
        if repo.get("private"):
            continue
        name = repo["name"]
        branch = repo.get("default_branch") or "main"
        try:
            latest = latest_commit(USER, name, branch)
        except Exception as exc:
            current[name] = {"branch": branch, "error": str(exc)[:180]}
            continue
        if not latest:
            current[name] = {"branch": branch, "empty": True}
            continue
        old = previous.get(name, {}).get("sha")
        new_count = compare_count(USER, name, old, latest["sha"]) if old else None
        current[name] = {"branch": branch, **latest}
        if old and old != latest["sha"]:
            changes.append({"name": name, "count": new_count, **latest})

    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    STATE.parent.mkdir(parents=True, exist_ok=True)
    STATE.write_text(json.dumps({"schema": 1, "scanned_at": now, "repositories": current}, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    lines = [
        "# Ecosystem Commit Monitor",
        "",
        "> Monitoramento de baixo ruído do ecossistema público. Este arquivo registra o último commit conhecido de cada repositório; ele não espelha os repositórios nem cria um commit por mudança individual.",
        "",
        f"**Última varredura:** `{now}`  ",
        f"**Repositórios acompanhados:** `{len(current)}`  ",
        f"**Repositórios com mudanças desde a última varredura:** `{len(changes)}`",
        "",
        "## Mudanças detectadas",
        "",
    ]
    if changes:
        for item in sorted(changes, key=lambda x: x["name"].lower()):
            count = "quantidade não determinada" if item["count"] is None else f"{item['count']} commit(s)"
            lines.append(f"- **{item['name']}** — {count} — [{item['sha'][:12]}]({item['url']}) — {item['message']}")
    else:
        lines.append("- Nenhuma mudança desde a última varredura.")

    lines += [
        "",
        "## Arquitetura",
        "",
        "```text",
        "projetos individuais",
        "       │",
        "       │ latest SHA + comparação",
        "       ▼",
        "ecosystem_watch.py",
        "       │",
        "       ├── ECOSYSTEM-COMMIT-STATE.json",
        "       └── ECOSYSTEM-COMMIT-MONITOR.md",
        "       │",
        "       ▼",
        "um commit agregado no perfil quando houver mudança",
        "```",
        "",
        "### Regra de estabilidade",
        "",
        "O perfil **não deve** tentar transformar cada commit dos projetos em um commit próprio. Ele acompanha os commits, agrega as mudanças e mantém apenas o estado necessário para continuar a observação.",
        "",
        "Isso permite crescer de poucos projetos para dezenas ou centenas sem transformar o repositório de perfil em um espelho gigantesco.",
        "",
    ]
    REPORT.write_text("\n".join(lines), encoding="utf-8")


if __name__ == "__main__":
    main()
