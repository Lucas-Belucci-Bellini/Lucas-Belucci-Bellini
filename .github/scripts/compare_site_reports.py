#!/usr/bin/env python3
"""Compara o relatório do `check_websites.py --json` com o do `profile-core check sites --json`.

Modo B (sombra) da migração do monitor de sites — docs/migration/PYTHON-TO-RUST.md §4.
Ignora só o `checked_at` (os dois rodam ao mesmo tempo, mas não no mesmo segundo).
Escreve um resumo em Markdown (no `$GITHUB_STEP_SUMMARY`, quando existir) e sai com
1 se houver qualquer diferença: é o sinal que conta para o critério de saída do modo B
(14 execuções agendadas seguidas sem diferença, ou todas explicadas).

    python3 .github/scripts/compare_site_reports.py python.json rust.json
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path


def load(path: str) -> dict:
    text = Path(path).read_text(encoding="utf-8")
    if not text.lstrip().startswith("{"):
        return {"checked": 0, "down": 0, "sites": [], "raw": text.strip()}
    report = json.loads(text)
    for site in report.get("sites", []):
        site.pop("checked_at", None)
    return report


def compare(python: dict, rust: dict) -> list[str]:
    differences = []
    if python.get("raw") != rust.get("raw"):
        differences.append(f"saída sem JSON: Python {python.get('raw')!r} × Rust {rust.get('raw')!r}")
    for key in ("checked", "down"):
        if python.get(key) != rust.get(key):
            differences.append(f"`{key}`: Python {python.get(key)} × Rust {rust.get(key)}")
    py_sites = {site["repository"]: site for site in python.get("sites", [])}
    rs_sites = {site["repository"]: site for site in rust.get("sites", [])}
    for repository in sorted(py_sites.keys() | rs_sites.keys()):
        py_site, rs_site = py_sites.get(repository), rs_sites.get(repository)
        if py_site != rs_site:
            fields = sorted(k for k in (py_site or {}).keys() | (rs_site or {}).keys()
                            if (py_site or {}).get(k) != (rs_site or {}).get(k))
            detail = ", ".join(f"{k}: {(py_site or {}).get(k)!r} × {(rs_site or {}).get(k)!r}" for k in fields)
            differences.append(f"`{repository}` — {detail}")
    return differences


def main() -> int:
    python, rust = load(sys.argv[1]), load(sys.argv[2])
    differences = compare(python, rust)
    lines = ["## Monitor de sites — sombra (Python × Rust)", ""]
    lines.append(f"{python.get('checked', 0)} sites · {python.get('down', 0)} fora do ar (Python)")
    lines.append("")
    if differences:
        lines.append(f"**{len(differences)} diferença(s):**")
        lines.extend(f"- {difference}" for difference in differences)
    else:
        lines.append("**Idênticos** (exceto `checked_at`).")
    summary = "\n".join(lines) + "\n"
    print(summary)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(summary)
    return 1 if differences else 0


if __name__ == "__main__":
    raise SystemExit(main())
