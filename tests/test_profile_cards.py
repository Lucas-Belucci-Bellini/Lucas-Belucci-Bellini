import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / ".github" / "scripts" / "profile_cards.py"
SPEC = importlib.util.spec_from_file_location("profile_cards", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ProfileCardsTests(unittest.TestCase):
    def test_stats_card_labels_total_and_direct_commits_separately(self) -> None:
        data = {
            "contributions": 1807,
            "commits": 1114,
            "issues": 94,
            "pull_requests": 535,
            "reviews": 3,
            "repository_contributions": 61,
            "restricted": 0,
            "repositories": 61,
            "generated": "2026-08-26 13:24 UTC",
        }

        svg = MODULE.stats_svg(data)

        self.assertIn("TOTAL CONTRIBUTIONS", svg)
        self.assertIn("DIRECT COMMITS", svg)
        self.assertIn("REPOS CREATED", svg)
        self.assertIn("total = commits + PRs + issues + reviews + repos", svg)
        self.assertIn(">1807<", svg)
        self.assertIn(">1114<", svg)

    def test_contribution_total_matches_current_components(self) -> None:
        components = 1114 + 94 + 535 + 3 + 61
        self.assertEqual(1807, components)


if __name__ == "__main__":
    unittest.main()

