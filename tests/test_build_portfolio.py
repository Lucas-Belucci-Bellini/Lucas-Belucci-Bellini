"""Regression tests for the public portfolio build pipeline."""
import json
import tempfile
import unittest
from pathlib import Path

from scripts.build_portfolio import BuildError, build

SCHEMA = "lucas-belucci-bellini/project-catalog@1"


def project(**overrides):
    item = {
        "repository": "owner/public-demo",
        "name": "Public Demo",
        "slug": "public-demo",
        "description": "A public project.",
        "category": "Software & Tools",
        "category_label": "Tools",
        "status": "🟢 Active",
        "github": "https://github.com/owner/public-demo",
        "website": "https://example.com",
        "github_visible": True,
        "private": False,
        "marketing_priority": 99,
        "website_status": "verified",
    }
    item.update(overrides)
    return item


class PortfolioBuildTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name) / "repo"
        self.root.mkdir()
        for relative in (
            "index.html",
            "assets/site.css",
            "assets/site.js",
            "assets/favicon.svg",
        ):
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("static asset\n", encoding="utf-8")

    def tearDown(self):
        self.temp.cleanup()

    def write_catalog(self, projects, schema=SCHEMA):
        catalog = {
            "schema": schema,
            "categories": ["Software & Tools"],
            "projects": projects,
        }
        path = self.root / "docs" / "project-catalog.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(
            json.dumps(catalog, ensure_ascii=False),
            encoding="utf-8",
        )

    def test_public_payload_is_allowlisted_and_private_projects_are_excluded(self):
        self.write_catalog([
            project(),
            project(name="Private", slug="private", private=True),
            project(name="Not visible", slug="not-visible", github_visible=False),
            project(name="Bad website", slug="bad-website", website="javascript:alert(1)"),
            project(name="Bad GitHub", slug="bad-github", github="javascript:alert(1)"),
        ])
        out = self.root / "dist"

        count = build(self.root, out)
        payload = json.loads((out / "data" / "projects.json").read_text(encoding="utf-8"))

        self.assertEqual(count, 2)
        self.assertEqual(payload["project_count"], 2)
        self.assertEqual(
            {item["slug"] for item in payload["projects"]},
            {"public-demo", "bad-website"},
        )
        for item in payload["projects"]:
            self.assertNotIn("repository", item)
            self.assertNotIn("private", item)
            self.assertNotIn("github_visible", item)
            self.assertNotIn("marketing_priority", item)
            self.assertNotIn("website_status", item)
        bad_website = next(item for item in payload["projects"] if item["slug"] == "bad-website")
        self.assertNotIn("website", bad_website)

    def test_build_is_deterministic_and_replaces_stale_output(self):
        self.write_catalog([project()])
        out = self.root / "dist"
        out.mkdir()
        (out / "stale-secret.txt").write_text("must disappear", encoding="utf-8")

        build(self.root, out)
        first = {p.relative_to(out).as_posix(): p.read_bytes() for p in out.rglob("*") if p.is_file()}
        self.assertNotIn("stale-secret.txt", first)

        build(self.root, out)
        second = {p.relative_to(out).as_posix(): p.read_bytes() for p in out.rglob("*") if p.is_file()}
        self.assertEqual(first, second)
        self.assertEqual(
            set(second),
            {
                ".nojekyll",
                "index.html",
                "assets/site.css",
                "assets/site.js",
                "assets/favicon.svg",
                "data/projects.json",
            },
        )

    def test_output_cannot_replace_source_assets(self):
        self.write_catalog([project()])
        asset = self.root / "assets" / "site.css"
        before = asset.read_text(encoding="utf-8")

        with self.assertRaisesRegex(BuildError, "restricted to the dist"):
            build(self.root, self.root / "assets")

        self.assertEqual(asset.read_text(encoding="utf-8"), before)

    def test_unsupported_schema_fails_explicitly(self):
        self.write_catalog([project()], schema="unexpected/schema@9")
        with self.assertRaisesRegex(BuildError, "unsupported catalog schema"):
            build(self.root, self.root / "dist")

    def test_duplicate_public_slugs_fail_explicitly(self):
        self.write_catalog([project(), project(name="Second", slug="public-demo")])
        with self.assertRaisesRegex(BuildError, "duplicate public slug"):
            build(self.root, self.root / "dist")


class DesignSystemContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.repo_root = Path(__file__).resolve().parents[1]
        cls.css = (cls.repo_root / "assets" / "site.css").read_text(encoding="utf-8")
        cls.html = (cls.repo_root / "index.html").read_text(encoding="utf-8")
        cls.js = (cls.repo_root / "assets" / "site.js").read_text(encoding="utf-8")

    def test_design_tokens_have_one_canonical_root(self):
        import re
        self.assertEqual(len(re.findall(r":root\\s*\\{", self.css)), 1)
        for token in (
            "--color-canvas", "--color-surface", "--color-text",
            "--color-gold", "--color-focus", "--font-body", "--font-code",
            "--space-4", "--radius-sm", "--motion-normal",
        ):
            with self.subTest(token=token):
                self.assertIn(token, self.css)

    def test_responsive_breakpoints_and_medium_navigation_exist(self):
        import re
        self.assertRegex(self.css, r"@media\\s*\\(max-width:\\s*850px\\)")
        self.assertRegex(self.css, r"@media\\s*\\(max-width:\\s*600px\\)")
        self.assertIn("scrollbar-width:thin", self.css)
        self.assertIn("grid-template-columns:repeat(2,minmax(0,1fr))", self.css)
        self.assertIn("grid-template-columns:1fr", self.css)

    def test_keyboard_focus_and_motion_preferences_are_preserved(self):
        self.assertIn(":focus-visible", self.css)
        self.assertIn(":focus-within", self.css)
        self.assertIn("@media(prefers-reduced-motion:reduce)", self.css)
        self.assertIn("@media (forced-colors: active)", self.css)

    def test_javascript_consumed_html_ids_remain_present(self):
        import re
        for element_id in (
            "project-search", "category-filter", "status-filter",
            "project-count", "project-grid", "catalog-message", "current-year",
        ):
            with self.subTest(element_id=element_id):
                self.assertRegex(self.html, rf'id="{re.escape(element_id)}"')
                self.assertIn(f'"#{element_id}"', self.js)

    def test_no_new_runtime_dependency_is_added(self):
        self.assertNotIn("npm install", self.html)
        self.assertNotIn("cdn.jsdelivr.net", self.html)
        self.assertNotIn("unpkg.com", self.html)


if __name__ == "__main__":
    unittest.main()
