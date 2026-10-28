"""Regression and negative tests for the public portfolio build pipeline."""
import json
import tempfile
import unittest
from pathlib import Path

from scripts.build_portfolio import BuildError, build, public_projects, safe_https_url

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
        for relative in ("index.html", "assets/site.css", "assets/site.js", "assets/favicon.svg"):
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("static asset\n", encoding="utf-8")

    def tearDown(self):
        self.temp.cleanup()

    def write_catalog(self, projects, schema=SCHEMA, **root_overrides):
        catalog = {"schema": schema, "categories": ["Software & Tools"], "projects": projects}
        catalog.update(root_overrides)
        path = self.root / "docs" / "project-catalog.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(catalog, ensure_ascii=False), encoding="utf-8")

    def test_public_payload_is_allowlisted_and_private_projects_are_excluded(self):
        self.write_catalog([
            project(), project(name="Private", slug="private", private=True),
            project(name="Not visible", slug="not-visible", github_visible=False),
            project(name="Bad website", slug="bad-website", website="javascript:alert(1)"),
            project(name="Bad GitHub", slug="bad-github", github="javascript:alert(1)"),
        ])
        out = self.root / "dist"
        self.assertEqual(build(self.root, out), 2)
        payload = json.loads((out / "data" / "projects.json").read_text(encoding="utf-8"))
        self.assertEqual(payload["project_count"], 2)
        self.assertEqual({item["slug"] for item in payload["projects"]}, {"public-demo", "bad-website"})
        for item in payload["projects"]:
            for forbidden in ("repository", "private", "github_visible", "marketing_priority", "website_status"):
                self.assertNotIn(forbidden, item)
        self.assertNotIn("website", next(x for x in payload["projects"] if x["slug"] == "bad-website"))

    def test_build_is_deterministic_and_replaces_stale_output(self):
        self.write_catalog([project()])
        out = self.root / "dist"
        out.mkdir()
        (out / "stale-secret.txt").write_text("must disappear", encoding="utf-8")
        build(self.root, out)
        first = {p.relative_to(out).as_posix(): p.read_bytes() for p in out.rglob("*") if p.is_file()}
        build(self.root, out)
        second = {p.relative_to(out).as_posix(): p.read_bytes() for p in out.rglob("*") if p.is_file()}
        self.assertEqual(first, second)
        self.assertEqual(set(second), {".nojekyll", "index.html", "assets/site.css", "assets/site.js",
                                       "assets/favicon.svg", "data/projects.json"})

    def test_output_must_be_exact_dist_and_cannot_replace_source_assets(self):
        self.write_catalog([project()])
        before = (self.root / "assets/site.css").read_text(encoding="utf-8")
        for target in (self.root / "assets", self.root / "elsewhere", Path(self.temp.name) / "external"):
            with self.subTest(target=target), self.assertRaisesRegex(BuildError, "exactly the repository dist"):
                build(self.root, target)
        self.assertEqual((self.root / "assets/site.css").read_text(encoding="utf-8"), before)

    def test_unsupported_schema_and_root_fields_fail(self):
        self.write_catalog([project()], schema="unexpected/schema@9")
        with self.assertRaisesRegex(BuildError, "unsupported catalog schema"):
            build(self.root, self.root / "dist")
        self.write_catalog([project()], debug_dump="raw catalog")
        with self.assertRaisesRegex(BuildError, "unsupported root fields"):
            build(self.root, self.root / "dist")

    def test_missing_required_data_fails(self):
        for bad in ([], [project(name=" ")], [{**project(), "private": None}],
                    [{k: v for k, v in project().items() if k != "description"}]):
            with self.subTest(bad=bad), self.assertRaises(BuildError):
                public_projects({"schema": SCHEMA, "categories": ["Software & Tools"], "projects": bad})

    def test_duplicate_or_unsafe_slugs_fail(self):
        with self.assertRaisesRegex(BuildError, "duplicate public slug"):
            public_projects({"schema": SCHEMA, "categories": ["Software & Tools"],
                             "projects": [project(), project(name="Second")]})
        for slug in ("../secret", "UPPER", "a//b", "-bad", "trailing-"):
            with self.subTest(slug=slug), self.assertRaisesRegex(BuildError, "safe slug"):
                public_projects({"schema": SCHEMA, "categories": ["Software & Tools"],
                                 "projects": [project(slug=slug)]})

    def test_invalid_urls_and_non_github_destinations_are_rejected(self):
        for url in ("http://github.com/owner/repo", "javascript:alert(1)", "https://user:pass@example.com",
                    "https://localhost/a", "https://127.0.0.1/a", "https://192.168.1.1/a",
                    "https://example.com/a b", "https://example.com:444", "https://github.com"):
            with self.subTest(url=url):
                self.assertFalse(safe_https_url(url))
        for url in ("https://evil.example/owner/repo", "https://github.com.evil.example/owner/repo"):
            with self.subTest(url=url):
                projects = public_projects({"schema": SCHEMA, "categories": ["Software & Tools"],
                                            "projects": [project(github=url)]})
                self.assertEqual(projects, [])

    def test_unverified_or_invalid_website_never_published(self):
        for status, url in (("unreachable", "https://example.com"), ("verified", "javascript:alert(1)"),
                            ("verified", "https://10.0.0.1")):
            with self.subTest(status=status, url=url):
                result = public_projects({"schema": SCHEMA, "categories": ["Software & Tools"],
                                          "projects": [project(website_status=status, website=url)]})
                self.assertNotIn("website", result[0])

    def test_required_static_file_missing_or_symlink_is_rejected(self):
        self.write_catalog([project()])
        (self.root / "assets/site.js").unlink()
        with self.assertRaisesRegex(BuildError, "required static file"):
            build(self.root, self.root / "dist")

    def test_unexpected_source_catalog_is_not_copied_to_artifact(self):
        self.write_catalog([project()])
        (self.root / "docs/private-catalog.json").write_text('{"secret":"no"}', encoding="utf-8")
        build(self.root, self.root / "dist")
        files = {p.relative_to(self.root / "dist").as_posix() for p in (self.root / "dist").rglob("*") if p.is_file()}
        self.assertNotIn("docs/project-catalog.json", files)
        self.assertNotIn("docs/private-catalog.json", files)


if __name__ == "__main__":
    unittest.main()
