from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


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


if __name__ == "__main__":
    unittest.main()
