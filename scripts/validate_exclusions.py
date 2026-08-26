#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
manifest = json.loads((ROOT / "docs" / "README_EXCLUDED.json").read_text(encoding="utf-8"))
readme = (ROOT / "README.md").read_text(encoding="utf-8")
excluded = [str(name) for name in manifest.get("repositories", [])]
found = [name for name in excluded if name in readme]
if found:
    print("excluded repository names found in README: " + ", ".join(found), file=sys.stderr)
    raise SystemExit(1)
print(f"excluded repositories absent from README: {len(excluded)}")
