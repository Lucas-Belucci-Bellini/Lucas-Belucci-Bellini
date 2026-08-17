#!/usr/bin/env python3
"""Hourly health/index pass over the public project ecosystem."""
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


def migrate_metrics(state: dict) -> tuple[dict, dict]:
    """Return V2 metrics and explicit legacy baseline without double counting."""
    baseline = state.get("baseline") or {}
    legacy = int(baseline.get("legacy_tracked_commits", LEGACY_BASELINE))
    metrics = state.get("metrics") or {}

    if state.get("schema", 0) >= 6:
        return {
            "project_commits_total": int(metrics.get("project_commits_total", 0)),
            "monitor_commits_total": int(metrics.get("monitor_commits_total", 0)),
        }, {"legacy_tracked_commits": legacy, "source": baseline.get("source", "legacy monitor state")}

    # Schema 5 stored the old baseline in project_commits and monitor commits
    # separately. Convert it without counting the 1538 baseline twice.
    old_project = int(metrics.get("project_commits", legacy))
    old_monitor = int(metrics.get("monitor_commits", 0))
    project_delta = max(0, old_project - legacy)
    return {
        "project_commits_total": project_delta,
        "monitor_commits_total": old_monitor,
    }, {"legacy_tracked_commits": legacy, "source": "legacy monitor state"}


def calculate_metrics(state: dict, detected_project_commits: int) -> dict:
    current, baseline = migrate_metrics(state)
    project_total = current["project_commits_total"] + detected_project_commits
    monitor_total = current["monitor_commits_total"] + 1
    tracked_total = baseline["legacy_tracked_commits"] + project_total + monitor_total
    return {
        "project_commits_total": project_total,
        "monitor_commits_total": monitor_total,
        "tracked_commits_total": tracked_total,
        "project_commits_this_run": detected_project_commits,
        "monitor_commit_this_run": 1,
        "legacy_baseline": baseline["legacy_tracked_commits"],
    }


def main():
    previous_state = {}
    if STATE.exists():
        try:
            previous_state = json.loads(STATE.read_text(encoding="utf-8"))
        except Exception:
            previous_state = {}

    previous = previous_state.get("repositories", {})
    previous_health = previous_state.get("health", {})
    started_at = datetime.now(timezone.utc)
    run_id = started_at.strftime("RUN-%Y%m%d-%H%M%S")

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

    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    metrics = calculate_metrics(previous_state, detected_project_commits)
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
                "schema": 6,
                "run": {
                    "run_id": run_id,
                    "started_at": started_at.strftime("%Y-%m-%dT%H:%M:%SZ"),
                    "completed_at": now,
                    "status": "candidate",
                    "publication_rule": "This snapshot is the payload of the monitor commit; if git commit/push fails, this candidate is not published and the next run starts from the last published state.",
                },
                "baseline": {
                    "legacy_tracked_commits": metrics["legacy_baseline"],
                    "source": "legacy monitor state",
                },
                "repositories": current,
                "metrics": metrics,
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
        f"**Run:** `{run_id}`  ",
        f"**Última varredura:** `{now}`  ",
        "**Intervalo configurado:** `1 hora`  ",
        f"**Repositórios acompanhados:** `{len(current)}`  ",
        f"**Repositórios com mudanças desde a última varredura:** `{len(changes)}`  ",
        f"**Falhas de consulta:** `{len(errors)}`",
        "",
        "## Saúde do monitor",
        "",
        f"- **Status:** `{'🟢 HEALTHY' if not errors else '🟡 DEGRADED'}`",
        f"- **Último scan:** `{now}`",
        "- **Agendamento:** `a cada hora, no minuto 17 UTC`",
        "- **Snapshot:** `candidate — publicado somente se o commit/push terminar com sucesso`",
        "- **GitHub Contributions:** métrica oficial do GitHub, não calculada por este monitor.",
        "",
        "## Contadores",
        "",
        f"- **Baseline legado:** `{metrics['legacy_baseline']}`",
        f"- **Commits de projetos desde o baseline:** `{metrics['project_commits_total']}`",
        f"- **Snapshots publicados pelo monitor desde o baseline:** `{metrics['monitor_commits_total']}`",
        f"- **Commits rastreados pelo ecossistema:** `{metrics['tracked_commits_total']}`",
        f"- **Commits de projetos detectados nesta hora:** `{detected_project_commits}`",
        "",
        "> O contador do monitor só avança em uma execução que prepara um snapshot para publicação. Se o `git commit`/`push` falhar, esse estado não chega à branch e o próximo ciclo parte do último snapshot publicado.",
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
        "       ├── delta de commits dos projetos",
        "       ├── + 1 snapshot candidato",
        "       ├── health check",
        "       └── contador acumulado",
        "       │",
        "       ▼",
        "git commit + push",
        "       │",
        "       ▼",
        "snapshot publicado",
        "```",
        "",
        "### Regras de estabilidade",
        "",
        "1. O perfil faz uma varredura programada por hora.",
        "2. `project_commits_total` contém somente deltas de commits detectados nos projetos desde o baseline legado.",
        "3. `monitor_commits_total` contém somente snapshots que chegaram a ser publicados pelo monitor.",
        "4. `tracked_commits_total = legacy_baseline + project_commits_total + monitor_commits_total`.",
        "5. O snapshot não tenta registrar seu próprio SHA: um commit não pode conhecer o SHA que ainda está sendo criado. A publicação é considerada bem-sucedida pelo workflow após `git commit` e `git push`.",
        "6. Retries e backoff protegem contra falhas transitórias da API.",
        "7. Repositórios novos do usuário são descobertos automaticamente; forks são ignorados.",
        "8. O contador próprio do ecossistema não tenta reproduzir a métrica oficial de GitHub Contributions.",
        "",
        "Em um ano comum, uma execução horária representa no máximo 8.760 snapshots programados; o scheduler do GitHub pode atrasar a execução real.",
        "",
    ]
    REPORT.write_text("\n".join(lines), encoding="utf-8")


if __name__ == "__main__":
    main()
