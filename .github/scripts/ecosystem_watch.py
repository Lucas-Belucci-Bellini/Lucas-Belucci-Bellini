#!/usr/bin/env python3
"""Hourly health/index pass over the public project ecosystem.

The profile repository is an index, not a mirror. This script records the latest
commit SHA of each public repository, counts commits added since the previous
scan, and maintains a cumulative ecosystem counter. The counter is intentionally
separate from GitHub's official Contributions metric.

Important: the counter includes the commit produced by this hourly monitor run.
That run's state is written before the workflow commits it, so the published
snapshot represents the commit that is being created by the monitor itself.
"""
from __future__ import annotations

import json
import os
import time
import urllib.error
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
MAX_RETRIES = 4
LEGACY_BASELINE = 1538


def api(path: str):
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "profile-ecosystem-watch",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if TOKEN:
        headers["Authorization"] = f"Bearer {TOKEN}"
    last_error = None
    for attempt in range(MAX_RETRIES):
        req = urllib.request.Request(API + path, headers=headers)
        try:
            with urllib.request.urlopen(req, timeout=30) as response:
                return json.load(response)
        except urllib.error.HTTPError as exc:
            last_error = exc
            retryable = exc.code == 429 or exc.code >= 500
            if exc.code == 403:
                retryable = exc.headers.get("X-RateLimit-Remaining") == "0"
            if not retryable or attempt == MAX_RETRIES - 1:
                raise
            retry_after = exc.headers.get("Retry-After")
            delay = int(retry_after) if retry_after and retry_after.isdigit() else 2 ** attempt
            time.sleep(min(delay, 30))
        except (urllib.error.URLError, TimeoutError) as exc:
            last_error = exc
            if attempt == MAX_RETRIES - 1:
                raise
            time.sleep(min(2 ** attempt, 30))
    raise RuntimeError(f"GitHub API failed after retries: {last_error}")


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
        return None


