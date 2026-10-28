#!/usr/bin/env python3
"""Build a deterministic, public-only static portfolio artifact."""
from __future__ import annotations

import argparse
import ipaddress
import json
import re
import shutil
import sys
import tempfile
from pathlib import Path
from urllib.parse import urlsplit

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
        # Accessing .port validates malformed port values such as :not-a-port.
        _ = parsed.port
        if not (parsed.scheme == "https" and host
                and parsed.username is None and parsed.password is None
                and host not in {"localhost", "localhost.localdomain"}
                and not host.endswith(".local")):
            return False
        try:
            address = ipaddress.ip_address(host)
        except ValueError:
            return True
        return address.is_global
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


def build(root: Path, out: Path) -> int:
    catalog_path = root / "docs" / "project-catalog.json"
    if not catalog_path.is_file():
        raise BuildError(f"catalog not found: {catalog_path}")
    try:
        catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise BuildError(f"cannot read valid UTF-8 catalog JSON: {exc}") from exc
    projects = public_projects(catalog)
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
