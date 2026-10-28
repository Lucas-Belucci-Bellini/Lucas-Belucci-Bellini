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
ROOT_FIELDS = {"schema", "note", "counts", "categories", "projects"}
REQUIRED_PROJECT_FIELDS = ("name", "slug", "description", "category", "category_label", "status", "github", "private", "github_visible")


class BuildError(ValueError):
    """Raised when source data or build boundaries violate the public contract."""


def safe_https_url(value: object) -> bool:
    """Accept absolute HTTPS URLs without credentials or local/private destinations."""
    if not isinstance(value, str) or not value or value != value.strip() or any(c.isspace() for c in value):
        return False
    try:
        parsed = urlsplit(value)
        host = (parsed.hostname or "").lower().rstrip(".")
        if parsed.scheme != "https" or not host or parsed.port not in (None, 443):
            return False
        if parsed.username is not None or parsed.password is not None or parsed.fragment:
            return False
        if host in {"localhost", "localhost.localdomain"} or host.endswith((".local", ".internal", ".localhost")):
            return False
        try:
            address = ipaddress.ip_address(host)
            if not address.is_global:
                return False
        except ValueError:
            if "." not in host or host.startswith(".") or ".." in host:
                return False
            if not re.fullmatch(r"[a-z0-9.-]+", host):
                return False
        return True
    except ValueError:
        return False


def safe_github_url(value: object) -> bool:
    """Accept only HTTPS URLs pointing to one GitHub owner/repository path."""
    if not safe_https_url(value):
        return False
    parsed = urlsplit(str(value))
    host = (parsed.hostname or "").lower()
    parts = [part for part in parsed.path.split("/") if part]
    return (
        host in {"github.com", "www.github.com"}
        and len(parts) == 2
        and all(part not in {".", ".."} and "." not in part for part in parts)
        and not parsed.query
    )


def public_projects(catalog: object) -> list[dict[str, object]]:
    if not isinstance(catalog, dict):
        raise BuildError("catalog root must be a JSON object")
    if catalog.get("schema") != SCHEMA:
        raise BuildError(f"unsupported catalog schema; expected {SCHEMA!r}")
    if set(catalog) - ROOT_FIELDS:
        raise BuildError("catalog contains unsupported root fields")
    categories = catalog.get("categories")
    projects = catalog.get("projects")
    if not isinstance(categories, list) or not categories or not all(isinstance(c, str) and c.strip() for c in categories):
        raise BuildError("categories must be a non-empty array of non-empty strings")
    if len(categories) != len(set(categories)):
        raise BuildError("categories must not contain duplicates")
    if not isinstance(projects, list) or not projects:
        raise BuildError("projects must be a non-empty array")

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
        missing = [field for field in REQUIRED_PROJECT_FIELDS if field not in project]
        if missing:
            raise BuildError(f"{where} missing required fields: {', '.join(missing)}")
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
        github = project.get("github")
        if not safe_github_url(github):
            continue
        item = {key: project[key] for key in PUBLIC_FIELDS if key in project and key != "website"}
        website = project.get("website")
        # Website links require an explicit verification marker and a safe HTTPS URL.
        if project.get("website_status") == "verified" and safe_https_url(website):
            item["website"] = website
        result.append(item)
    result.sort(key=lambda item: (str(item["name"]).casefold(), str(item["slug"])))
    if not result:
        raise BuildError("no valid public projects remain after filtering")
    return result


def _assert_safe_paths(root: Path, out: Path) -> tuple[Path, Path]:
    root = root.resolve(strict=True)
    expected = root / "dist"
    # Compare lexical path before resolving to prevent symlink aliases escaping dist/.
    requested = out if out.is_absolute() else Path.cwd() / out
    if requested.absolute() != expected.absolute():
        raise BuildError("output must be exactly the repository dist/ directory")
    if expected.is_symlink():
        raise BuildError("dist/ must not be a symlink")
    return root, expected


def build(root: Path, out: Path) -> int:
    root, out = _assert_safe_paths(root, out)
    catalog_path = root / "docs" / "project-catalog.json"
    if catalog_path.is_symlink() or not catalog_path.is_file():
        raise BuildError("catalog must be a regular file at docs/project-catalog.json")
    try:
        catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise BuildError(f"cannot read valid UTF-8 catalog JSON: {exc}") from exc
    projects = public_projects(catalog)
    for relative in STATIC_FILES:
        source = root / relative
        if source.is_symlink() or not source.is_file():
            raise BuildError(f"required static file missing or unsafe: {relative}")

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
        expected_files = {".nojekyll", *STATIC_FILES, "data/projects.json"}
        actual_files = {p.relative_to(stage).as_posix() for p in stage.rglob("*") if p.is_file()}
        if actual_files != expected_files:
            raise BuildError("staged artifact does not match the static file allowlist")
        if out.exists():
            if not out.is_dir() or out.is_symlink():
                raise BuildError("dist/ exists but is not a safe directory")
            shutil.rmtree(out)
        stage.rename(out)
    except Exception:
        shutil.rmtree(stage, ignore_errors=True)
        raise
    return len(projects)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="repository root")
    parser.add_argument("--out", type=Path, default=ROOT / "dist", help="must be exactly <root>/dist")
    args = parser.parse_args()
    try:
        count = build(args.root, args.out)
    except (BuildError, OSError) as exc:
        print(f"portfolio build failed: {exc}", file=sys.stderr)
        return 2
    print(f"portfolio build succeeded: {count} public projects -> {args.out.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
