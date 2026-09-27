#!/usr/bin/env python3
"""Saúde dos sites do catálogo.

Carrega as URLs conhecidas, verifica cada uma com concorrência limitada e
imprime um relatório. **Não altera conteúdo editorial** — nem o README, nem os
manifestos. Serve para responder "quais sites estão no ar agora?" sem rodar o
gerador inteiro.

    python3 scripts/check_websites.py
    python3 scripts/check_websites.py --json
    python3 scripts/check_websites.py --fail-on-down

O `checked_at` aparece aqui, no relatório, e **não** no catálogo versionado:
um carimbo que muda a cada execução criaria commit a cada execução, e este
repositório trabalha com "dado igual → README igual → nenhum commit".
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from project_catalog import check_websites, normalize_site_overrides  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
SITES_FILE = ROOT / "docs" / "README_SITES.json"
CATALOG_FILE = ROOT / "docs" / "project-catalog.json"


def collect_urls(root: Path = ROOT) -> dict[str, str]:
    """URLs conhecidas, por repositório.

    Usa o catálogo gerado quando existir (é a fonte mais completa, porque
    inclui homepages descobertas via GitHub) e cai no manifesto quando não.
    `root` só muda nos testes e na comparação com o `profile-core check sites`.
    """
    urls: dict[str, str] = {}
    catalog_file = root / CATALOG_FILE.relative_to(ROOT)
    sites_file = root / SITES_FILE.relative_to(ROOT)

    if catalog_file.exists():
        try:
            catalog = json.loads(catalog_file.read_text(encoding="utf-8"))
            for project in catalog.get("projects", []):
                # `website_declared` inclui os deployments que estavam fora do ar
                # na última geração; são justamente os que precisam ser cobrados.
                website = project.get("website_declared") or project.get("website")
                if website:
                    urls[str(project.get("repository"))] = str(website)
        except (OSError, ValueError):
            pass

    if sites_file.exists():
        try:
            overrides = normalize_site_overrides(json.loads(sites_file.read_text(encoding="utf-8")))
            for repo, entry in overrides.items():
                urls.setdefault(repo, str(entry["website"]))
        except (OSError, ValueError):
            pass

    return urls


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="saída em JSON")
    parser.add_argument("--timeout", type=float, default=15.0)
    parser.add_argument("--retries", type=int, default=1, help="tentativas extras por URL")
    parser.add_argument("--max-workers", type=int, default=6)
    parser.add_argument(
        "--fail-on-down",
        action="store_true",
        help="sai com código 1 se algum site conhecido estiver fora",
    )
    parser.add_argument("--root", type=Path, default=ROOT, help="raiz com docs/ (padrão: este repositório)")
    args = parser.parse_args(argv)

    urls = collect_urls(args.root)
    if not urls:
        print("nenhuma URL conhecida; rode o gerador ou preencha docs/README_SITES.json")
        return 0

    checks = check_websites(
        urls.values(),
        max_workers=args.max_workers,
        timeout=args.timeout,
        retries=args.retries,
    )

    rows = []
    for repo, url in sorted(urls.items()):
        check = checks.get(url)
        rows.append(
            {
                "repository": repo,
                "website": url,
                "status": check.status if check else "unknown",
                "http_status": check.http_status if check else 0,
                "final_url": check.final_url if check else "",
                "checked_at": check.checked_at if check else "",
            }
        )

    down = [r for r in rows if r["status"] != "verified"]

    if args.json:
        print(json.dumps({"checked": len(rows), "down": len(down), "sites": rows}, ensure_ascii=False, indent=2))
    else:
        for row in rows:
            mark = "ok  " if row["status"] == "verified" else "FORA"
            http = row["http_status"] or "—"
            print(f"  {mark} {http:>4}  {row['repository']}  →  {row['website']}")
            if row["final_url"] and row["final_url"] != row["website"]:
                print(f"            redireciona para: {row['final_url']}")
        print(f"\n{len(rows)} sites verificados · {len(down)} fora do ar")

    if args.fail_on_down and down:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
