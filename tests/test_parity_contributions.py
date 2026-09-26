"""Casos de paridade da coleta de contribuições: o Python é a referência.

    OLD PYTHON ──▶ tests/fixtures/parity/contributions.json ◀── NEW RUST

- `periods`: `period_ranges(start, end)` — as janelas mensais.
- `runs`: o `main()` de verdade do `scripts/update_contribution_timeline.py`
  contra um GraphQL simulado (tests/fake_github.py), com relógio fixo. Ficam
  registrados cada pedido (as variáveis) com a resposta dada, o código de
  saída, o stderr e os dois arquivos gravados, byte a byte.

    cargo test -p profile-core --test parity_contributions

confere o `profile-core sync contributions` contra os mesmos casos.

    UPDATE_PARITY=1 python3.12 -m unittest tests.test_parity_contributions
"""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import sys
import tempfile
import unittest
from datetime import date
from pathlib import Path
from typing import Any
from unittest import mock

from fake_github import FakeGitHub, Response

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "parity" / "contributions.json"
SCRIPT = ROOT / "scripts" / "update_contribution_timeline.py"
REFERENCE_MINOR = (3, 12)


def _load():
    spec = importlib.util.spec_from_file_location("timeline_parity", SCRIPT)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PERIODS = [
    ("2025-09-25", "2026-09-25"),
    ("2023-03-01", "2024-02-29"),
    ("2024-01-31", "2024-03-01"),
    ("2026-09-25", "2026-09-25"),
    ("2026-12-15", "2027-01-02"),
    ("2026-09-26", "2026-09-25"),
]


def collection(variables: dict[str, Any], values: dict[str, Any] | None = None) -> dict[str, Any]:
    month = date.fromisoformat(variables["from"][:10]).month
    base = {
        "contributionCalendar": {"totalContributions": month * 11},
        "totalCommitContributions": month * 7,
        "totalIssueContributions": month % 3,
        "totalPullRequestContributions": month % 5,
        "totalPullRequestReviewContributions": 1,
        "totalRepositoryContributions": month % 2,
        "restrictedContributionsCount": 40 - month,
    }
    base.update(values or {})
    return {"data": {"user": {"contributionsCollection": base}}}


SCENARIOS: list[dict[str, Any]] = [
    {"name": "janela de 13 meses", "now": "2026-09-25T12:00:00Z", "login": "Lucas-Belucci-Bellini",
     "respond": lambda v: collection(v)},
    {"name": "fuso e ano bissexto", "now": "2024-03-01T00:30:00+03:00", "login": "o<&\"'x",
     "respond": lambda v: collection(v)},
    {"name": "valores de tipo inesperado vão crus", "now": "2026-01-05T09:00:00Z", "login": "o",
     "respond": lambda v: collection(v, {"totalCommitContributions": "12", "restrictedContributionsCount": 1.5,
                                         "totalIssueContributions": None})},
    {"name": "errors do GraphQL", "now": "2026-09-25T12:00:00Z", "login": "o",
     "respond": lambda v: {"errors": [{"message": "Could not resolve to a User"}, {"type": "X"}]}},
    {"name": "HTTP de erro", "now": "2026-09-25T12:00:00Z", "login": "o",
     "respond": lambda v: Response(502, {"message": "Bad Gateway"})},
    {"name": "usuário nulo", "now": "2026-09-25T12:00:00Z", "login": "o",
     "respond": lambda v: {"data": {"user": None}}},
    {"name": "falha no meio da janela", "now": "2026-09-25T12:00:00Z", "login": "o",
     "respond": lambda v: Response(500, {}) if v["from"].startswith("2026-03") else collection(v)},
    {"name": "sem token", "now": "2026-09-25T12:00:00Z", "login": "o", "token": None,
     "respond": lambda v: collection(v)},
]


