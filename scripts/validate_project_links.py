#!/usr/bin/env python3
"""Valida a regra de CTA do perfil.

A regra que este repositório existe para sustentar:

    projeto COM site verificado  →  o site vem primeiro, o código depois
    projeto SEM site             →  só o código

O validador lê `docs/project-catalog.json` e o `README.md` gerado e falha se a
ordem estiver invertida em qualquer lugar. É o teste que impede a vitrine
voltar a ser uma lista de repositórios sem ninguém perceber.

    python3 scripts/validate_project_links.py
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
README = ROOT / "README.md"
CATALOG = ROOT / "docs" / "project-catalog.json"

# Blocos que apresentam projetos ao visitante e, portanto, seguem a regra.
SHOWCASE_MARKERS = ("PRODUCT-CARDS", "FEATURED-PROJECTS", "WEBSITE-DIRECTORY", "LIVE-PROJECTS")


def block(text: str, marker: str) -> str:
    match = re.search(
        rf"<!-- {re.escape(marker)}:START -->(.*?)<!-- {re.escape(marker)}:END -->",
        text,
        re.S,
    )
    return match.group(1) if match else ""


def main() -> int:
    problems: list[str] = []

    if not CATALOG.exists():
        print(f"catálogo ausente: {CATALOG}", file=sys.stderr)
        return 2

    catalog = json.loads(CATALOG.read_text(encoding="utf-8"))
    projects = catalog.get("projects", [])
    if not projects:
        print("catálogo sem projetos", file=sys.stderr)
        return 2

    # 1. Coerência do próprio catálogo.
    for project in projects:
        name = project.get("name", "?")
        website = project.get("website")
        primary = project.get("primary_cta")
        secondary = project.get("secondary_cta")

        if website and primary != "website":
            problems.append(f"{name}: tem site mas primary_cta é {primary!r}")
        if website and secondary != "github":
            problems.append(f"{name}: tem site mas secondary_cta é {secondary!r}")
        if not website and primary != "github":
            problems.append(f"{name}: sem site mas primary_cta é {primary!r}")
        if not website and secondary is not None:
            problems.append(f"{name}: sem site mas tem secondary_cta {secondary!r}")

        if project.get("private") and website:
            problems.append(f"{name}: repositório privado não pode anunciar site público")

        if website and not str(website).startswith("https://"):
            problems.append(f"{name}: site precisa ser https, veio {website!r}")

        if website and project.get("website_status") != "verified":
            problems.append(
                f"{name}: site publicado com status {project.get('website_status')!r}; "
                "só 'verified' pode aparecer"
            )

        # Uma URL declarada que não respondeu fica registrada, mas nunca vira link.
        declared = project.get("website_declared")
        if declared and project.get("website_status") != "verified" and website:
            problems.append(f"{name}: URL fora do ar publicada como site")
        if project.get("private") and declared:
            problems.append(f"{name}: repositório privado não declara site público")

    # 2. Ordem no README renderizado.
    if README.exists():
        text = README.read_text(encoding="utf-8")
        for marker in SHOWCASE_MARKERS:
            body = block(text, marker)
            if not body:
                continue
            for line in body.splitlines():
                if "](http" not in line:
                    continue
                site_pos = max(line.find("Abrir site"), line.find("ABRIR%20SITE"))
                code_pos = max(line.find("[código]"), line.find("C%C3%93DIGO"))
                if site_pos == -1 or code_pos == -1:
                    continue
                if code_pos < site_pos:
                    problems.append(f"{marker}: código aparece antes do site em «{line.strip()[:70]}…»")

    if problems:
        print("validação de CTA falhou:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1

    with_site = sum(1 for p in projects if p.get("website"))
    print(
        f"project-links validation: pass · {len(projects)} projetos, "
        f"{with_site} com site como CTA primário"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
