from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import tempfile
import unittest
from pathlib import Path

from fake_github import FakeGitHub


SCRIPT = Path(__file__).resolve().parents[1] / ".github" / "scripts" / "ecosystem_watch.py"
SPEC = importlib.util.spec_from_file_location("ecosystem_watch", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class EcosystemWatchV2Tests(unittest.TestCase):
    def test_excludes_profile_repository_case_insensitively(self) -> None:
        self.assertTrue(MODULE.is_profile_repository("Lucas-Belucci-Bellini"))
        self.assertTrue(MODULE.is_profile_repository("lucas-belucci-bellini"))
        self.assertFalse(MODULE.is_profile_repository("Projeto-Baluarte"))

    def test_unchanged_repository_state_is_noop(self) -> None:
        state = {"repositories": {"Projeto-Baluarte": {"sha": "abc", "branch": "main"}}}
        current = {"Projeto-Baluarte": {"sha": "abc", "branch": "main"}}
        self.assertFalse(MODULE.semantic_state_changed(state, current))

    def test_changed_sha_requires_snapshot(self) -> None:
        state = {"repositories": {"Projeto-Baluarte": {"sha": "abc", "branch": "main"}}}
        current = {"Projeto-Baluarte": {"sha": "def", "branch": "main"}}
        self.assertTrue(MODULE.semantic_state_changed(state, current))

    def test_error_state_is_part_of_semantic_snapshot(self) -> None:
        state = {"repositories": {"Projeto-Baluarte": {"sha": "abc", "branch": "main"}}}
        current = {"Projeto-Baluarte": {"branch": "main", "error": "HTTP 409"}}
        self.assertTrue(MODULE.semantic_state_changed(state, current))



def commit(sha: str, message: str, date: str = "2026-09-20T10:00:00Z") -> list[dict]:
    return [{
        "sha": sha,
        "commit": {"committer": {"date": date}, "message": message},
        "html_url": f"https://github.com/o/r/commit/{sha}",
    }]


class EcosystemWatchScanTests(unittest.TestCase):
    """A varredura inteira contra um GitHub simulado, com --root e --now.

    --root e --now existem para o núcleo em Rust ser comparado com este
    script sobre a mesma cópia do estado (tests/e2e/github_parity.py); aqui
    fica o comportamento que essa comparação toma como referência.
    """

    A1, A2, B1 = "a" * 40, "b" * 40, "c" * 40

    def setUp(self) -> None:
        self.fake = FakeGitHub()
        self.fake.get("/users/o/repos?type=owner&per_page=100&page=1", [
            {"name": "alpha", "default_branch": "main"},
            {"name": "beta", "default_branch": "dev"},
            {"name": "fork1", "fork": True},
            {"name": "secret", "private": True},
            {"name": "O"},  # o próprio perfil, com outra caixa
            {"name": "empty"},
            {"name": "broken", "default_branch": None},
        ])
        self.fake.get("/repos/o/alpha/commits?sha=main&per_page=1", commit(self.A1, "feat: primeira\n\ncorpo"))
        self.fake.get("/repos/o/beta/commits?sha=dev&per_page=1", commit(self.B1, "docs: beta"))
        self.fake.get("/repos/o/empty/commits?sha=main&per_page=1", [])
        self.fake.get("/repos/o/broken/commits?sha=main&per_page=1", {"message": "Git Repository is empty."}, status=409)
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.saved = {name: getattr(MODULE, name) for name in ("API", "USER", "TOKEN", "PROFILE_REPOSITORY_NAME")}

    def tearDown(self) -> None:
        for name, value in self.saved.items():
            setattr(MODULE, name, value)
        self.tmp.cleanup()

    def scan(self, now: str) -> tuple[dict, str, str]:
        MODULE.API, MODULE.USER, MODULE.TOKEN, MODULE.PROFILE_REPOSITORY_NAME = self.fake.url, "o", "", "o"
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertIsNone(MODULE.main(["--root", str(self.root), "--now", now]))
        state = json.loads((self.root / "docs" / "ECOSYSTEM-COMMIT-STATE.json").read_text(encoding="utf-8"))
        report = (self.root / "docs" / "ECOSYSTEM-COMMIT-MONITOR.md").read_text(encoding="utf-8")
        return state, report, out.getvalue()

    def test_primeira_varredura_contagens_e_estados(self) -> None:
        with self.fake:
            state, report, _ = self.scan("2026-09-26T07:00:00Z")
        self.assertEqual("2026-09-26T07:00:00Z", state["scanned_at"])
        self.assertEqual(["alpha", "beta", "empty", "broken"], list(state["repositories"]))
        self.assertEqual({"branch": "main", "sha": self.A1, "date": "2026-09-20T10:00:00Z", "message": "feat: primeira",
                          "url": f"https://github.com/o/r/commit/{self.A1}"}, state["repositories"]["alpha"])
        self.assertEqual({"branch": "main", "empty": True}, state["repositories"]["empty"])
        self.assertEqual({"branch": "main", "error": "HTTP Error 409: Conflict"}, state["repositories"]["broken"])
        self.assertEqual({"tracked_commits": 1539, "project_commits": 1538, "monitor_commits": 1,
                          "detected_project_commits_this_scan": 0, "monitor_commit_this_scan": 1}, state["metrics"])
        self.assertEqual({"repositories_scanned": 4, "repositories_changed": 0, "errors": 1}, state["summary"])
        self.assertIn("**Última varredura:** `2026-09-26T07:00:00Z`", report)

    def test_commits_novos_somam_e_varredura_igual_nao_reescreve(self) -> None:
        with self.fake:
            self.scan("2026-09-26T07:00:00Z")
            self.fake.get("/repos/o/alpha/commits?sha=main&per_page=1", commit(self.A2, "fix: segunda"))
            self.fake.get(f"/repos/o/alpha/compare/{self.A1}...{self.A2}", {"ahead_by": 3})
            state, report, _ = self.scan("2026-09-26T08:00:00Z")
            self.assertEqual((1541, 2, 1543, 3), tuple(state["metrics"][key] for key in (
                "project_commits", "monitor_commits", "tracked_commits", "detected_project_commits_this_scan")))
            self.assertIn(f"- **alpha** — 3 commit(s) — [{self.A2[:12]}]", report)

            before = (self.root / "docs" / "ECOSYSTEM-COMMIT-STATE.json").read_bytes()
            _, _, printed = self.scan("2026-09-26T09:00:00Z")
        self.assertIn("Nenhuma mudança semântica", printed)
        self.assertEqual(before, (self.root / "docs" / "ECOSYSTEM-COMMIT-STATE.json").read_bytes())


if __name__ == "__main__":
    unittest.main()
