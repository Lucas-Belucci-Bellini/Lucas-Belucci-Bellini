#!/usr/bin/env python3
"""Build a deterministic, public-only static portfolio artifact."""
from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
import tempfile
from pathlib import Path
from urllib.parse import urlsplit
import unicodedata

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = "lucas-belucci-bellini/project-catalog@1"
PUBLIC_FIELDS = ("name", "slug", "description", "category", "category_label", "status", "github", "website")
SLUG_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
STATUSES = {"🟢 Active", "🟡 In Development", "🔵 Experimental", "🟣 Academic", "⚪ Archived"}
STATIC_FILES = ("index.html", "assets/site.css", "assets/site.js", "assets/favicon.svg")


class BuildError(ValueError):
    pass


def safe_https_url(value: object) -> bool:
    if not isinstance(value, str) or not value or value != value.strip() or any(c.isspace() for c in value):
        return False
    try:
        parsed = urlsplit(value)
        host = (parsed.hostname or "").lower()
        return (
            parsed.scheme == "https" and bool(host)
            and parsed.username is None and parsed.password is None
            and host not in {"localhost", "localhost.localdomain"}
            and not host.endswith(".local")
        )
    except ValueError:
        return False


def public_projects(catalog: object) -> list[dict[str, object]]:
    if not isinstance(catalog, dict):
        raise BuildError("catalog root must be a JSON object")
    if catalog.get("schema") != SCHEMA:
        raise BuildError(f"unsupported catalog schema; expected {SCHEMA!r}")
    categories = catalog.get("categories")
    projects = catalog.get("projects")
    if not isinstance(categories, list) or not all(isinstance(c, str) and c.strip() for c in categories):
        raise BuildError("categories must be an array of non-empty strings")
    if not isinstance(projects, list):
        raise BuildError("projects must be an array")

    allowed_categories = set(categories)
    seen: set[str] = set()
    result: list[dict[str, object]] = []
    for index, project in enumerate(projects):
        where = f"projects[{index}]"
        if not isinstance(project, dict):
            raise BuildError(f"{where} must be an object")
        if project.get("private") is True:
            continue
        if project.get("private") is not False:
            raise BuildError(f"{where}.private must be an explicit boolean")
        if project.get("github_visible") is not True:
            continue
        for field in ("name", "slug", "description", "category", "category_label", "status"):
            if not isinstance(project.get(field), str) or not project[field].strip():
                raise BuildError(f"{where}.{field} must be a non-empty string")
        slug = project["slug"]
        if not SLUG_RE.fullmatch(slug):
            raise BuildError(f"{where}.slug is not a safe slug")
        if slug in seen:
            raise BuildError(f"duplicate public slug: {slug!r}")
        seen.add(slug)
        if project["category"] not in allowed_categories:
            raise BuildError(f"{where}.category is not declared in categories")
        if project["status"] not in STATUSES:
            raise BuildError(f"{where}.status is not an allowed public status")
        if not safe_https_url(project.get("github")) or (urlsplit(str(project.get("github"))).hostname or "").lower() not in {"github.com", "www.github.com"}:
            # Invalid GitHub link means no safe primary project card.
            continue
        item = {key: project[key] for key in PUBLIC_FIELDS if key in project and key != "website"}
        website = project.get("website")
        if project.get("website_status") == "verified" and safe_https_url(website):
            item["website"] = website
        result.append(item)
    result.sort(key=lambda item: (str(item["name"]).casefold(), str(item["slug"])))
    if not result:
        raise BuildError("no valid public projects remain after filtering")
    return result


def normalize_evidence(value: str) -> str:
    value = unicodedata.normalize("NFKD", value.casefold())
    value = "".join(char for char in value if not unicodedata.combining(char))
    return re.sub(r"[^a-z0-9]+", " ", value).strip()


def parse_language_bytes(value: str) -> int:
    match = re.fullmatch(r"([0-9]+(?:\\.[0-9]+)?)\\s*(B|KB|MB|GB)", value.strip())
    if not match:
        raise BuildError(f"unsupported language size: {value!r}")
    amount = float(match.group(1))
    multiplier = {"B": 1, "KB": 1024, "MB": 1024**2, "GB": 1024**3}[match.group(2)]
    return round(amount * multiplier)


