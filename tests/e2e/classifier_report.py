#!/usr/bin/env python3
"""Relatório do `classifier@2` (D-039): o que muda na vitrine se o dono ligar.

Usa a mesma reconstrução da árvore real de `readme_validators.py` (o README
publicado, os manifestos e um inventário refeito do `docs/project-catalog.json`
versionado) e gera o README duas vezes com o `profile-core`: uma com
`py-classify@1` (o padrão, igual ao Python) e outra com `classifier@2`. Imprime
em Markdown os projetos que mudam de rótulo e as linhas do README que mudam.

    python3 tests/e2e/classifier_report.py target/release/profile-core > relatório.md

O inventário reconstruído não tem `fork` nem `homepage` exatos (vêm do status e
da origem do site no catálogo); a lista definitiva sai do mesmo comando com os
dados reais — ver docs/audits/2026-09-27-classifier-v2.md.
"""
from __future__ import annotations

import difflib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import readme_validators as real  # noqa: E402


def render(binary: str, root: Path, work: Path, classifier: str) -> tuple[str, dict]:
    out = work / classifier
    command = [binary, "render", "readme", "--root", str(root), "--input-repos", str(work / "repos.json"),
               "--languages-dir", str(work / "languages"), "--site-checks-fixture", str(work / "checks.json"),
               "--now", real.NOW, "--classifier", classifier, "--out-dir", str(out)]
    result = subprocess.run(command, capture_output=True, text=True, env=real.CLEAN_ENV, check=False)
    if result.returncode != 0:
        raise SystemExit(f"{classifier}: {result.stderr}")
    catalog = json.loads((out / "project-catalog.json").read_text(encoding="utf-8"))
    return (out / "README.md").read_text(encoding="utf-8"), catalog


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    binary = str(Path(sys.argv[1]).resolve())
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        repos, checks = real.inventory()
        (work / "repos.json").write_text(json.dumps(repos, ensure_ascii=False), encoding="utf-8")
        (work / "checks.json").write_text(json.dumps(checks, ensure_ascii=False), encoding="utf-8")
        (work / "languages").mkdir()
        for full_name, data in real.languages(repos).items():
            (work / "languages" / f"{full_name.replace('/', '__')}.json").write_text(json.dumps(data), encoding="utf-8")
        root = work / "tree"
        real.copy_tree(root)
        v1_readme, v1 = render(binary, root, work, "py-classify@1")
        v2_readme, v2 = render(binary, root, work, "classifier@2")

    by_repo = {p["repository"]: p for p in v2["projects"]}
    changed = [(p, by_repo[p["repository"]]) for p in v1["projects"]
               if p["repository"] in by_repo and p["category_label"] != by_repo[p["repository"]]["category_label"]]
    public = [(a, b) for a, b in changed if not a["private"]]
    print(f"## Rótulos que mudam ({len(changed)} de {len(v1['projects'])} projetos; {len(public)} públicos)\n")
    print("| Projeto | Descrição pública | py-classify@1 | classifier@2 |")
    print("|:---|:---|:---|:---|")
    for before, after in sorted(changed, key=lambda pair: pair[0]["name"].lower()):
        name = before["name"] + (" 🔒" if before["private"] else "")
        description = before["description"].replace("|", "\\|")
        print(f"| {name} | {description} | {before['category_label']} | **{after['category_label']}** |")

    diff = [line for line in difflib.unified_diff(v1_readme.splitlines(), v2_readme.splitlines(),
                                                  "README (py-classify@1)", "README (classifier@2)", lineterm="", n=0)
            if not line.startswith("@@")]
    print(f"\n## Linhas do README que mudam ({sum(1 for l in diff if l[:1] in '+-' and l[:3] not in ('+++', '---'))})\n")
    print("```diff")
    print("\n".join(diff))
    print("```")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
