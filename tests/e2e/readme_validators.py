#!/usr/bin/env python3
"""Critério de saída da Fase 4: README do Rust = README do Python, e os
validadores do Python aprovam o README do Rust — sobre a árvore REAL do perfil.

O golden (tests/fixtures/profile) e o fixture de paridade usam READMEs
sintéticos. Aqui a entrada é o que está publicado: o README.md com todo o
desenho em volta dos marcadores, os manifestos docs/README_*.json e os assets.
O inventário é reconstruído do docs/project-catalog.json versionado (os
projetos com nome, descrição, visibilidade, status e site, e o resultado da
última verificação); as linguagens, da matriz LANGUAGE-STATS publicada — nada
disso sai da rede nem de arquivo privado.

Os dois geradores rodam com --write, cada um na sua cópia, e o teste exige:

  1. os mesmos bytes — README.md, assets/profile-snapshot.svg,
     docs/project-catalog.json e a saída padrão;
  2. os validadores verdes sobre a cópia do Rust: os do update-profile.yml
     (validate_dynamic_sections contra o README publicado, validate_exclusions,
     validate_project_links), o validate_restored_style e as conferências de
     validate_language_badges e validate_profile — estes dois sem a rede (o
     que eles checam online são sites de terceiros, não o gerador).

    python3 tests/e2e/readme_validators.py target/release/profile-core
"""
from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
OWNER = "Lucas-Belucci-Bellini"
NOW = "2026-09-25T12:00:00Z"
NOW_DT = datetime(2026, 9, 25, 12, tzinfo=timezone.utc)
CLEAN_ENV = {key: value for key, value in os.environ.items()
             if "TOKEN" not in key and not key.startswith("GITHUB_") and key != "DATABASE_URL"}
DISPLAY_TO_GITHUB = {"Batch": "Batchfile", "PL/pgSQL": "PLpgSQL"}
AGE_BY_STATUS = {"🟢 Active": 10, "🟡 In Development": 120, "🔵 Experimental": 500, "🟣 Academic": 200, "🔒 Private": 5}
OUTPUTS = ("README.md", "assets/profile-snapshot.svg", "docs/project-catalog.json")


def ago(days: int) -> str:
    return (NOW_DT - timedelta(days=days)).strftime("%Y-%m-%dT%H:%M:%SZ")


def copy_tree(target: Path) -> None:
    """O que o gerador e os validadores leem — e nada além disso."""
    target.mkdir(parents=True)
    shutil.copy2(ROOT / "README.md", target / "README.md")
    shutil.copytree(ROOT / "assets", target / "assets")
    shutil.copytree(ROOT / "scripts", target / "scripts", ignore=shutil.ignore_patterns("__pycache__"))
    (target / "docs").mkdir()
    for manifest in (ROOT / "docs").glob("*.json"):
        shutil.copy2(manifest, target / "docs" / manifest.name)
    (target / ".github" / "workflows").mkdir(parents=True)
    shutil.copy2(ROOT / ".github" / "workflows" / "update-profile.yml", target / ".github" / "workflows")


def inventory() -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """Repositórios e checagens a partir do catálogo versionado."""
    catalog = json.loads((ROOT / "docs" / "project-catalog.json").read_text(encoding="utf-8"))
    repos: list[dict[str, Any]] = []
    checks: dict[str, Any] = {}
    for n, project in enumerate(catalog["projects"]):
        status = project["status"]
        description = project["description"]
        declared = project.get("website_declared")
        repos.append({
            "id": 50_000 + n,
            "name": project["name"],
            "full_name": project["repository"],
            "owner": {"id": 1, "login": project["repository"].split("/")[0], "type": "User"},
            "description": None if description == "Descrição pública não informada." else description,
            "private": project["private"],
            "visibility": "private" if project["private"] else "public",
            "fork": status == "🔵 Experimental",
            "archived": status == "⚪ Archived",
            "homepage": declared if project.get("website_source") == "github_homepage" else None,
            "pushed_at": ago(AGE_BY_STATUS.get(status, 30) + n % 7),
            "size": 1000 + 97 * n,
        })
        if declared:
            checks[declared] = {
                "status": project["website_status"],
                "http_status": project.get("website_http_status", 0),
                "final_url": project.get("website_final_url") or project.get("website") or declared,
            }
    # Os sites do manifesto também precisam de resultado (a mais é permitido).
    sites = json.loads((ROOT / "docs" / "README_SITES.json").read_text(encoding="utf-8"))
    for value in sites.values():
        url = (value.get("website") if isinstance(value, dict) else value) or ""
        checks.setdefault(str(url).strip(), {"status": "unreachable", "http_status": 0})
    # Os excluídos entram no inventário: os dois geradores têm de tirá-los.
    excluded = json.loads((ROOT / "docs" / "README_EXCLUDED.json").read_text(encoding="utf-8"))
    for n, entry in enumerate(excluded.get("repositories", [])):
        full_name = entry if "/" in entry else f"{OWNER}/{entry}"
        repos.append({"id": 90_000 + n, "name": full_name.split("/")[-1], "full_name": full_name,
                      "owner": {"id": 1, "login": OWNER, "type": "User"}, "description": "Excluído.",
                      "private": False, "fork": False, "archived": False, "homepage": None,
                      "pushed_at": ago(1), "size": 10})
    return repos, checks