def build_stack_inventory(root: Path, projects: list[dict[str, object]]) -> dict[str, object]:
    archive_path = root / "docs" / "PROFILE-ARCHIVE.md"
    stack_path = root / "docs" / "README_STACK.json"
    if not archive_path.is_file() or not stack_path.is_file():
        raise BuildError("professional stack sources are missing")

    archive = archive_path.read_text(encoding="utf-8")
    stack = json.loads(stack_path.read_text(encoding="utf-8"))
    if not isinstance(stack, dict) or not isinstance(stack.get("tools"), list):
        raise BuildError("README_STACK.json must contain a tools array")

    start_marker = "<!-- LANGUAGE-STATS:START -->"
    end_marker = "<!-- LANGUAGE-STATS:END -->"
    start = archive.find(start_marker)
    end = archive.find(end_marker, start + len(start_marker))
    if start < 0 or end < 0:
        raise BuildError("public language statistics markers are missing")
    section = archive[start + len(start_marker):end]

    summary_match = re.search(
        r"\\*\\*(\\d+) linguagens\\*\\*\\s*·\\s*"
        r"\\*\\*(\\d+) repositórios públicos\\*\\*\\s*·\\s*"
        r"\\*\\*([0-9.]+ MB) de código detectado\\*\\*\\s*·\\s*"
        r"atualizado em \\x60([^\\x60]+)\\x60",
        section,
    )
    if not summary_match:
        raise BuildError("public language statistics summary is invalid")

    language_pattern = re.compile(
        r"^\\|\\s*(\\d+)\\s*\\|\\s*\\*\\*(.+?)\\*\\*\\s*"
        r"\\|\\s*\\x60([^\\x60]+)\\x60\\s*"
        r"\\|\\s*\\x60([0-9.]+)%\\x60\\s*\\|\\s*(\\d+)\\s*\\|$"
    )
    languages: list[dict[str, object]] = []
    for line in section.splitlines():
        match = language_pattern.match(line.strip())
        if not match:
            continue
        rank, name, size, share, repositories = match.groups()
        languages.append({
            "rank": int(rank),
            "name": name,
            "bytes": parse_language_bytes(size),
            "size": size,
            "share": float(share),
            "repositories": int(repositories),
        })

    expected_count = int(summary_match.group(1))
    if len(languages) != expected_count:
        raise BuildError(
            f"language matrix is incomplete: expected {expected_count}, found {len(languages)}"
        )

    public_projects_by_name = [
        (normalize_evidence(str(project["name"])), project)
        for project in projects
    ]
    tools: list[dict[str, object]] = []
    allowed_categories = {
        "Frameworks & Web",
        "Infraestrutura & DevOps",
        "IA & Conhecimento",
        "Hardware & Simulação",
    }
    for index, tool in enumerate(stack["tools"]):
        if not isinstance(tool, dict):
            raise BuildError(f"tools[{index}] must be an object")
        name = tool.get("name")
        category = tool.get("category")
        family = tool.get("family")
        evidence = tool.get("evidence")
        if not all(isinstance(value, str) and value.strip() for value in (name, category, family, evidence)):
            raise BuildError(f"tools[{index}] has missing professional metadata")
        if category not in allowed_categories:
            raise BuildError(f"tools[{index}] uses an unsupported category: {category!r}")

        evidence_text = normalize_evidence(evidence)
        evidence_projects: list[dict[str, str]] = []
        for normalized_name, project in public_projects_by_name:
            if normalized_name and normalized_name in evidence_text:
                item = {
                    "name": str(project["name"]),
                    "github": str(project["github"]),
                }
                website = project.get("website")
                if isinstance(website, str) and safe_https_url(website):
                    item["website"] = website
                evidence_projects.append(item)

        tools.append({
            "name": name,
            "category": category,
            "family": family,
            "evidence": evidence,
            "projects": evidence_projects,
        })

    return {
        "schema": "lucas-belucci-bellini/professional-stack@1",
        "summary": {
            "language_count": expected_count,
            "public_repository_count": int(summary_match.group(2)),
            "total_code": summary_match.group(3),
            "updated_at": summary_match.group(4),
        },
        "languages": languages,
        "tools": tools,
        "categories": [
            "Frameworks & Web",
            "Infraestrutura & DevOps",
            "IA & Conhecimento",
            "Hardware & Simulação",
        ],
        "methodology": {
            "languages": "Agregadas dos mapas públicos de linguagens do GitHub; peso em bytes e participação sobre o total detectado.",
            "tools": "Curadoria baseada em documentação pública, manifests e evidências registradas no manifesto do perfil.",
            "proficiency": "A presença de uma tecnologia indica evidência de uso/documentação, não uma certificação nem domínio equivalente.",
        },
    }


def build(root: Path, out: Path) -> int:
    catalog_path = root / "docs" / "project-catalog.json"
    if not catalog_path.is_file():
        raise BuildError(f"catalog not found: {catalog_path}")
    try:
        catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise BuildError(f"cannot read valid UTF-8 catalog JSON: {exc}") from exc
    projects = public_projects(catalog)
    stack_inventory = build_stack_inventory(root, projects)
    for relative in STATIC_FILES:
        if not (root / relative).is_file():
            raise BuildError(f"required static file missing: {relative}")

    out = out.resolve()
    root = root.resolve()
    if out == root or (root in out.parents and out != root / "dist"):
        raise BuildError("output inside the repository is restricted to the dist/ directory")
    out.parent.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=".portfolio-build-", dir=out.parent))
    try:
        for relative in STATIC_FILES:
            target = stage / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(root / relative, target)
        data_dir = stage / "data"
        data_dir.mkdir(parents=True, exist_ok=True)
        payload = {
            "schema": "lucas-belucci-bellini/public-portfolio@1",
            "project_count": len(projects),
            "projects": projects,
        }
        (data_dir / "projects.json").write_text(
            json.dumps(payload, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
            encoding="utf-8", newline="\n",
        )
        (stage / ".nojekyll").write_text("", encoding="utf-8", newline="\n")
        if out.exists():
            if not out.is_dir():
                raise BuildError(f"output path exists and is not a directory: {out}")
            shutil.rmtree(out)
        stage.rename(out)
    except Exception:
        shutil.rmtree(stage, ignore_errors=True)
        raise
    return len(projects)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="repository root")
    parser.add_argument("--out", type=Path, default=ROOT / "dist", help="output directory")
    args = parser.parse_args()
    try:
        count = build(args.root.resolve(), args.out)
    except (BuildError, OSError) as exc:
        print(f"portfolio build failed: {exc}", file=sys.stderr)
        return 2
    print(f"portfolio build succeeded: {count} public projects -> {args.out.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
