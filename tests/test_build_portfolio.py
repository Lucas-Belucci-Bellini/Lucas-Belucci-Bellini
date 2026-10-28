"""Regression tests for the public portfolio build pipeline."""
import json
import tempfile
import unittest
from pathlib import Path

from scripts.build_portfolio import BuildError, build, safe_https_url

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
    def test_https_url_policy_accepts_valid_urls_and_rejects_unsafe_inputs(self):
        valid = [
            "https://example.com",
            "https://docs.example.com/path?q=1#section",
            "https://github.com/owner/repo.with.dots",
            "https://www.github.com/owner/repo",
        ]
        invalid = [
            "", " https://example.com", "https://example.com ",
            "javascript:alert(1)", "data:text/html,hello",
            "//example.com/path", "http://example.com",
            "https://user:pass@example.com", "https://localhost",
            "https://service.localhost", "https://printer.local",
            "https://example.com\\\\@evil.example", "https://example.com/with space",
            "https://example.com:bad/",
        ]
        for value in valid:
            with self.subTest(value=value):
                self.assertTrue(safe_https_url(value))
        for value in invalid:
            with self.subTest(value=value):
                self.assertFalse(safe_https_url(value))

    def test_dotted_github_repository_is_not_removed_from_public_catalog(self):
        self.write_catalog([project(github="https://github.com/owner/repo.with.dots")])
        out = self.root / "dist"
        self.assertEqual(build(self.root, out), 1)
        payload = json.loads((out / "data" / "projects.json").read_text(encoding="utf-8"))
        self.assertEqual(payload["projects"][0]["github"], "https://github.com/owner/repo.with.dots")

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


if __name__ == "__main__":
    unittest.main()