def main():
    previous_state = {}
    if STATE.exists():
        try:
            previous_state = json.loads(STATE.read_text(encoding="utf-8"))
        except Exception:
            previous_state = {}

    previous = previous_state.get("repositories", {})
    previous_metrics = previous_state.get("metrics", {})
    previous_health = previous_state.get("health", {})

    project_commits = int(previous_metrics.get("project_commits", LEGACY_BASELINE))
    monitor_commits = int(previous_metrics.get("monitor_commits", 0))
    current = {}
    changes = []
    errors = []
    detected_project_commits = 0

    for repo in repos():
        if repo.get("private"):
            continue
        name = repo["name"]
        branch = repo.get("default_branch") or "main"
        try:
            latest = latest_commit(USER, name, branch)
        except Exception as exc:
            current[name] = {"branch": branch, "error": str(exc)[:180]}
            errors.append({"name": name, "stage": "latest_commit", "error": str(exc)[:180]})
            continue
        if not latest:
            current[name] = {"branch": branch, "empty": True}
            continue
        old = previous.get(name, {}).get("sha")
        new_count = compare_count(USER, name, old, latest["sha"]) if old else None
        current[name] = {"branch": branch, **latest}
        if old and old != latest["sha"]:
            changes.append({"name": name, "count": new_count, **latest})
            if new_count is not None:
                detected_project_commits += new_count

    # A successful scan represents the monitor commit that will publish this snapshot.
    monitor_commits += 1
    project_commits += detected_project_commits
    tracked_commits = project_commits + monitor_commits
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    health = {
        "status": "healthy" if not errors else "degraded",
        "last_successful_scan": now,
        "last_scan_had_errors": bool(errors),
        "repositories_scanned": len(current),
        "repositories_changed": len(changes),
        "query_errors": len(errors),
        "previous_status": previous_health.get("status", "unknown"),
        "scheduler": "hourly (cron minute 17)",
        "snapshot_commit_expected": True,
    }

    STATE.parent.mkdir(parents=True, exist_ok=True)
    STATE.write_text(
        json.dumps(
            {
                "schema": 5,
                "scanned_at": now,
                "scan_interval": "hourly",
                "repositories": current,
                "metrics": {
                    "tracked_commits": tracked_commits,
                    "project_commits": project_commits,
                    "monitor_commits": monitor_commits,
                    "detected_project_commits_this_scan": detected_project_commits,
                    "monitor_commit_this_scan": 1,
                },
                "health": health,
                "summary": {
                    "repositories_scanned": len(current),
                    "repositories_changed": len(changes),
                    "errors": len(errors),
                },
                "errors": errors,
            },
            indent=2,
            ensure_ascii=False,
        ) + "\n",
        encoding="utf-8",
    )

    lines = [
        "# Ecosystem Commit Monitor",
        "",
        "> Snapshot horário do ecossistema público. O perfil acompanha o último commit de cada repositório e agrega mudanças; ele não espelha o histórico inteiro dos projetos.",
        "",
        f"**Última varredura:** `{now}`  ",
        "**Intervalo configurado:** `1 hora`  ",
        f"**Repositórios acompanhados:** `{len(current)}`  ",
        f"**Repositórios com mudanças desde a última varredura:** `{len(changes)}`  ",
        f"**Falhas de consulta:** `{len(errors)}`",
        "",
        "## Saúde do monitor",
        "",
        f"- **Status:** `{'🟢 HEALTHY' if not errors else '🟡 DEGRADED'}`",
        f"- **Último scan bem-sucedido:** `{now}`",
        "- **Agendamento:** `a cada hora, no minuto 17 UTC`",
        f"- **Snapshot esperado nesta execução:** `{'SIM' if True else 'NÃO'}`",
        "- **GitHub Contributions:** métrica oficial do GitHub, não calculada por este monitor.",
        "",
        "## Contadores",
        "",
        f"- **Commits rastreados pelo ecossistema:** `{tracked_commits}`",
        f"- **Commits dos projetos:** `{project_commits}`",
        f"- **Commits do próprio monitor:** `{monitor_commits}`",
        f"- **Commits de projetos detectados nesta hora:** `{detected_project_commits}`",
        "",
        "> O contador acima é uma métrica própria do monitor. Cada execução horária bem-sucedida acrescenta 1 ao contador de commits do próprio monitor, porque a execução gera o commit que publica este snapshot.",
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

    if errors:
        lines += ["", "## Erros de consulta", ""]
        for item in errors:
            lines.append(f"- **{item['name']}** — `{item['stage']}` — {item['error']}")

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
        "       ├── estado dos projetos",
        "       ├── commits dos projetos",
        "       ├── + 1 commit do monitor",
        "       ├── health check",
        "       └── contador acumulado",
        "       │",
        "       ▼",
        "snapshot agregado a cada hora",
        "```",
        "",
        "### Regras de estabilidade",
        "",
        "1. O perfil faz uma varredura programada por hora.",
        "2. Cada execução bem-sucedida acrescenta exatamente 1 ao contador de commits do monitor, correspondente ao commit que publica o snapshot.",
        "3. As mudanças dos projetos são agregadas: um snapshot pode registrar quantos commits cada repositório recebeu desde a varredura anterior, sem copiar esses commits para o perfil.",
        "4. Retries e backoff protegem contra falhas transitórias da API.",
        "5. Repositórios novos do usuário são descobertos automaticamente; forks são ignorados.",
        "6. O contador próprio do ecossistema não tenta reproduzir a métrica oficial de GitHub Contributions.",
        "7. O health check registra explicitamente o último scan, erros e o snapshot esperado.",
        "",
        "Em um ano comum, uma execução horária representa no máximo 8.760 snapshots programados; o scheduler do GitHub pode atrasar a execução real.",
        "",
    ]
    REPORT.write_text("\n".join(lines), encoding="utf-8")


if __name__ == "__main__":
    main()
