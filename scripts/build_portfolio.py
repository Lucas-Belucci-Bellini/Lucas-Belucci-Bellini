#!/usr/bin/env python3
"""Build a minimal public-only static portfolio artifact.

Only public project metadata is copied into dist/data/projects.json. The rest of
the repository (including source manifests, automation and private project
metadata) is not part of the deploy artifact.
"""
from __future__ import annotations

import json
import shutil
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]
DIST = ROOT / "dist"
CATALOG = ROOT / "docs" / "project-catalog.json"
PUBLIC_FIELDS = (
    "name",
    "description",
    "category",
    "category_label",
    "status",
    "github",
    "github_visible",
    "website_status",
    "website",
)


def safe_https_url(value: object) -> bool:
    if not isinstance(value, str):
        return False
    parsed = urlparse(value)
    return parsed.scheme == "https" and bool(parsed.netloc)


def main() -> int:
    if not CATALOG.is_file():
        raise SystemExit(f"Catalog not found: {CATALOG}")

    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    if not isinstance(catalog.get("projects"), list):
        raise SystemExit("Invalid catalog: projects must be an array")

    public_projects = []
    for project in catalog["projects"]:
        if not isinstance(project, dict) or project.get("private") is True:
            continue
        if project.get("github_visible") is False or not safe_https_url(project.get("github")):
            continue
        public_projects.append({key: project[key] for key in PUBLIC_FIELDS if key in project})

    if not public_projects:
        raise SystemExit("Refusing to build an empty public project catalog")

    if DIST.exists():
        shutil.rmtree(DIST)
    (DIST / "assets").mkdir(parents=True)
    (DIST / "data").mkdir(parents=True)

    shutil.copy2(ROOT / "index.html", DIST / "index.html")
    for asset in ("site.css", "site.js", "favicon.svg"):
        shutil.copy2(ROOT / "assets" / asset, DIST / "assets" / asset)

    (DIST / "data" / "projects.json").write_text(
        json.dumps(
            {
                "schema": "lucas-belucci-bellini/public-portfolio@1",
                "project_count": len(public_projects),
                "projects": public_projects,
            },
            ensure_ascii=False,
            indent=2,
        ) + "\n",
        encoding="utf-8",
    )
    (DIST / ".nojekyll").write_text("", encoding="utf-8")
    print(f"Portfolio built: {len(public_projects)} public projects -> {DIST}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