def languages(repos: list[dict[str, Any]]) -> dict[str, dict[str, int]]:
    """A matriz LANGUAGE-STATS publicada, espalhada pelos repositórios públicos."""
    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    block = readme.split("<!-- LANGUAGE-STATS:START -->")[1].split("<!-- LANGUAGE-STATS:END -->")[0]
    units = {"B": 1, "KB": 1024, "MB": 1024 * 1024}
    public = sorted((r for r in repos if not r["private"]), key=lambda r: r["name"].lower())
    result: dict[str, dict[str, int]] = {}
    rows = re.findall(r"^\| (\d+) \| \*\*(.+?)\*\* \| `([\d.]+) (B|KB|MB)` \| `[\d.]+%` \| (\d+) \|$", block, re.M)
    if len(rows) < 5:
        raise SystemExit("matriz de linguagens do README não reconhecida")
    for index, display, size, unit, count in rows:
        language = DISPLAY_TO_GITHUB.get(display, display)
        total, count = int(float(size) * units[unit]), int(count)
        for k in range(count):
            repo = public[(int(index) * 7 + k * 3) % len(public)]
            share = total // count + (total % count if k == 0 else 0)
            result.setdefault(repo["full_name"], {})[language] = share
    # Um privado com linguagem: PROJECT-MAP a mostra (A20), LANGUAGE-STATS não.
    private = next(r for r in repos if r["private"])
    result.setdefault(private["full_name"], {})["Rust"] = 4242
    return result


def run(command: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, capture_output=True, text=True, env=CLEAN_ENV, check=False, timeout=300)


def read(path: Path) -> str | None:
    return path.read_bytes().decode("utf-8") if path.exists() else None


OFFLINE_BADGES = """
import sys
sys.path.insert(0, sys.argv[1])
import validate_language_badges as v
readme = v.README.read_text(encoding="utf-8")
count, labels, _ = v.assert_language_badges(readme)
categories = v.assert_categories(readme)
v.assert_language_count_matches(readme, count)
print(f"language badges (offline): {count} badges, {len(categories)} categorias")
"""

OFFLINE_PROFILE = """
import sys
from urllib.error import URLError
sys.path.insert(0, sys.argv[1])
import validate_profile as v
def offline(*_args, **_kwargs):
    raise URLError("offline")
v.urlopen = offline
sys.exit(v.main())
"""


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    binary = str(Path(sys.argv[1]).resolve())
    failures: list[str] = []
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        repos, checks = inventory()
        (work / "repos.json").write_text(json.dumps(repos, ensure_ascii=False), encoding="utf-8")
        (work / "checks.json").write_text(json.dumps(checks, ensure_ascii=False), encoding="utf-8")
        (work / "languages").mkdir()
        for full_name, data in languages(repos).items():
            (work / "languages" / f"{full_name.replace('/', '__')}.json").write_text(json.dumps(data), encoding="utf-8")
        inputs = ["--input-repos", str(work / "repos.json"), "--languages-dir", str(work / "languages"),
                  "--site-checks-fixture", str(work / "checks.json"), "--now", NOW, "--write"]
        py_root, rs_root = work / "py", work / "rs"
        copy_tree(py_root)
        copy_tree(rs_root)
        print(f"árvore real: {len(repos)} repositórios no inventário reconstruído, {len(checks)} checagens")

        python = run([sys.executable, str(ROOT / "scripts" / "update_profile.py"), "--root", str(py_root), *inputs])
        rust = run([binary, "render", "readme", "--root", str(rs_root), *inputs])
        if python.returncode != 0 or rust.returncode != 0:
            print(f"python {python.returncode}: {python.stderr[-500:]}\nrust {rust.returncode}: {rust.stderr[-500:]}",
                  file=sys.stderr)
            return 1
        for label, a, b in [("saída padrão", python.stdout, rust.stdout),
                            *((path, read(py_root / path), read(rs_root / path)) for path in OUTPUTS)]:
            if a == b:
                print(f"  ok   {label}")
            else:
                print(f"  FAIL {label}")
                failures.append(f"{label}: Python e Rust diferem")
        if read(rs_root / "README.md") == read(ROOT / "README.md"):
            failures.append("o README não mudou: a reconstrução do inventário não exercitou o gerador")

        scripts = rs_root / "scripts"
        validators = [
            ("validate_dynamic_sections", [sys.executable, str(scripts / "validate_dynamic_sections.py"),
                                           "--before", str(ROOT / "README.md"), "--after", str(rs_root / "README.md")]),
            ("validate_exclusions", [sys.executable, str(scripts / "validate_exclusions.py")]),
            ("validate_project_links", [sys.executable, str(scripts / "validate_project_links.py")]),
            ("validate_restored_style", [sys.executable, str(scripts / "validate_restored_style.py")]),
            ("validate_language_badges (sem rede)", [sys.executable, "-c", OFFLINE_BADGES, str(scripts)]),
            ("validate_profile (sem rede)", [sys.executable, "-c", OFFLINE_PROFILE, str(scripts)]),
        ]
        for label, command in validators:
            result = run(command)
            if result.returncode == 0:
                print(f"  ok   {label}")
            else:
                print(f"  FAIL {label}")
                failures.append(f"{label}: {(result.stderr or result.stdout).strip()[-800:]}")
    if failures:
        print(f"\n{len(failures)} falha(s):", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("\nREADME do Rust = README do Python, e os validadores aprovam.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
