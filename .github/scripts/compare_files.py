#!/usr/bin/env python3
"""Compara, byte a byte, pares de arquivos gerados pelo Python e pelo profile-core.

Modo B (sombra) da coleta — docs/migration/PYTHON-TO-RUST.md §4. Cada argumento é
`rótulo=arquivo_python:arquivo_rust`. Arquivo ausente dos dois lados conta como
igual (o monitor não reescreve o estado quando nada mudou). Escreve um resumo em
Markdown no `$GITHUB_STEP_SUMMARY`, quando existir, e sai com 1 se qualquer par
diferir — o sinal que conta para o critério de saída do modo B.

    python3 .github/scripts/compare_files.py estado=py/state.json:rs/state.json
"""
from __future__ import annotations

import os
import sys
from pathlib import Path


def read(path: str) -> str | None:
    file = Path(path)
    return file.read_text(encoding="utf-8") if file.exists() else None


def first_difference(python: str, rust: str) -> str:
    a, b = python.splitlines(), rust.splitlines()
    for number, (left, right) in enumerate(zip(a, b), start=1):
        if left != right:
            return f"linha {number}: python `{left[:200]}` · rust `{right[:200]}`"
    return f"tamanhos diferentes: python {len(a)} linhas · rust {len(b)} linhas"


def main(pairs: list[str]) -> int:
    if not pairs:
        print(__doc__, file=sys.stderr)
        return 2
    rows, differences = [], 0
    for pair in pairs:
        label, _, paths = pair.partition("=")
        python_path, _, rust_path = paths.partition(":")
        python, rust = read(python_path), read(rust_path)
        if python == rust:
            rows.append(f"| {label} | ✅ igual | {'ausente nos dois' if python is None else f'{len(python)} bytes'} |")
            continue
        differences += 1
        if python is None or rust is None:
            detail = f"só {'o Rust' if python is None else 'o Python'} gravou"
        else:
            detail = first_difference(python, rust)
        rows.append(f"| {label} | ❌ diferente | {detail} |")
    summary = "\n".join([
        "## Modo sombra — coleta (Python × profile-core)",
        "",
        "| Saída | Resultado | Detalhe |",
        "|:---|:---|:---|",
        *rows,
        "",
        "Nenhuma diferença." if not differences else f"**{differences} diferença(s).** Diferença explicada por "
        "mudança real no GitHub entre as duas leituras não conta contra o critério; as outras são defeito do port.",
    ])
    print(summary)
    target = os.environ.get("GITHUB_STEP_SUMMARY")
    if target:
        with open(target, "a", encoding="utf-8") as handle:
            handle.write(summary + "\n")
    return 1 if differences else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
