import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "update_profile.py"
SPEC = importlib.util.spec_from_file_location("update_profile", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class LanguageBadgeTests(unittest.TestCase):
    def test_special_language_names_keep_readable_markdown_labels(self) -> None:
        rows = [
            {"language": "C#", "display": "C#", "repositories": 2, "bytes": 200, "share": 60.0},
            {"language": "PLpgSQL", "display": "PL/pgSQL", "repositories": 2, "bytes": 100, "share": 30.0},
            {"language": "Rust", "display": "Rust", "repositories": 1, "bytes": 33, "share": 10.0},
        ]

        rendered = MODULE.render_language_badges(rows)

        self.assertIn("[![C#]", rendered)
        self.assertIn("[![PL/pgSQL]", rendered)
        self.assertNotIn("[![C%23]", rendered)
        self.assertNotIn("[![PL%2FpgSQL]", rendered)
        self.assertIn("/badge/C%23-2%20repos-", rendered)
        self.assertIn("/badge/PL%2FpgSQL-2%20repos-", rendered)
        self.assertIn("language=PLpgSQL", rendered)


if __name__ == "__main__":
    unittest.main()

