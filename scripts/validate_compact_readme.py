#!/usr/bin/env python3
"""Validate the compact public README and the detailed generated archive contract."""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
README = ROOT / "README.md"
ARCHIVE = ROOT / "docs" / "PROFILE-ARCHIVE.md"
MAX_README_BYTES = 6_000
MARKERS = (
    "PROFILE-DASHBOARD",
    "WHAT-I-BUILD",
    "PRODUCT-CARDS",
    "FEATURED-PROJECTS",
    "ARSENAL-STACK",
    "LANGUAGE-BADGES",
    "LANGUAGE-STATS",
    "PUBLIC-PROJECTS",
    "PRIVATE-PROJECTS",
    "WEBSITE-DIRECTORY",
    "LIVE-PROJECTS",
    "ECOSYSTEM-MAP",
    "PROJECT-MAP",
)


def main() -> int:
    errors: list[str] = []
    if not README.is_file() or not ARCHIVE.is_file():
        print("README.md or docs/PROFILE-ARCHIVE.md is missing", file=sys.stderr)
        return 1

    compact = README.read_text(encoding="utf-8")
    archive = ARCHIVE.read_text(encoding="utf-8")
    size = README.stat().st_size

    if size > MAX_README_BYTES:
        errors.append(f"README.md is {size} bytes; maximum is {MAX_README_BYTES}")
    if "docs/PROFILE-ARCHIVE.md" not in compact:
        errors.append("README.md must link to docs/PROFILE-ARCHIVE.md")
    if "Dark-Legion" not in compact:
        errors.append("README.md must link to the Dark Legion project hub")

    for marker in MARKERS:
        start = f"<!-- {marker}:START -->"
        end = f"<!-- {marker}:END -->"
        if archive.count(start) != 1 or archive.count(end) != 1:
            errors.append(f"archive marker pair is missing or duplicated: {marker}")
        if start in compact or end in compact:
            errors.append(f"generated marker leaked into compact README: {marker}")

    try:
        catalog = json.loads((ROOT / "docs" / "project-catalog.json").read_text(encoding="utf-8"))
        for project in catalog.get("projects", []):
            if project.get("private"):
                errors.append("the public project catalog contains a private repository entry")
                break
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"cannot validate private repository exclusions: {error}")

    if errors:
        for error in errors:
            print(f"compact README validation failed: {error}", file=sys.stderr)
        return 1

    print(f"compact README validation passed: {size} bytes; {len(MARKERS)} archive marker pairs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
