#!/usr/bin/env python3
"""Ensure a refresh changed only generated README blocks and allowed assets."""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

MARKERS = [
    "PROFILE-DASHBOARD",
    "FEATURED-PROJECTS",
    "CURATED-FEATURED",
    "ARSENAL-STACK",
    "LANGUAGE-BADGES",
    "LANGUAGE-STATS",
    "PUBLIC-PROJECTS",
    "PRIVATE-PROJECTS",
    "LIVE-PROJECTS",
    "PROJECT-MAP",
]


def canonicalize(text: str) -> str:
    for marker in MARKERS:
        pattern = re.compile(
            rf"<!-- {re.escape(marker)}:START -->.*?<!-- {re.escape(marker)}:END -->",
            re.S,
        )
        text, count = pattern.subn(
            f"<!-- {marker}:START -->\n__GENERATED_{marker}__\n<!-- {marker}:END -->",
            text,
            count=1,
        )
        if count != 1:
            raise ValueError(f"README marker not found exactly once: {marker}")
    return text


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--before", required=True, type=Path)
    parser.add_argument("--after", required=True, type=Path)
    args = parser.parse_args()
    before = canonicalize(args.before.read_text(encoding="utf-8"))
    after = canonicalize(args.after.read_text(encoding="utf-8"))
    if before != after:
        print("static README content changed outside generated markers", file=sys.stderr)
        return 1
    print("dynamic-only README validation: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
