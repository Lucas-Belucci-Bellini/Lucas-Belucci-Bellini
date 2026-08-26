#!/usr/bin/env python3
"""Rebuild the profile README from the preserved original visual template.

This one-time migration keeps the original banners, typing strips, badges, ASCII
field-manual panels, skill icons, visual assets, activity blocks and contact area,
while adding the audited generator markers around data that must remain current.
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = Path("/home/ubuntu/profile_readme_audit/original_README.md")
TARGET = ROOT / "README.md"

DASHBOARD = """
## `// LEITURA DO ECOSSISTEMA` · ECOSYSTEM READOUT

<!-- PROFILE-DASHBOARD:START -->
<div align="center">

> O painel abaixo é regenerado a partir do inventário auditado do GitHub. Conteúdo de arquivos privados nunca é publicado.

</div>
<!-- PROFILE-DASHBOARD:END -->
""".strip()

REPOSITORY_SECTIONS = """
## `// MAPA DO ECOSSISTEMA` · PROJECT MAP

<!-- PROJECT-MAP:START -->
<!-- PROJECT-MAP:END -->

## `// REPOSITÓRIOS PÚBLICOS` · PUBLIC PROJECTS

<!-- PUBLIC-PROJECTS:START -->
<!-- PUBLIC-PROJECTS:END -->

## `// REPOSITÓRIOS PRIVADOS` · PRIVATE PROJECTS

<!-- PRIVATE-PROJECTS:START -->
<!-- PRIVATE-PROJECTS:END -->

> 🔒 A seção privada é limitada a metadados de inventário. Nenhuma linha publica código, `.env`, secret, token, credencial ou estrutura interna.
""".strip()

FEATURED = """
![Projects](https://readme-typing-svg.demolab.com?font=IBM+Plex+Mono&weight=700&size=14&duration=4000&pause=9999&color=E8C07A&center=true&vCenter=true&width=580&height=30&lines=%E2%97%86+PROJETOS+EM+DESTAQUE+%2F+FEATURED+MISSIONS+%E2%97%86)

<div align="center">

![Projetos em destaque](./assets/profile-projects.svg)

<!-- FEATURED-PROJECTS:START -->
<!-- FEATURED-PROJECTS:END -->

</div>
""".strip()

LANGUAGE = """
![Lang Analysis](https://readme-typing-svg.demolab.com?font=IBM+Plex+Mono&weight=700&size=14&duration=4000&pause=9999&color=D4A24E&center=true&vCenter=true&width=600&height=30&lines=%E2%97%86+AN%C3%81LISE+DE+LINGUAGENS+%2F+LANGUAGE+ANALYSIS+%E2%97%86)

> 🤖 Seção mantida por um **bot**: a GitHub Action consulta os dados auditados, soma bytes por linguagem nos repositórios públicos e reescreve somente o bloco abaixo. Repositórios privados não entram na tabela pública.

<!-- LANGUAGE-STATS:START -->
<!-- LANGUAGE-STATS:END -->
""".strip()

SITES = """
![Sites](https://readme-typing-svg.demolab.com?font=IBM+Plex+Mono&weight=700&size=14&duration=4000&pause=9999&color=D4A24E&center=true&vCenter=true&width=520&height=30&lines=%E2%97%86+SITES+AO+VIVO+%2F+LIVE+SITES+%E2%97%86)

<div align="center">

<!-- LIVE-PROJECTS:START -->
<!-- LIVE-PROJECTS:END -->

</div>

> Status **live** significa que a homepage declarada respondeu HTTP 200 durante a auditoria. URLs 404 não são apresentadas como online.
""".strip()


def replace_once(text: str, pattern: str, replacement: str) -> str:
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise RuntimeError(f"anchor not found exactly once: {pattern[:80]}")
    return updated


def main() -> None:
    text = SOURCE.read_text(encoding="utf-8")

    # Preserve the original header, badges, field manual, agent files, JARVIS links,
    # skill icons and all personal/activity sections. Replace only stale catalog blocks.
    text = replace_once(
        text,
        r"!\[Projects\].*?(?=\n\n### 🗂 Arsenal completo / Full arsenal)",
        FEATURED,
    )
    text = replace_once(
        text,
        r"### 🗂 Arsenal completo / Full arsenal.*?(?=\n\n---\n\n!\[Lang Analysis\])",
        REPOSITORY_SECTIONS,
    )
    text = replace_once(
        text,
        r"!\[Lang Analysis\].*?<!-- LANG-STATS:END -->",
        LANGUAGE,
    )
    text = replace_once(
        text,
        r"!\[Sites\].*?(?=\n\n---\n\n!\[Personal Log\])",
        SITES,
    )

    # Insert the dashboard immediately after the original field-manual panel.
    field_manual_end = "╚══════════════════════════════════════════════════════════════╝\n```"
    if "<!-- PROFILE-DASHBOARD:START -->" not in text:
        if field_manual_end not in text:
            raise RuntimeError("field manual anchor not found")
        text = text.replace(field_manual_end, field_manual_end + "\n\n" + DASHBOARD, 1)

    TARGET.write_text(text.rstrip() + "\n", encoding="utf-8")
    print(f"restored original visual template -> {TARGET}")


if __name__ == "__main__":
    main()
