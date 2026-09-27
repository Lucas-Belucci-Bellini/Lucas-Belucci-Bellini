"""Casos de paridade dos SVGs de `assets/`: o Python é a referência, o Rust confere.

    OLD PYTHON ──▶ tests/fixtures/parity/assets.json ◀── NEW RUST

Chama as funções de desenho de verdade — `build_svg`,
`build_profile_top_langs_svg`, `mesmo_conteudo` e `extensao` do
`.github/scripts/lang_stats.py`; `stats_svg`, `streak_svg`, `trophies_svg` e
`projects_svg` do `.github/scripts/profile_cards.py` — sobre dados montados
para as bordas: cores de reserva por posição, B/KB/MB/GB, barra mínima,
milhar, famílias e "outros", empates, escapes, fuso no carimbo e valores
nulos nos cards. O que fica registrado é a saída, byte a byte.

    cargo test -p profile-render --test assets_parity

A coleta (listagem, linguagens, árvores, GraphQL) é conferida de ponta a ponta
em tests/e2e/github_parity.py, contra um GitHub simulado.

    UPDATE_PARITY=1 python3.12 -m unittest tests.test_parity_assets
"""
from __future__ import annotations

import importlib.util
import json
import os
import sys
import unittest
from datetime import datetime
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "parity" / "assets.json"
REFERENCE_MINOR = (3, 12)


def _load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def stats_data(per_lang: list[tuple[str, int]], per_ext: list[tuple[str, int]], *, repo_count: int,
               generated: str) -> dict[str, Any]:
    return {
        "per_lang": per_lang, "per_ext": per_ext, "repo_count": repo_count,
        "arquivos_total": sum(n for _, n in per_ext), "total_bytes": sum(n for _, n in per_lang),
        "generated": generated,
    }


LANG_CASES = [
    ("cheio: cores de reserva, B a GB, milhar, famílias, empates", stats_data(
        [("JavaScript", 3 * 1024 ** 3 + 7), ("Linguagem<&\"'>", 5 * 1024 ** 2), ("TypeScript", 900_000),
         ("Zig", 2048), ("Python", 1023), ("Nim", 1000), ("Odin", 999), ("Batchfile", 12), ("C#", 1)],
        [("js", 12_345), ("png", 4_000), ("md", 4_000), ("ts", 1_500), ("json", 999), ("glb", 120),
         ("woff2", 40), ("mp4", 33), ("xyz", 4_500), ("csv", 5), ("ab", 5), ("aa", 5), ("svg", 2)],
        repo_count=42, generated="2026-09-25T09:30:00-03:00")),
    ("vazio: nenhuma linguagem nem arquivo", stats_data([], [], repo_count=0, generated="2026-09-25T12:00:00Z")),
    ("barra mínima e vírgula num nome de extensão", stats_data(
        [("HTML", 10_000_000), ("CSS", 0)], [("a,b", 1), ("html", 5_000)], repo_count=1,
        generated="0999-01-02T03:04:00Z")),
    ("oito linguagens e dez tipos exatos, cinco famílias", stats_data(
        [(f"L{n}", 100 - n) for n in range(8)],
        [("py", 10), ("css", 9), ("csv", 8), ("pdf", 7), ("jpg", 6), ("mp3", 5), ("obj", 4), ("ttf", 3),
         ("zzz", 2), ("rs", 1)],
        repo_count=8, generated="2026-12-31T23:59:59+14:00")),
]

CARD_CASES = [
    ("números comuns", {
        "login": "Lucas-Belucci-Bellini", "name": "Lucas", "contributions": 1234, "commits": 987,
        "issues": 12, "pull_requests": 34, "reviews": 5, "repository_contributions": 7, "restricted": 0,
        "repositories": 70, "generated": "2026-09-25 12:00 UTC"}),
    ("nulos, escapes e números grandes", {
        "login": "o", "name": "O'Neil & <Co>", "contributions": None, "commits": 10_000_000,
        "issues": "12\"", "pull_requests": True, "reviews": 0, "repository_contributions": -1,
        "restricted": 1.5, "repositories": 0, "generated": "<agora>"}),
]

PATHS = ["src/app.JS", "Makefile", "dir/.gitignore", "a.b-c", "arquivo.", "x.abcdefghijklm", "x.abcdefghijkl",
         "x.ÇÃO", "dist/pacote.tar.gz", "sem/barra.", "a/b/c.MD", ".env", "LICENSE", "x.py3", "foto.JPEG",
         "x.a_b", "x.12", "caminho/com.ponto/arquivo"]