def run(module, scenario: dict[str, Any]) -> dict[str, Any]:
    exchanges: list[dict[str, Any]] = []

    def respond(variables: dict[str, Any]):
        result = scenario["respond"](variables)
        response = result if isinstance(result, Response) else Response(200, result)
        exchanges.append({"variables": variables, "status": response.status, "body": response.body})
        return response

    fake = FakeGitHub()
    fake.graphql(respond)
    token = scenario.get("token", "fake-token")
    env = {"PROFILE_README_TOKEN": token} if token else {}
    with fake, tempfile.TemporaryDirectory() as tmp, \
            mock.patch.dict(os.environ, env, clear=False):
        if not token:
            os.environ.pop("PROFILE_README_TOKEN", None)
            os.environ.pop("GITHUB_TOKEN", None)
        module.GRAPHQL_URL = f"{fake.url}/graphql"
        module.LOGIN = scenario["login"]
        err = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
            code = module.main(["--root", tmp, "--now", scenario["now"]])
        assets = Path(tmp) / "docs" / "assets"
        data = assets / "contributions-timeline-data.json"
        page = assets / "contributions-timeline.html"
        return {
            "input": {"now": scenario["now"], "login": scenario["login"], "token": bool(token),
                      "exchanges": exchanges},
            "expected": {
                "exit": code,
                "stderr": err.getvalue(),
                "data": data.read_text(encoding="utf-8") if data.exists() else None,
                "html": page.read_text(encoding="utf-8") if page.exists() else None,
            },
        }


def build() -> dict[str, Any]:
    module = _load()
    periods = [{"input": [s, e], "expected": [[a.isoformat(), b.isoformat()] for a, b in
                                              module.period_ranges(date.fromisoformat(s), date.fromisoformat(e))]}
               for s, e in PERIODS]
    runs = []
    for scenario in SCENARIOS:
        case = run(module, scenario)
        runs.append({"name": scenario["name"], **case})
    return {
        "schema": "lucas-belucci-bellini/parity-contributions@1",
        "note": "Gerado por tests/test_parity_contributions.py a partir do script real. Não editar à mão.",
        "python": sys.version.split()[0],
        "periods": periods,
        "runs": runs,
    }


def render(data: dict[str, Any]) -> str:
    return json.dumps(data, ensure_ascii=False, indent=2) + "\n"


class ContributionsParityTests(unittest.TestCase):
    def setUp(self) -> None:
        if sys.version_info[:2] != REFERENCE_MINOR:
            self.skipTest(f"fixture do CPython {REFERENCE_MINOR[0]}.{REFERENCE_MINOR[1]} (o do CI)")

    def test_fixture_reflete_o_python_atual(self) -> None:
        data = build()
        if os.environ.get("UPDATE_PARITY") == "1":
            FIXTURE.write_text(render(data), encoding="utf-8")
        recorded = json.loads(FIXTURE.read_text(encoding="utf-8"))
        data["python"] = recorded["python"]
        self.assertEqual(
            FIXTURE.read_text(encoding="utf-8"), render(data),
            "contributions.json não bate com o script atual. Se a coleta mudou de propósito, regenere com a "
            "versão do CI e porte a mudança para o Rust no mesmo PR.",
        )

    def test_armadilhas_estao_cobertas(self) -> None:
        runs = {r["name"]: r for r in build()["runs"]}
        self.assertEqual(13, len(runs["janela de 13 meses"]["input"]["exchanges"]))
        self.assertIn("&lt;&amp;&quot;&#x27;x", runs["fuso e ano bissexto"]["expected"]["html"])
        self.assertEqual(1, runs["errors do GraphQL"]["expected"]["exit"])
        self.assertIn("Could not resolve to a User; GraphQL error", runs["errors do GraphQL"]["expected"]["stderr"])
        self.assertEqual(2, runs["sem token"]["expected"]["exit"])
        self.assertIsNone(runs["falha no meio da janela"]["expected"]["data"], "falha no meio não grava nada")


if __name__ == "__main__":
    unittest.main()
