import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CSS = (ROOT / "assets" / "site.css").read_text(encoding="utf-8")
HTML = (ROOT / "index.html").read_text(encoding="utf-8")
JS = (ROOT / "assets" / "site.js").read_text(encoding="utf-8")


class DesignSystemContractTests(unittest.TestCase):
    def test_design_tokens_have_one_canonical_root(self):
        self.assertEqual(len(re.findall(r":root\s*\{", CSS)), 1)
        for token in (
            "--color-canvas", "--color-surface", "--color-text",
            "--color-gold", "--color-focus", "--font-body", "--font-code",
            "--space-4", "--radius-sm", "--motion-normal",
        ):
            with self.subTest(token=token):
                self.assertIn(token, CSS)

    def test_responsive_breakpoints_and_medium_navigation_exist(self):
        self.assertRegex(CSS, r"@media\s*\(max-width:\s*850px\)")
        self.assertRegex(CSS, r"@media\s*\(max-width:\s*600px\)")
        self.assertIn("scrollbar-width:thin", CSS)
        self.assertIn("grid-template-columns:repeat(2,minmax(0,1fr))", CSS)
        self.assertIn("grid-template-columns:1fr", CSS)

    def test_keyboard_focus_and_motion_preferences_are_preserved(self):
        self.assertIn(":focus-visible", CSS)
        self.assertIn(":focus-within", CSS)
        self.assertIn("@media(prefers-reduced-motion:reduce)", CSS)
        self.assertIn("@media (forced-colors: active)", CSS)

    def test_javascript_consumed_html_ids_remain_present(self):
        for element_id in (
            "project-search", "category-filter", "status-filter",
            "project-count", "project-grid", "catalog-message", "current-year",
        ):
            with self.subTest(element_id=element_id):
                self.assertRegex(HTML, rf'id="{re.escape(element_id)}"')
                self.assertIn(f'"#{element_id}"', JS)

    def test_no_new_runtime_dependency_is_added(self):
        self.assertNotIn("npm install", HTML)
        self.assertNotIn("cdn.jsdelivr.net", HTML)
        self.assertNotIn("unpkg.com", HTML)


if __name__ == "__main__":
    unittest.main()