SAME = [
    ("<t>atualizado · 2026-09-27 08:35 UTC</t><t>19</t>", "<t>atualizado · 2026-09-20 08:35 UTC</t><t>19</t>"),
    ("<t>2026-09-27 08:35 UTC</t><t>20</t>", "<t>2026-09-20 08:35 UTC</t><t>19</t>"),
    ("<t>٢٠٢٦-٠٩-٢٧ ٠٨:٣٥ UTC</t>", "<t>2026-09-20 08:35 UTC</t>"),
    ("sem carimbo", "sem carimbo"),
    ("2026-09-27 08:35 UTC 2026-09-27 08:36 UTC", "@ @"),
]


def build() -> dict[str, Any]:
    lang_stats = _load("lang_stats_parity", ROOT / ".github" / "scripts" / "lang_stats.py")
    cards = _load("profile_cards_parity", ROOT / ".github" / "scripts" / "profile_cards.py")
    lang_cases = []
    for name, data in LANG_CASES:
        python_data = dict(data, per_lang=dict(data["per_lang"]), per_ext=dict(data["per_ext"]),
                           generated=datetime.fromisoformat(data["generated"].replace("Z", "+00:00")))
        lang_cases.append({
            "name": name, "input": data,
            "expected": {"lang_stats": lang_stats.build_svg(python_data),
                         "top_langs": lang_stats.build_profile_top_langs_svg(python_data)},
        })
    card_cases = [{
        "name": name, "input": data,
        "expected": {"stats": cards.stats_svg(data), "streak": cards.streak_svg(data),
                     "trophies": cards.trophies_svg(data), "projects": cards.projects_svg()},
    } for name, data in CARD_CASES]
    return {
        "schema": "lucas-belucci-bellini/parity-assets@1",
        "note": "Gerado por tests/test_parity_assets.py a partir do lang_stats.py e do profile_cards.py reais. "
                "Não editar à mão.",
        "python": sys.version.split()[0],
        "lang_stats": lang_cases,
        "cards": card_cases,
        "extensao": [[path, lang_stats.extensao(path)] for path in PATHS],
        "mesmo_conteudo": [[new, old, lang_stats.mesmo_conteudo(new, old)] for new, old in SAME],
        "human": [[n, lang_stats.human(n)] for n in (0, 1023, 1024, 1536, 1024 ** 2 - 1, 1024 ** 2,
                                                       1024 ** 3 - 1, 1024 ** 3, 5 * 1024 ** 4)],
    }


def render(data: dict[str, Any]) -> str:
    return json.dumps(data, ensure_ascii=False, indent=2) + "\n"


class AssetsParityTests(unittest.TestCase):
    data: dict[str, Any]

    @classmethod
    def setUpClass(cls) -> None:
        if sys.version_info[:2] != REFERENCE_MINOR:
            raise unittest.SkipTest(f"fixture do CPython {REFERENCE_MINOR[0]}.{REFERENCE_MINOR[1]} (o do CI)")
        cls.data = build()
        if os.environ.get("UPDATE_PARITY") == "1":
            FIXTURE.write_text(render(cls.data), encoding="utf-8")

    def test_fixture_reflete_o_python_atual(self) -> None:
        recorded = json.loads(FIXTURE.read_text(encoding="utf-8"))
        data = dict(self.data, python=recorded["python"])
        self.assertEqual(
            FIXTURE.read_text(encoding="utf-8"), render(data),
            "assets.json não bate com o lang_stats.py/profile_cards.py atuais. Se o desenho mudou de propósito, "
            "regenere com a versão do CI e porte a mudança para o profile-render no mesmo PR.",
        )

    def test_armadilhas_estao_cobertas(self) -> None:
        full = self.data["lang_stats"][0]["expected"]["lang_stats"]
        self.assertIn("3.00 GB", full)
        self.assertIn("12.345", full, "milhar com ponto")
        self.assertIn("Linguagem&lt;&amp;&quot;'&gt;", full, "esc() deixa o apóstrofo passar")
        self.assertIn('fill="#8fa6c4"/><text x="370" y="389"', full, "Nim: cor de reserva pela posição (6ª)")
        self.assertIn('fill="#d4a24e"/><text x="370" y="418"', full, "Odin: o ciclo volta ao começo")
        self.assertIn(">OUTROS<", full)
        self.assertIn("2026-09-25 09:30 UTC", full, "hora local do fuso, com o texto UTC")
        comma = self.data["lang_stats"][2]["expected"]["lang_stats"]
        self.assertIn(">.a.b<", comma, "o replace pega a linha inteira")
        self.assertIn("999-01-02 03:04 UTC", comma)
        nulls = self.data["cards"][1]["expected"]["stats"]
        self.assertIn(">None<", nulls)
        self.assertIn("&lt;agora&gt;", nulls)
        self.assertIn(["dir/.gitignore", None], self.data["extensao"])
        self.assertIn(["x.ÇÃO", "ção"], self.data["extensao"])
        self.assertEqual([True, False, True, True, True], [case[2] for case in self.data["mesmo_conteudo"]])


if __name__ == "__main__":
    unittest.main()
