#!/usr/bin/env python3
"""Validate the canonical project catalog and website override manifest.

Run: python scripts/validate_project_catalog.py
This is an offline structural check; it does not claim that remote sites are live.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = "lucas-belucci-bellini/project-catalog@1"
SLUG = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
LIVE = {"verified", "unreachable", "invalid"}


def read_json(path: Path, errors: list[str]):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        errors.append(f"{path.relative_to(ROOT)}: cannot read valid UTF-8 JSON: {exc}")
        return None


def https_url(value: object) -> bool:
    if not isinstance(value, str) or not value or value != value.strip() or any(c.isspace() for c in value):
        return False
    try:
        parsed = urlsplit(value)
        host = (parsed.hostname or "").lower()
        return parsed.scheme == "https" and bool(host) and parsed.username is None and parsed.password is None and host not in {"localhost", "localhost.localdomain"} and not host.endswith(".local")
    except ValueError:
        return False


def validate(root: Path = ROOT) -> tuple[list[str], list[str], dict[str, int]]:
    global ROOT
    ROOT = root.resolve()
    errors: list[str] = []
    warnings: list[str] = []
    catalog = read_json(ROOT / "docs/project-catalog.json", errors)
    sites = read_json(ROOT / "docs/README_SITES.json", errors)
    counts = {"records": 0, "public_candidates": 0, "private": 0, "verified_websites": 0}

    if isinstance(catalog, dict):
        if catalog.get("schema") != SCHEMA:
            errors.append(f"catalog.schema must equal {SCHEMA!r}")
        categories = catalog.get("categories")
        projects = catalog.get("projects")
        if not isinstance(categories, list) or any(not isinstance(c, str) or not c.strip() for c in categories):
            errors.append("catalog.categories must be an array of non-empty strings")
            categories = []
        elif len(categories) != len(set(categories)):
            errors.append("catalog.categories contains duplicates")
        if not isinstance(projects, list):
            errors.append("catalog.projects must be an array")
            projects = []
        counts["records"] = len(projects)
        seen_slugs: set[str] = set()
        for i, project in enumerate(projects):
            where = f"catalog.projects[{i}]"
            if not isinstance(project, dict):
                errors.append(f"{where} must be an object")
                continue
            if not isinstance(project.get("repository"), str) or not re.fullmatch(r"[^/\\s]+/[^/\\s]+", project["repository"]):
                errors.append(f"{where}.repository must be owner/repository")
            private = project.get("private")
            if not isinstance(private, bool):
                errors.append(f"{where}.private must be an explicit boolean")
            elif private:
                counts["private"] += 1
            if project.get("github_visible") is True and private is False:
                counts["public_candidates"] += 1
            slug = project.get("slug")
            if not isinstance(slug, str) or not SLUG.fullmatch(slug):
                errors.append(f"{where}.slug is not a safe lowercase slug")
            elif slug in seen_slugs:
                errors.append(f"{where}.slug duplicates {slug!r}")
            else:
                seen_slugs.add(slug)
            if project.get("website_status") not in LIVE:
                errors.append(f"{where}.website_status must be one of {sorted(LIVE)}")
            website = project.get("website")
            if website:
                if not https_url(website):
                    errors.append(f"{where}.website must be a safe HTTPS URL")
                elif project.get("website_status") != "verified":
                    warnings.append(f"{where}: website exists but status is not verified; public builder must omit it")
            if project.get("website_status") == "verified" and not website:
                warnings.append(f"{where}: marked verified but no website URL is present")
            if project.get("website_status") == "verified" and website:
                counts["verified_websites"] += 1
        declared = catalog.get("counts")
        if not isinstance(declared, dict):
            warnings.append("catalog.counts is absent; counts are calculated from projects")
        else:
            expected = {
                "projects": counts["records"],
                "private": counts["private"],
                "public": counts["records"] - counts["private"],
                "with_live_website": counts["verified_websites"],
            }
            for key, actual in expected.items():
                if declared.get(key) != actual:
                    warnings.append(f"catalog.counts.{key}={declared.get(key)!r}, calculated value is {actual}")
    elif catalog is not None:
        errors.append("catalog root must be an object")

    if isinstance(sites, dict):
        for repository, value in sites.items():
            where = f"README_SITES[{repository!r}]"
            if not isinstance(repository, str) or not re.fullmatch(r"[^/\\s]+/[^/\\s]+", repository):
                errors.append(f"{where}: key must be owner/repository")
            url = value if isinstance(value, str) else value.get("website") if isinstance(value, dict) else None
            if not https_url(url):
                errors.append(f"{where}: declared website must be a safe HTTPS URL")
            if isinstance(value, dict) and value.get("status") == "verified":
                warnings.append(f"{where}: verification status belongs to generated catalog, not the hand-edited override")
            if not isinstance(value, (str, dict)):
                errors.append(f"{where}: expected URL string or object with website")
    elif sites is not None:
        errors.append("README_SITES.json root must be an object")

    return errors, warnings, counts


def main() -> int:
    errors, warnings, counts = validate()
    print("Catalog integrity summary (structural/offline):")
    for key, value in counts.items():
        print(f"  {key}: {value}")
    for warning in warnings:
        print(f"WARNING: {warning}")
    for error in errors:
        print(f"ERROR: {error}")
    if errors:
        print(f"Catalog validation failed with {len(errors)} error(s) and {len(warnings)} warning(s).", file=sys.stderr)
        return 1
    print(f"Catalog validation passed with {len(warnings)} warning(s).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
