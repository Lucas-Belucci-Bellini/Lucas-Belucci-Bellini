#!/usr/bin/env python3
from pathlib import Path
import json
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
README = ROOT / "README.md"
text = README.read_text(encoding="utf-8")
errors = []
markers = [
    "PROFILE-DASHBOARD",
    "FEATURED-PROJECTS",
    "LIVE-PROJECTS",
    "PROJECT-MAP",
    "PUBLIC-PROJECTS",
    "PRIVATE-PROJECTS",
    "LANGUAGE-STATS",
]
for marker in markers:
    start = f"<!-- {marker}:START -->"
    end = f"<!-- {marker}:END -->"
    if text.count(start) != 1 or text.count(end) != 1:
        errors.append(f"marker pair: {marker}")
for token in [
    "capsule-render",
    "readme-typing-svg",
    "skillicons.dev",
    "assets/profile-projects.svg",
    "assets/jarvis-console.svg",
    "assets/profile-stats.svg",
    "assets/profile-top-langs.svg",
    "assets/profile-streak.svg",
    "assets/profile-trophies.svg",
    "github-contribution-grid-snake-dark.svg",
    "komarev.com/ghpvc",
]:
    if token not in text:
        errors.append(f"missing original visual component: {token}")
for asset in [
    "jarvis-console.svg",
    "lang-stats.svg",
    "profile-projects.svg",
    "profile-stats.svg",
    "profile-streak.svg",
    "profile-top-langs.svg",
    "profile-trophies.svg",
]:
    if not (ROOT / "assets" / asset).is_file() or (ROOT / "assets" / asset).stat().st_size == 0:
        errors.append(f"missing asset: {asset}")
try:
    json.loads((ROOT / "docs" / "README_SITES.json").read_text(encoding="utf-8"))
except Exception as exc:
    errors.append(f"README_SITES.json: {exc}")
public_count = text.count("| **")
print(json.dumps({"errors": errors, "visual_components": 11, "readme_bytes": len(text.encode("utf-8")), "public_count_markers": public_count}, ensure_ascii=False))
sys.exit(1 if errors else 0)
