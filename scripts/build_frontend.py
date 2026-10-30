#!/usr/bin/env python3
"""Compile the portfolio browser runtime from strict TypeScript sources."""
from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "js"


def main() -> int:
    if shutil.which("node") is None or shutil.which("npx") is None:
        print("Node.js 22+ and npm/npx are required to compile the frontend.", file=sys.stderr)
        return 2

    OUTPUT.mkdir(parents=True, exist_ok=True)
    command = [
        "npx",
        "--yes",
        "--package",
        "typescript@5.9.3",
        "tsc",
        "--strict",
        "--noUncheckedIndexedAccess",
        "--noEmitOnError",
        "--target",
        "ES2022",
        "--module",
        "ES2022",
        "--moduleResolution",
        "bundler",
        "--lib",
        "ES2022,DOM",
        "--rootDir",
        "frontend",
        "--outDir",
        "assets/js",
        "frontend/main.ts",
    ]
    try:
        subprocess.run(command, cwd=ROOT, check=True)
    except subprocess.CalledProcessError as exc:
        print(f"TypeScript compilation failed with exit code {exc.returncode}.", file=sys.stderr)
        return exc.returncode or 1

    expected = (OUTPUT / "main.js", OUTPUT / "contracts.js")
    missing = [path.relative_to(ROOT).as_posix() for path in expected if not path.is_file()]
    if missing:
        print(f"TypeScript compiler did not emit expected files: {', '.join(missing)}", file=sys.stderr)
        return 1

    print("Compiled strict TypeScript frontend: assets/js/main.js and assets/js/contracts.js")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
