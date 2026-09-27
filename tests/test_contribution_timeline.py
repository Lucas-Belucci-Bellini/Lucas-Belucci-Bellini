"""`scripts/update_contribution_timeline.py` contra um GraphQL simulado.

A coleta mensal é a referência do `profile-core sync contributions`; --root e
--now existem para as duas implementações rodarem com a mesma raiz e o mesmo
relógio (tests/e2e/github_parity.py).
"""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import tempfile
import unittest
from datetime import date
from pathlib import Path
from unittest import mock

from fake_github import FakeGitHub, Response

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "update_contribution_timeline.py"
SPEC = importlib.util.spec_from_file_location("update_contribution_timeline", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def collection(variables: dict) -> dict:
    """Valores derivados da janela, para cada linha ser conferível."""
    month = date.fromisoformat(variables["from"][:10]).month
    return {"data": {"user": {"contributionsCollection": {
        "contributionCalendar": {"totalContributions": month * 10},
        "totalCommitContributions": month,
        "totalIssueContributions": 1,
        "totalPullRequestContributions": 2,
        "totalPullRequestReviewContributions": 0,
        "totalRepositoryContributions": 3,
        "restrictedContributionsCount": 4,
    }}}}


class ContributionTimelineTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.saved = MODULE.GRAPHQL_URL

    def tearDown(self) -> None:
        MODULE.GRAPHQL_URL = self.saved
        self.tmp.cleanup()

    def run_main(self, fake: FakeGitHub, *args: str) -> tuple[int, str]:
        MODULE.GRAPHQL_URL = f"{fake.url}/graphql"
        err = io.StringIO()
        with mock.patch.dict(os.environ, {"PROFILE_README_TOKEN": "fake-token"}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(err):
            code = MODULE.main(["--root", str(self.root), *args])
        return code, err.getvalue()

    def test_janelas_mensais_com_relogio_fixo(self) -> None:
        fake = FakeGitHub()
        fake.graphql(collection)
        with fake:
            code, _ = self.run_main(fake, "--now", "2026-09-25T12:00:00Z")
        self.assertEqual(0, code)
        data = json.loads((self.root / "docs/assets/contributions-timeline-data.json").read_text(encoding="utf-8"))
        self.assertEqual(("2025-09-25", "2026-09-25", "2026-09-25T12:00:00+00:00"),
                         (data["window_start"], data["window_end"], data["generated_at"]))
        self.assertEqual(13, len(data["rows"]))
        self.assertEqual({"total": 90, "commits": 9, "pull_requests": 2, "issues": 1, "reviews": 0,
                          "repositories": 3, "restricted": 4,
                          "period_start": "2025-09-25", "period_end": "2025-09-30"}, data["rows"][0])
        self.assertEqual(("2026-09-01", "2026-09-25"), (data["rows"][-1]["period_start"], data["rows"][-1]["period_end"]))
        self.assertTrue((self.root / "docs/assets/contributions-timeline.html").exists())
        self.assertEqual({"Bearer fake-token"}, {auth for _, _, auth in fake.requests})

    def test_erro_do_graphql_sai_com_1_e_nao_grava(self) -> None:
        fake = FakeGitHub()
        fake.graphql(lambda variables: {"errors": [{"message": "Could not resolve to a User"}]})
        with fake:
            code, err = self.run_main(fake, "--now", "2026-09-25T12:00:00Z")
        self.assertEqual(1, code)
        self.assertIn("Could not resolve to a User", err)
        self.assertFalse((self.root / "docs/assets/contributions-timeline-data.json").exists())

    def test_http_de_erro_vira_mensagem_com_o_codigo(self) -> None:
        fake = FakeGitHub()
        fake.graphql(lambda variables: Response(502, {"message": "Bad Gateway"}))
        with fake:
            code, err = self.run_main(fake, "--now", "2026-09-25T12:00:00Z")
        self.assertEqual((1, True), (code, "GitHub GraphQL HTTP 502" in err))


if __name__ == "__main__":
    unittest.main()
