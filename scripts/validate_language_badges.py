"""Validate the generated language badges and their public Markdown contract."""
from __future__ import annotations

import re
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
README = ROOT / "README.md"

START = "<!-- LANGUAGE-BADGES:START -->"
END = "<!-- LANGUAGE-BADGES:END -->"
STATS_START = "<!-- LANGUAGE-STATS:START -->"
STATS_END = "<!-- LANGUAGE-STATS:END -->"
EXPECTED_CATEGORIES = {
    "Frameworks & Web",
    "Infraestrutura & DevOps",
    "IA & Conhecimento",
    "Hardware & Simulação",
}


def block(text: str, start: str, end: str) -> str:
    pattern = re.compile(re.escape(start) + r"(.*?)" + re.escape(end), re.S)
    match = pattern.search(text)
    if not match:
        raise AssertionError(f"marcadores ausentes: {start} / {end}")
    return match.group(1)


def assert_language_badges(readme: str) -> tuple[int, list[str], list[str]]:
    badges_block = block(readme, START, END)
    badges = re.findall(r"\[!\[([^\]]+)\]\((https://img\.shields\.io/badge/[^)]+)\)\]", badges_block)
    if not badges:
        raise AssertionError("nenhum badge de linguagem encontrado")

    labels = [label for label, _ in badges]
    if len(labels) != len(set(labels)):
        raise AssertionError("há labels de linguagem duplicados")
    if any("%23" in label or "%2F" in label for label in labels):
        raise AssertionError("label percent-encoded visível no Markdown")
    if "C#" not in labels or "PL/pgSQL" not in labels:
        raise AssertionError("C# e PL/pgSQL precisam permanecer legíveis")

    for label, url in badges:
        parsed = urlparse(url)
        if parsed.netloc != "img.shields.io" or not parsed.path.startswith("/badge/"):
            raise AssertionError(f"endpoint inesperado para {label}: {url}")
        if "style=flat-square" not in parsed.query or "labelColor=0e0c16" not in parsed.query:
            raise AssertionError(f"estilo incompleto para {label}: {url}")
    return len(badges), labels, [url for _, url in badges]


def assert_categories(readme: str) -> list[str]:
    arsenal = block(readme, "<!-- ARSENAL-STACK:START -->", "<!-- ARSENAL-STACK:END -->")
    categories = re.findall(r"^### [^\n]* (Frameworks & Web|Infraestrutura & DevOps|IA & Conhecimento|Hardware & Simulação)$", arsenal, re.M)
    missing = EXPECTED_CATEGORIES.difference(categories)
    if missing:
        raise AssertionError(f"categorias ausentes no Arsenal: {', '.join(sorted(missing))}")
    return categories


def assert_language_count_matches(readme: str, badge_count: int) -> None:
    stats = block(readme, STATS_START, STATS_END)
    table_rows = re.findall(r"^\| \d+ \| \*\*[^|]+\*\* \|", stats, re.M)
    if len(table_rows) != badge_count:
        raise AssertionError(f"badges={badge_count}, linhas da matriz={len(table_rows)}")


def check_one_url(url: str) -> str | None:
    last_error: Exception | None = None
    for attempt in range(3):
        request = Request(url, headers={"User-Agent": "profile-readme-badge-validation/1.0"})
        try:
            with urlopen(request, timeout=15) as response:
                if 200 <= response.status < 400:
                    return None
                last_error = AssertionError(f"HTTP {response.status}")
        except (HTTPError, URLError, TimeoutError) as error:
            last_error = error
        if attempt < 2:
            time.sleep(0.5 * (attempt + 1))
    return f"{url} ({last_error})"


def check_urls(urls: list[str]) -> None:
    failures: list[str] = []
    workers = min(8, max(1, len(urls)))
    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = [executor.submit(check_one_url, url) for url in urls]
        for future in as_completed(futures):
            failure = future.result()
            if failure:
                failures.append(failure)
    if failures:
        raise AssertionError("badges indisponíveis: " + "; ".join(sorted(failures)))


def main() -> int:
    try:
        readme = README.read_text(encoding="utf-8")
        badge_count, labels, urls = assert_language_badges(readme)
        categories = assert_categories(readme)
        assert_language_count_matches(readme, badge_count)
        check_urls(urls)
    except (OSError, AssertionError) as error:
        print(f"language badge validation failed: {error}", file=sys.stderr)
        return 1
    print(
        f"language badge validation passed: {badge_count} badges, "
        f"{len(categories)} tool categories, HTTP 2xx/3xx for all badge URLs"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

