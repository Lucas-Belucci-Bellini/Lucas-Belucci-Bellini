#!/usr/bin/env python3
"""Critérios de saída das Fases 3 e 4: Python = Rust, contra o mesmo GitHub.

Sobe um GitHub simulado (tests/fake_github.py) com um ecossistema sintético —
inventário paginado, privados, forks, arquivados, colaboração, exclusões,
homepages boas e ruins, descrições com `|`, `\\`, acentos e U+001F — e roda os
dois programas contra ele:

    catálogo       update_profile.py --catalog-out   ×  profile-core catalog build
                   (com PROFILE_GITHUB_TOKEN e sem ele)
    README         update_profile.py --out-dir       ×  profile-core render readme --out-dir
                   (README, profile-snapshot.svg, catálogo e saída padrão, com e sem
                   token; com token também --write na raiz, como no workflow)
    monitor        ecosystem_watch.py --root         ×  profile-core sync commits --write --no-db
                   (5 varreduras encadeadas: commits novos, comparação que falha,
                   409, 503 que passa na segunda tentativa, rate limit, repositório
                   novo e sumido, varredura sem mudança)
    contribuições  update_contribution_timeline.py   ×  profile-core sync contributions --write --no-db
    assets         lang_stats.py / profile_cards.py   ×  profile-core render lang-stats / cards
                   (listagem com e sem token, token recusado, forks, árvores git
                   truncadas, vazias, que falham e 429 que passa; segunda rodada
                   sem mudança; tracebacks; GraphQL com e sem erro) — saída,
                   SVGs, código de saída e a sequência de chamadas à API
    pipeline       os três scripts em sequência      ×  profile-core render all --out-dir / --write
                   (Fase 5: mesmos arquivos, mesma saída e mesmas chamadas; e o
                   --check: publicação em dia sai 0, carimbo diferente não conta,
                   bloco editado à mão e SVG apagado saem 1)

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
    # Bytes, não read_text: as quebras universais esconderiam um `\r` a mais.
    return path.read_bytes().decode("utf-8") if path.exists() else None


PROFILE_OUTPUTS = {"README.md": "README.md", "profile-snapshot.svg": "assets/profile-snapshot.svg",
                   "project-catalog.json": "docs/project-catalog.json"}


def profile_parity(binary: str, fake: FakeGitHub, work: Path, failures: list[str]) -> None:
    """Catálogo e README inteiro: mesmas leituras do GitHub, mesmos bytes."""
    for tag, token in (("com token", "inventory-token"), ("sem token", None)):
        py_root, rs_root = work / f"profile-py-{tag}", work / f"profile-rs-{tag}"
        build_root(py_root)
        build_root(rs_root)
        env = {"GITHUB_API_URL": fake.url, "GITHUB_TOKEN": "actions-token"}
        if token:
            env["PROFILE_GITHUB_TOKEN"] = token
        py_catalog, py_out, rs_out = work / f"catalog-py-{tag}.json", work / f"out-py-{tag}", work / f"out-rs-{tag}"
        python = run([sys.executable, str(ROOT / "scripts" / "update_profile.py"), "--root", str(py_root),
                      "--site-checks-fixture", str(py_root / "site-checks.json"), "--now", NOW,
                      "--catalog-out", str(py_catalog), "--out-dir", str(py_out)], env)
        catalog = run([binary, "catalog", "build", "--root", str(rs_root),
                       "--site-checks-fixture", str(rs_root / "site-checks.json"), "--now", NOW], env)
        rust = run([binary, "render", "readme", "--root", str(rs_root),
                    "--site-checks-fixture", str(rs_root / "site-checks.json"), "--now", NOW,
                    "--out-dir", str(rs_out)], env)
        if python.returncode != 0 or catalog.returncode != 0 or rust.returncode != 0:
            failures.append(f"perfil {tag}: python {python.returncode} ({python.stderr.strip()[-300:]}) / "
                            f"catalog {catalog.returncode} / render {rust.returncode} ({rust.stderr.strip()[-300:]})")
            continue
        compare(f"catálogo {tag}", read(py_catalog), catalog.stdout, failures)
        compare(f"README {tag}: saída", python.stdout, rust.stdout, failures)
        for name in PROFILE_OUTPUTS:
            compare(f"README {tag}: {name}", read(py_out / name), read(rs_out / name), failures)
        if not token:
            continue
        # O que o workflow faz: --write na raiz (exige o token).
        python = run([sys.executable, str(ROOT / "scripts" / "update_profile.py"), "--root", str(py_root),
                      "--site-checks-fixture", str(py_root / "site-checks.json"), "--now", NOW, "--write"], env)
        rust = run([binary, "render", "readme", "--root", str(rs_root),
                    "--site-checks-fixture", str(rs_root / "site-checks.json"), "--now", NOW, "--write"], env)
        if python.returncode != 0 or rust.returncode != 0:
            failures.append(f"perfil --write: python {python.returncode} / rust {rust.returncode} ({rust.stderr.strip()[-300:]})")
            continue
        compare("README --write: saída", python.stdout, rust.stdout, failures)
        for path in PROFILE_OUTPUTS.values():
            compare(f"README --write: {path}", read(py_root / path), read(rs_root / path), failures)


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


# ------------------------------------------------------------------ assets

LS_LISTING = "/user/repos?affiliation=owner&sort=pushed&per_page=100&page={}"
PUBLIC_LISTING = "/users/{}/repos?type=owner&per_page=100&page={}"
ASSET_FILES = ("lang-stats.svg", "profile-top-langs.svg", "profile-stats.svg", "profile-streak.svg",
               "profile-trophies.svg", "profile-projects.svg")


def tree_of(i: int) -> dict[str, Any]:
    paths = ["src/main.rs", "README.md", "lib/util.rs", "img/Logo.PNG", "Makefile", ".gitignore", "data/tabela.csv",
             f"src/f{i}.ts", "estranho.a-b", "arquivo.ÇÃO", "docs/manual.pdf", "dist/pacote.tar.gz",
             "3d/modelo.glb", f"fontes/f{i % 3}.woff2", "x.abcdefghijklm"]
    nodes = [{"path": path, "type": "blob"} for path in paths[: 4 + i % len(paths)]]
    # Diretório e submódulo com cara de extensão: só "blob" conta.
    nodes += [{"path": "src", "type": "tree"}, {"path": "conf.d", "type": "tree"}, {"path": "vendor/lib.js", "type": "commit"}]
    return {"sha": sha(i), "tree": nodes, "truncated": i % 4 == 1}


def assets_routes(fake: FakeGitHub, *, listing: list[dict[str, Any]], user_repos: str = "ok",
                  flaky: bool = False, bad_language: bool = False) -> None:
    """GitHub da análise de linguagens; refeito antes de cada programa (as filas de resposta se gastam)."""
    fake.routes.clear()
    if user_repos == "ok":
        for path, items in pages(listing, LS_LISTING.format).items():
            fake.get(path, items)
    elif user_repos == "403":
        fake.get(LS_LISTING.format(1), {"message": "Resource not accessible by integration"}, status=403)
    public = [r for r in listing if not r["private"]]
    for path, items in pages(public, lambda n: PUBLIC_LISTING.format(OWNER, n)).items():
        fake.get(path, items)
    languages = [
        lambda n: {"Python": 1000 + n, "Rust": n},
        lambda n: {"C#": 5000 * n, "PLpgSQL": 7},
        lambda n: {"JavaScript": 2_000_000 + n, "HTML": 3000, "CSS": 1024, "Shell": 12, "Batchfile": 1},
        # Odin e Zig empatam no total: a ordem é a de chegada (sorted estável).
        lambda n: {"Jupyter Notebook": 9, "Odin": 1_300_000_000, "Zig": 1_300_000_000},
        lambda n: {},
    ]
    for n, repo in enumerate(listing):
        full, branch = repo["full_name"], repo["default_branch"]
        if bad_language and n == 0:
            fake.get(f"/repos/{full}/languages", {"Python": "não é número"})
        elif n % 10 == 7:
            # Falha tratada: aviso, repositório com falha, e não conta como "sem linguagem".
            fake.get(f"/repos/{full}/languages", {"message": "Server Error"}, status=500)
        elif n % 10 != 3:
            fake.get(f"/repos/{full}/languages", languages[n % 5](n))
        tree_path = f"/repos/{full}/git/trees/{branch}?recursive=1"
        i = repo["id"] - 10_000
        if i % 7 == 2:
            fake.get(tree_path, {"message": "Git Repository is empty."}, status=409)
        elif i % 7 == 3:
            fake.get(tree_path, {"message": "Server Error"}, status=500)
        elif i % 7 == 5 and flaky and i < 20:
            fake.get(tree_path, responses=[Response(429, {"message": "secondary rate limit"}), Response(200, tree_of(i))])
        elif i % 7 != 0:
            fake.get(tree_path, tree_of(i))


def cards_handler(case: str) -> Callable[[dict[str, Any]], Any]:
    def respond(variables: dict[str, Any]) -> Any:
        if case == "502":
            return Response(502, {"message": "Bad Gateway"})
        if case == "erros":
            return {"errors": [{"message": "Something went wrong"}, {"type": "SEM_MENSAGEM"}]}
        window = f"{variables['from']} → {variables['to']} · {variables['login']}"
        return {"data": {"user": {
            "login": OWNER, "name": None if case == "sem nome" else f"Nome <{window}> & 'cia'",
            "contributionsCollection": {
                "totalCommitContributions": 987, "totalIssueContributions": 12,
                "totalPullRequestContributions": 34, "totalPullRequestReviewContributions": 5,
                "totalRepositoryContributions": 7, "restrictedContributionsCount": 1200,
                "contributionCalendar": {"totalContributions": 1245}},
            "repositories": {"totalCount": 98}}}}
    return respond


def assets_parity(binary: str, fake: FakeGitHub, work: Path, failures: list[str]) -> None:
    base = [r for r in OWNER_REPOS if " " not in str(r["default_branch"])]
    accented = [{**owner_repo(0), "default_branch": "função"}]
    scenarios = [
        ("lang-stats com token", "lang-stats", {"GITHUB_TOKEN": "tok"}, dict(listing=base, flaky=True), None),
        ("lang-stats token recusado (cai no público)", "lang-stats", {"GITHUB_TOKEN": "tok"},
         dict(listing=base, user_repos="403"), None),
        ("lang-stats sem token", "lang-stats", {}, dict(listing=base), None),
        ("lang-stats com forks e sem árvores", "lang-stats",
         {"GITHUB_TOKEN": "tok", "INCLUDE_FORKS": "1", "SEM_ARQUIVOS": "1"}, dict(listing=base), None),
        ("lang-stats segunda rodada: só o carimbo mudaria", "lang-stats", {"GITHUB_TOKEN": "tok"},
         dict(listing=base), "lang-stats com token"),
        ("lang-stats de quem não tem repositório", "lang-stats", {"GH_USER": "ninguem"}, dict(listing=[]), None),
        ("lang-stats com branch acentuado (traceback)", "lang-stats", {"GITHUB_TOKEN": "tok"},
         dict(listing=accented), None),
        ("lang-stats com linguagem que não é número (traceback)", "lang-stats", {"GITHUB_TOKEN": "tok"},
         dict(listing=base[:3], bad_language=True), None),
        ("cards", "cards", {"GITHUB_TOKEN": "tok"}, "ok", None),
        ("cards sem nome e com fuso", "cards", {"GITHUB_TOKEN": "tok", "NOW": "2026-01-01T01:00:00+05:00"},
         "sem nome", None),
        ("cards sem token", "cards", {}, "ok", None),
        ("cards com erros do GraphQL", "cards", {"GITHUB_TOKEN": "tok"}, "erros", None),
        ("cards com HTTP 502", "cards", {"GITHUB_TOKEN": "tok"}, "502", None),
    ]
    scripts = {"lang-stats": ROOT / ".github" / "scripts" / "lang_stats.py",
               "cards": ROOT / ".github" / "scripts" / "profile_cards.py"}
    for label, command, extra, routes, reuse in scenarios:
        env = {"GITHUB_API_URL": fake.url, "GITHUB_GRAPHQL_URL": f"{fake.url}/graphql", "GH_USER": OWNER,
               **{k: v for k, v in extra.items() if k != "NOW"}}
        now = extra.get("NOW", NOW)
        if reuse:
            now = "2026-09-26T08:00:00Z"
        roots = {side: work / f"assets-{side}-{reuse or label}" for side in ("py", "rs")}
        results, logs = {}, {}
        for side in ("py", "rs"):
            if command == "lang-stats":
                assets_routes(fake, **routes)
            else:
                fake.graphql(cards_handler(routes))
            fake.requests.clear()
            if side == "py":
                cmd = [sys.executable, str(scripts[command]), "--root", str(roots[side]), "--now", now]
            else:
                cmd = [binary, "render", command, "--root", str(roots[side]), "--now", now]
            results[side] = run(cmd, env)
            logs[side] = list(fake.requests)
        python, rust = results["py"], results["rs"]
        if python.returncode != rust.returncode:
            failures.append(f"{label}: python saiu com {python.returncode}, rust com {rust.returncode}\n"
                            f"    python: {python.stderr.strip()[-300:]}\n    rust: {rust.stderr.strip()[-300:]}")
            print(f"  FAIL {label} (código)")
            continue
        print(f"  ok   {label}: código {python.returncode}")
        compare(f"{label}: saída", python.stdout, rust.stdout, failures)
        compare(f"{label}: chamadas à API", "\n".join(map(str, logs["py"])), "\n".join(map(str, logs["rs"])), failures)
        # O cenário exercita o que diz? (Python = Rust não basta se os dois pularem o caminho.)
        paths = [path for _, path, _ in logs["py"]]
        expectations = {
            "lang-stats com token": ("svg_changed=True" in python.stdout
                                     and len(paths) != len(set(paths)), "grava e repete a árvore que deu 429"),
            "lang-stats token recusado (cai no público)": (
                LS_LISTING.format(1) in paths and PUBLIC_LISTING.format(OWNER, 1) in paths, "tenta e cai no público"),
            "lang-stats segunda rodada: só o carimbo mudaria": (
                "svg_changed=False top_langs_changed=False" in python.stdout, "não regrava"),
            "lang-stats com forks e sem árvores": (
                not any("/git/trees/" in path for path in paths), "não lê árvore"),
        }
        if label in expectations and not expectations[label][0]:
            failures.append(f"{label}: o cenário não exercitou o caminho ({expectations[label][1]})")
        for name in ASSET_FILES:
            compare(f"{label}: {name}", read(roots["py"] / "assets" / name), read(roots["rs"] / "assets" / name),
                    failures)


PROFILE_LISTING = "/user/repos?affiliation=owner,collaborator,organization_member&per_page=100&page={}"


def profile_routes(fake: FakeGitHub, *, sane_languages: bool = False) -> None:
    """O GitHub do update_profile.py: inventário com e sem token e as linguagens."""
    for path, items in pages(ALL_REPOS, PROFILE_LISTING.format).items():
        fake.get(path, items)
    for path, items in pages(PUBLIC_OWNER, lambda p: PUBLIC_LISTING.format(OWNER, p)).items():
        fake.get(path, items)
    languages = [
        lambda n: {"Python": 1000 + n, "Rust": n},
        lambda n: {"C#": 5000 * n, "PLpgSQL": 7},
        lambda n: {"JavaScript": 2_000_000 + n, "HTML": 3000, "CSS": 1024, "Shell": 12, "Batchfile": 1, "Dockerfile": 2},
        lambda n: {"Jupyter Notebook": 9},
        # O mapa inteiro vira vazio no update_profile.py; o lang_stats.py quebraria (A26).
        (lambda n: {"Go": 4096 + n}) if sane_languages else (lambda n: {"Python": "não é número"}),
    ]
    for n, repo in enumerate(ALL_REPOS):
        if n % 10 == 3:
            fake.get(f"/repos/{repo['full_name']}/languages", {"message": "Not Found"}, status=404)
        else:
            fake.get(f"/repos/{repo['full_name']}/languages", languages[n % 5](n))


def pipeline_routes(fake: FakeGitHub) -> None:
    """Um GitHub só para os três programas: o do README, as árvores do lang-stats e o GraphQL dos cards."""
    fake.routes.clear()
    profile_routes(fake, sane_languages=True)
    listing = [r for r in OWNER_REPOS if " " not in str(r["default_branch"])]
    for path, items in pages(listing, LS_LISTING.format).items():
        fake.get(path, items)
    for repo in listing:
        i = repo["id"] - 10_000
        tree_path = f"/repos/{repo['full_name']}/git/trees/{repo['default_branch']}?recursive=1"
        if i % 7 == 2:
            fake.get(tree_path, {"message": "Git Repository is empty."}, status=409)
        elif i % 7 != 0:
            fake.get(tree_path, tree_of(i))
    fake.graphql(cards_handler("ok"))


def pipeline_parity(binary: str, fake: FakeGitHub, work: Path, failures: list[str]) -> None:
    """`render all` = update_profile.py + lang_stats.py + profile_cards.py, e o --check."""
    pipeline_routes(fake)
    env = {"GITHUB_API_URL": fake.url, "GITHUB_GRAPHQL_URL": f"{fake.url}/graphql", "GH_USER": OWNER,
           "GITHUB_TOKEN": "tok", "PROFILE_GITHUB_TOKEN": "inventory-token"}
    checks = ["--site-checks-fixture"]

    def python_side(root: Path, *mode: str) -> tuple[str, int, list[Any]]:
        fake.requests.clear()
        out, code = "", 0
        for command in ([sys.executable, str(ROOT / "scripts" / "update_profile.py"), "--root", str(root),
                         *checks, str(root / "site-checks.json"), "--now", NOW, *mode],
                        [sys.executable, str(ROOT / ".github" / "scripts" / "lang_stats.py"), "--root", str(root),
                         "--now", NOW],
                        [sys.executable, str(ROOT / ".github" / "scripts" / "profile_cards.py"), "--root", str(root),
                         "--now", NOW]):
            result = run(command, env)
            out += result.stdout
            code = code or result.returncode
            if result.returncode != 0:
                failures.append(f"pipeline python: {command[1]} saiu com {result.returncode}: {result.stderr[-300:]}")
                break
        return out, code, list(fake.requests)

    def rust_side(root: Path, *mode: str) -> subprocess.CompletedProcess[str]:
        fake.requests.clear()
        return run([binary, "render", "all", "--root", str(root), *checks, str(root / "site-checks.json"),
                    "--now", NOW, *mode], env)

    # --write: o que o workflow consolidado faria.
    py_write, rs_write = work / "pipeline-py-write", work / "pipeline-rs-write"
    build_root(py_write)
    build_root(rs_write)
    py_stdout, py_code, _ = python_side(py_write, "--write")
    rust = rust_side(rs_write, "--write")
    if py_code != 0 or rust.returncode != 0:
        failures.append(f"pipeline --write: python {py_code} / rust {rust.returncode} ({rust.stderr.strip()[-300:]})")
        return
    compare("render all --write: saída", py_stdout, rust.stdout, failures)
    for path in [*PROFILE_OUTPUTS.values(), *(f"assets/{name}" for name in ASSET_FILES)]:
        compare(f"render all --write: {path}", read(py_write / path), read(rs_write / path), failures)

    # --out-dir: o modo sombra do pipeline inteiro, a partir do que está publicado
    # (o "mudou?" do lang-stats compara com os SVGs da raiz).
    py_root, rs_root, py_out, rs_out = (work / f"pipeline-{n}" for n in ("py", "rs", "py-out", "rs-out"))
    shutil.copytree(py_write, py_root)
    shutil.copytree(rs_write, rs_root)
    before = {path: read(rs_root / path) for path in [*PROFILE_OUTPUTS.values(), *(f"assets/{n}" for n in ASSET_FILES)]}
    py_stdout, py_code, py_calls = python_side(py_root, "--out-dir", str(py_out))
    rust = rust_side(rs_root, "--out-dir", str(rs_out))
    rs_calls = list(fake.requests)
    if py_code != 0 or rust.returncode != 0:
        failures.append(f"pipeline --out-dir: python {py_code} / rust {rust.returncode} ({rust.stderr.strip()[-300:]})")
        return
    if "svg_changed=False top_langs_changed=False" not in py_stdout:
        failures.append("pipeline --out-dir: o cenário não partiu dos SVGs publicados")
    compare("render all --out-dir: saída", py_stdout, rust.stdout, failures)
    compare("render all --out-dir: chamadas à API", "\n".join(map(str, py_calls)), "\n".join(map(str, rs_calls)),
            failures)
    for name, path in PROFILE_OUTPUTS.items():
        compare(f"render all --out-dir: {path}", read(py_out / name), read(rs_out / path), failures)
    for name in ASSET_FILES:
        compare(f"render all --out-dir: assets/{name}", read(py_root / "assets" / name),
                read(rs_out / "assets" / name), failures)
    if any(read(rs_root / path) != text for path, text in before.items()):
        failures.append("render all --out-dir tocou na raiz")

    # --check: publicação em dia sai 0; carimbo diferente não conta; o resto, sim.
    published = work / "pipeline-published"
    shutil.copytree(rs_write, published)
    stats = published / "assets" / "profile-stats.svg"
    stats.write_text(stats.read_text(encoding="utf-8").replace(NOW[:10], "2020-01-01"), encoding="utf-8")
    fresh = rust_side(published, "--check")
    ok = fresh.returncode == 0 and "nada mudaria" in fresh.stdout and "mudaria  " not in fresh.stdout
    print(f"  {'ok  ' if ok else 'FAIL'} render all --check: publicação em dia (carimbo antigo num card) sai 0")
    if not ok:
        failures.append(f"render all --check em dia: {fresh.returncode}\n{fresh.stdout}{fresh.stderr[-300:]}")
    readme = published / "README.md"
    text = readme.read_text(encoding="utf-8")
    readme.write_text(text.replace("<!-- PROFILE-DASHBOARD:START -->", "<!-- PROFILE-DASHBOARD:START -->\neditado à mão", 1),
                      encoding="utf-8")
    (published / "assets" / "profile-trophies.svg").unlink()
    stale = rust_side(published, "--check")
    changed = sorted(line.split()[1] for line in stale.stdout.splitlines() if line.strip().startswith("mudaria "))
    ok = stale.returncode == 1 and changed == ["README.md", "assets/profile-trophies.svg"]
    print(f"  {'ok  ' if ok else 'FAIL'} render all --check: bloco editado e SVG apagado saem 1 ({changed})")
    if not ok:
        failures.append(f"render all --check desatualizado: {stale.returncode} {changed}\n{stale.stdout}")
    readme_only = run([binary, "render", "readme", "--root", str(published), *checks,
                       str(published / "site-checks.json"), "--now", NOW, "--check"], env)
    changed = sorted(line.split()[1] for line in readme_only.stdout.splitlines() if line.strip().startswith("mudaria "))
    ok = readme_only.returncode == 1 and changed == ["README.md"]
    print(f"  {'ok  ' if ok else 'FAIL'} render readme --check: só o README ({changed})")
    if not ok:
        failures.append(f"render readme --check: {readme_only.returncode} {changed}\n{readme_only.stdout}")
    if read(readme) is None or "editado à mão" not in (read(readme) or ""):
        failures.append("--check gravou na raiz")


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    binary = str(Path(sys.argv[1]).resolve())
    fake = FakeGitHub()
    profile_routes(fake)
    failures: list[str] = []
    with fake, tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        print(f"GitHub simulado: {len(ALL_REPOS)} repositórios ({len(PUBLIC_OWNER)} públicos do dono)")
        # E2E_ONLY=assets (etc.) roda só uma parte — útil para testar mutações.
        only = os.environ.get("E2E_ONLY")
        for name, section in (("profile", profile_parity), ("monitor", monitor_parity),
                              ("contributions", contributions_parity), ("assets", assets_parity),
                              ("pipeline", pipeline_parity)):
            if not only or only == name:
                section(binary, fake, work, failures)
    if failures:
        print(f"\n{len(failures)} divergência(s):", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("\nPython = Rust em todos os cenários.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
