#!/usr/bin/env python3
"""Critério de saída da Fase 3: coleta Python = coleta Rust, contra o mesmo GitHub.

Sobe um GitHub simulado (tests/fake_github.py) com um ecossistema sintético —
inventário paginado, privados, forks, arquivados, colaboração, exclusões,
homepages boas e ruins, descrições com `|`, `\\`, acentos e U+001F — e roda os
dois programas contra ele:

    catálogo       update_profile.py --catalog-out   ×  profile-core catalog build
                   (com PROFILE_GITHUB_TOKEN e sem ele)
    monitor        ecosystem_watch.py --root         ×  profile-core sync commits --write --no-db
                   (5 varreduras encadeadas: commits novos, comparação que falha,
                   409, 503 que passa na segunda tentativa, rate limit, repositório
                   novo e sumido, varredura sem mudança)
    contribuições  update_contribution_timeline.py   ×  profile-core sync contributions --write --no-db

Exige os mesmos bytes em cada arquivo e a mesma saída do monitor.

    python3 tests/e2e/github_parity.py target/release/profile-core

Use o Python do CI (3.12.14).
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Callable

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests"))

from fake_github import FakeGitHub, Response  # noqa: E402

OWNER = "Lucas-Belucci-Bellini"
PARTNER = "parceiro"
NOW = "2026-09-25T12:00:00Z"
NOW_DT = datetime(2026, 9, 25, 12, tzinfo=timezone.utc)
CLEAN_ENV = {key: value for key, value in os.environ.items()
             if "TOKEN" not in key and not key.startswith("GITHUB_") and key not in {"GH_USER", "PROFILE_REPOSITORY_NAME", "PROFILE_LOGIN", "DATABASE_URL"}}


def ago(days: int) -> str:
    return (NOW_DT - timedelta(days=days)).strftime("%Y-%m-%dT%H:%M:%SZ")


def sha(seed: int, version: int = 0) -> str:
    return f"{seed:08x}{version:04x}".ljust(40, "a")[:40]


# ------------------------------------------------------------------ inventário

SPECIAL = {
    0: {"name": "Projeto-Baluarte", "description": "Plataforma do ecossistema Baluarte.", "homepage": "https://projeto-baluarte.example.org"},
    1: {"name": "baluarte-dominio", "description": "Domínio extraído."},
    2: {"name": "baluarte-obra-segura", "description": "Obra segura.", "homepage": "  https://obra.example.org  "},
    3: {"name": "Veritas", "description": "Digital logic local-first.", "homepage": "https://veritas.example.org"},
    4: {"name": "daily-notes", "description": "Anotações diárias (ai por substring, A6)."},
    5: {"name": "Academic-Java", "description": "Atividade de Java.", "homepage": "not a url"},
    6: {"name": "black-mesa-mod", "description": "Mod de game.", "homepage": ""},
    7: {"name": "sujok-backend", "description": "Banco de dados."},
    8: {"name": "Excluded-One", "description": "Excluído pelo nome."},
    9: {"name": "Excluded-Two", "description": "Excluído pelo nome completo."},
    10: {"name": "descricao-rara", "description": "  Com | barra, \\ contrabarra,\ttab e\x1fseparador — ação útil  "},
    11: {"name": "sem-descricao", "description": None},
    12: {"name": "descricao-vazia", "description": ""},
    13: {"name": "curta", "description": "Curta."},
    14: {"name": "data-ruim", "description": "Data de push que não é data.", "pushed_at": "ontem"},
    15: {"name": "sem-data", "description": "Sem push.", "pushed_at": None, "updated_at": None},
    # Empata em prioridade com os proj-NNN: o desempate é por nome SEM caixa.
    18: {"name": "Zeta-Projeto", "description": "Maiúscula no fim do alfabeto, empatada com os proj-NNN."},
}


def owner_repo(i: int) -> dict[str, Any]:
    spec = SPECIAL.get(i, {})
    name = spec.get("name", f"proj-{i:03d}")
    repo = {
        "id": 10_000 + i,
        "node_id": f"R_{i}",
        "name": name,
        "full_name": f"{OWNER}/{name}",
        "owner": {"id": 1, "login": OWNER, "type": "User"},
        "description": spec.get("description", f"Projeto sintético número {i} com descrição suficiente."),
        "private": i % 13 == 12,
        "visibility": "private" if i % 13 == 12 else "public",
        "fork": i % 11 == 10,
        "archived": i % 17 == 16,
        "homepage": spec.get("homepage", f"https://site-{i}.example.org" if i % 5 == 0 else None),
        "pushed_at": spec.get("pushed_at", ago((i * 7) % 500)),
        "updated_at": spec.get("updated_at", ago((i * 7) % 500)),
        "size": (i * 1234) % 60000,
        "default_branch": "dev/x y" if i == 20 else ("master" if i % 9 == 0 else "main"),
        "language": "Python",
        "topics": [],
    }
    if i % 11 == 10 and i % 2 == 0:
        repo["pushed_at"] = None  # fork sem push: "Experimental"
    return repo


OWNER_REPOS = [owner_repo(i) for i in range(106)]
PARTNER_REPOS = [
    {**owner_repo(200 + k), "name": name, "full_name": f"{PARTNER}/{name}",
     "owner": {"id": 2, "login": PARTNER, "type": "Organization"}, "private": False, "visibility": "public", "fork": False}
    for k, name in enumerate(("colab-game", "colab-site", "Veritas"))
]
ALL_REPOS = OWNER_REPOS + PARTNER_REPOS
PUBLIC_OWNER = [r for r in OWNER_REPOS if not r["private"]]


def pages(items: list[dict[str, Any]], path: Callable[[int], str]) -> dict[str, list[dict[str, Any]]]:
    out = {}
    for page in range(1, len(items) // 100 + 2):
        out[path(page)] = items[(page - 1) * 100: page * 100]
    return out


# ------------------------------------------------------------------ raiz

SITES_MANIFEST = {
    f"{OWNER}/proj-021": "https://manifest-21.example.org",
    f"{OWNER}/proj-022": {"website": " https://manifest-22.example.org "},
    f"{OWNER}/proj-023": {"website": ""},
    f"{OWNER}/proj-024": "ftp://nao-http.example.org",
    f"{OWNER}/proj-025": "https://ignorado-porque-homepage.example.org",
    f"{OWNER}/sujok-backend": "https://sujok.example.org",
    f"{PARTNER}/colab-site": "https://colab.example.org",
    f"{OWNER}/nao-existe": "https://fantasma.example.org",
}
FEATURED = {"projects": [
    {"name": "Projeto-Baluarte", "label": "FLAGSHIP", "order": 1, "priority": 100},
    {"name": "Veritas", "label": "LOGIC", "order": 2, "priority": 95},
    {"name": "proj-021", "label": "ORDEM EM TEXTO", "order": "3"},
    {"name": "proj-030", "label": "PRIORIDADE RUIM", "order": 4, "priority": "abc"},
    {"name": "proj-031", "label": "SEM ORDEM"},
    {"name": "black-mesa-mod", "label": "GAME", "order": 6, "priority": 70},
    {"label": "SEM NOME", "order": 7},
]}
EXCLUDED = {"reason": "Sintético.", "repositories": ["Excluded-One", f"{OWNER}/Excluded-Two"]}


def site_checks() -> dict[str, dict[str, Any]]:
    """Resultado para toda URL que a descoberta pode produzir (a mais é permitido)."""
    urls = {str(r["homepage"]).strip() for r in ALL_REPOS if r.get("homepage")}
    for value in SITES_MANIFEST.values():
        urls.add((value["website"] if isinstance(value, dict) else value).strip())
    checks = {}
    for n, url in enumerate(sorted(urls)):
        if n % 4 == 0:
            checks[url] = {"status": "unreachable", "http_status": 404 if n % 8 == 0 else 0, "final_url": url}
        elif n % 7 == 0:
            checks[url] = {"status": "verified", "http_status": 200, "final_url": url.rstrip("/") + "/home"}
        else:
            checks[url] = {"status": "verified", "http_status": 200}
    return checks


def build_root(root: Path) -> None:
    shutil.copytree(ROOT / "tests" / "fixtures" / "profile" / "input", root)
    (root / "docs" / "README_SITES.json").write_text(json.dumps(SITES_MANIFEST, ensure_ascii=False), encoding="utf-8")
    (root / "docs" / "README_FEATURED.json").write_text(json.dumps(FEATURED, ensure_ascii=False), encoding="utf-8")
    (root / "docs" / "README_EXCLUDED.json").write_text(json.dumps(EXCLUDED), encoding="utf-8")
    (root / "site-checks.json").write_text(json.dumps(site_checks()), encoding="utf-8")


# ------------------------------------------------------------------ execução

def run(command: list[str], env: dict[str, str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, capture_output=True, text=True, env={**CLEAN_ENV, **env}, check=False, timeout=600)


def compare(label: str, python: str | None, rust: str | None, failures: list[str]) -> None:
    if python == rust:
        print(f"  ok   {label}")
        return
    if python is None or rust is None:
        failures.append(f"{label}: python {'—' if python is None else 'gravou'} / rust {'—' if rust is None else 'gravou'}")
    else:
        lines = list(zip(python.splitlines(), rust.splitlines()))
        at = next((n for n, (a, b) in enumerate(lines) if a != b), len(lines))
        failures.append(f"{label}: linha {at + 1}\n    python: {python.splitlines()[at:at + 1]}\n    rust:   {rust.splitlines()[at:at + 1]}")
    print(f"  FAIL {label}")


def read(path: Path) -> str | None:
    return path.read_text(encoding="utf-8") if path.exists() else None


def catalog_parity(binary: str, fake: FakeGitHub, work: Path, failures: list[str]) -> None:
    for tag, token in (("com token", "inventory-token"), ("sem token", None)):
        py_root, rs_root = work / f"catalog-py-{tag}", work / f"catalog-rs-{tag}"
        build_root(py_root)
        build_root(rs_root)
        env = {"GITHUB_API_URL": fake.url, "GITHUB_TOKEN": "actions-token"}
        if token:
            env["PROFILE_GITHUB_TOKEN"] = token
        py_out = work / f"catalog-py-{tag}.json"
        python = run([sys.executable, str(ROOT / "scripts" / "update_profile.py"), "--root", str(py_root),
                      "--site-checks-fixture", str(py_root / "site-checks.json"), "--now", NOW,
                      "--catalog-out", str(py_out)], env)
        rust = run([binary, "catalog", "build", "--root", str(rs_root),
                    "--site-checks-fixture", str(rs_root / "site-checks.json"), "--now", NOW], env)
        if python.returncode != 0 or rust.returncode != 0:
            failures.append(f"catálogo {tag}: python {python.returncode} ({python.stderr.strip()[-300:]}) / "
                            f"rust {rust.returncode} ({rust.stderr.strip()[-300:]})")
            continue
        compare(f"catálogo {tag}", read(py_out), rust.stdout, failures)


REPO_STATES: dict[str, Any] = {}


def monitor_routes(fake: FakeGitHub, step: int) -> None:
    """Estado do GitHub na varredura `step` (0–4)."""
    fake.routes.clear()
    listed = [r for r in OWNER_REPOS if not (step >= 2 and r["name"] == "proj-033")]
    if step >= 2:
        listed.append({**owner_repo(150), "name": "novo-repo", "full_name": f"{OWNER}/novo-repo"})
    for path, items in pages(listed, lambda p: f"/users/{OWNER}/repos?type=owner&per_page=100&page={p}").items():
        fake.get(path, items)
    from urllib.parse import quote
    for repo in listed:
        if repo["private"] or repo["fork"]:
            continue
        name, branch = repo["name"], repo["default_branch"] or "main"
        path = f"/repos/{OWNER}/{quote(name)}/commits?sha={quote(branch)}&per_page=1"
        seed = repo["id"]
        version = 0
        if step >= 1 and seed % 3 == 0:
            version = 1
        if step >= 2 and seed % 3 == 0 and seed % 2 == 0:
            version = 2 if step < 4 else 3
        if step == 4 and seed % 5 == 1:
            version = 4
        entry = [{"sha": sha(seed, version), "html_url": f"https://github.com/{OWNER}/{name}/commit/{sha(seed, version)}",
                  "commit": {"committer": {"date": ago(seed % 30)}, "message": f"feat: {name} v{version}\n\ncorpo"}}]
        if name == "proj-040":
            fake.get(path, {"message": "Git Repository is empty."}, status=409)
        elif name == "proj-041":
            fake.get(path, [])
        elif name == "proj-042" and step in (1, 3):
            fake.get(path, responses=[Response(503, {}, {"Retry-After": "0"}), Response(200, entry)])
        elif name == "proj-043" and step == 4:
            fake.get(path, responses=[Response(403, {}, {"X-RateLimit-Remaining": "0", "Retry-After": "0"}), Response(200, entry)])
        elif name == "proj-044" and step >= 2:
            fake.get(path, {"message": "Server Error"}, status=500)
        elif name == "proj-045":
            fake.get(path, [None])
        else:
            fake.get(path, entry)
        for old in range(version):
            compare_path = f"/repos/{OWNER}/{quote(name)}/compare/{sha(seed, old)}...{sha(seed, version)}"
            if name == "proj-048":
                fake.get(compare_path, {"message": "Not Found"}, status=404)
            else:
                fake.get(compare_path, {"ahead_by": (seed % 7) + version - old, "status": "ahead"})


def monitor_parity(binary: str, fake: FakeGitHub, work: Path, failures: list[str]) -> None:
    py_root, rs_root = work / "monitor-py", work / "monitor-rs"
    for root in (py_root, rs_root):
        (root / "docs").mkdir(parents=True)
    env = {"GITHUB_API_URL": fake.url, "GH_USER": OWNER, "PROFILE_REPOSITORY_NAME": OWNER, "GITHUB_TOKEN": "actions-token"}
    for step in range(5):
        now = (NOW_DT + timedelta(hours=step)).strftime("%Y-%m-%dT%H:%M:%SZ")
        # Na varredura 3 nada muda (as rotas são as da 2, sem a falha passageira): no-op.
        monitor_routes(fake, 2 if step == 3 else step)
        python = run([sys.executable, str(ROOT / ".github" / "scripts" / "ecosystem_watch.py"),
                      "--root", str(py_root), "--now", now], env)
        monitor_routes(fake, 2 if step == 3 else step)
        rust = run([binary, "sync", "commits", "--root", str(rs_root), "--now", now, "--write", "--no-db"], env)
        label = f"monitor {step + 1}/5"
        if python.returncode != 0 or rust.returncode != 0:
            failures.append(f"{label}: python {python.returncode} ({python.stderr.strip()[-300:]}) / "
                            f"rust {rust.returncode} ({rust.stderr.strip()[-300:]})")
            continue
        compare(f"{label} saída", python.stdout, rust.stdout, failures)
        compare(f"{label} estado", read(py_root / "docs" / "ECOSYSTEM-COMMIT-STATE.json"),
                read(rs_root / "docs" / "ECOSYSTEM-COMMIT-STATE.json"), failures)
        compare(f"{label} relatório", read(py_root / "docs" / "ECOSYSTEM-COMMIT-MONITOR.md"),
                read(rs_root / "docs" / "ECOSYSTEM-COMMIT-MONITOR.md"), failures)
    state = json.loads(read(rs_root / "docs" / "ECOSYSTEM-COMMIT-STATE.json") or "{}")
    print(f"       contadores finais: {state.get('metrics')}")


def contributions_parity(binary: str, fake: FakeGitHub, work: Path, failures: list[str]) -> None:
    def respond(variables: dict[str, Any]) -> dict[str, Any]:
        seed = int(variables["from"][5:7]) + int(variables["from"][8:10])
        return {"data": {"user": {"contributionsCollection": {
            "contributionCalendar": {"totalContributions": seed * 9},
            "totalCommitContributions": seed * 5, "totalIssueContributions": seed % 4,
            "totalPullRequestContributions": seed % 6, "totalPullRequestReviewContributions": seed % 2,
            "totalRepositoryContributions": seed % 3, "restrictedContributionsCount": 50 - seed}}}}

    fake.graphql(respond)
    py_root, rs_root = work / "timeline-py", work / "timeline-rs"
    env = {"GITHUB_GRAPHQL_URL": f"{fake.url}/graphql", "PROFILE_README_TOKEN": "tok", "PROFILE_LOGIN": OWNER}
    python = run([sys.executable, str(ROOT / "scripts" / "update_contribution_timeline.py"),
                  "--root", str(py_root), "--now", NOW], env)
    rust = run([binary, "sync", "contributions", "--root", str(rs_root), "--now", NOW, "--write", "--no-db"], env)
    if python.returncode != 0 or rust.returncode != 0:
        failures.append(f"contribuições: python {python.returncode} / rust {rust.returncode}: {rust.stderr.strip()[-300:]}")
        return
    compare("contribuições saída", python.stdout, rust.stdout, failures)
    for name in ("contributions-timeline-data.json", "contributions-timeline.html"):
        compare(f"contribuições {name}", read(py_root / "docs" / "assets" / name),
                read(rs_root / "docs" / "assets" / name), failures)


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    binary = str(Path(sys.argv[1]).resolve())
    fake = FakeGitHub()
    for path, items in pages(ALL_REPOS, lambda p: f"/user/repos?affiliation=owner,collaborator,organization_member&per_page=100&page={p}").items():
        fake.get(path, items)
    for path, items in pages(PUBLIC_OWNER, lambda p: f"/users/{OWNER}/repos?type=owner&per_page=100&page={p}").items():
        fake.get(path, items)
    for n, repo in enumerate(ALL_REPOS):
        if n % 10 == 3:
            fake.get(f"/repos/{repo['full_name']}/languages", {"message": "Not Found"}, status=404)
        else:
            fake.get(f"/repos/{repo['full_name']}/languages", {"Python": 1000 + n, "Rust": n})
    failures: list[str] = []
    with fake, tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        print(f"GitHub simulado: {len(ALL_REPOS)} repositórios ({len(PUBLIC_OWNER)} públicos do dono)")
        catalog_parity(binary, fake, work, failures)
        monitor_parity(binary, fake, work, failures)
        contributions_parity(binary, fake, work, failures)
    if failures:
        print(f"\n{len(failures)} divergência(s):", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("\nPython = Rust em todos os cenários.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
