"""Integration checks for the built JSON contract consumed by the portfolio UI."""
import json
import tempfile
import unittest
from pathlib import Path

from scripts.build_portfolio import BuildError, build, public_projects

SCHEMA = "lucas-belucci-bellini/project-catalog@1"
PUBLIC_SCHEMA = "lucas-belucci-bellini/public-portfolio@1"
PUBLIC_FIELDS = {
    "name", "slug", "description", "category", "category_label",
    "status", "github", "website",
}


def project(**overrides):
    item = {
        "repository": "owner/demo",
        "name": "Demo",
        "slug": "demo",
        "description": "A public demo",
        "category": "Tools",
        "category_label": "Tools",
        "status": "🟢 Active",
        "github": "https://github.com/owner/demo",
        "private": False,
        "github_visible": True,
        "website_status": "verified",
        "website": "https://example.com",
    }
    item.update(overrides)
    return item


class PublicCatalogContractTests(unittest.TestCase):
    def catalog(self, projects):
        return {"schema": SCHEMA, "categories": ["Tools"], "projects": projects}

    def test_public_records_have_only_documented_fields(self):
        result = public_projects(self.catalog([project()]))
        self.assertEqual(len(result), 1)
        self.assertEqual(set(result[0]), PUBLIC_FIELDS)
        self.assertEqual(result[0]["website"], "https://example.com")

    def test_unverified_or_unsafe_website_is_omitted_not_published(self):
        result = public_projects(self.catalog([
            project(website_status="pending"),
            project(name="Unsafe", slug="unsafe", website="javascript:alert(1)"),
        ]))
        self.assertEqual([item["slug"] for item in result], ["Demo", "Unsafe"] if False else ["demo", "unsafe"])
        self.assertTrue(all("website" not in item for item in result))

    def test_missing_required_visibility_flag_fails_closed(self):
        item = project()
        del item["private"]
        with self.assertRaisesRegex(BuildError, r"private must be an explicit boolean"):
            public_projects(self.catalog([item]))

    def test_non_boolean_visibility_flag_is_not_treated_as_public(self):
        with self.assertRaisesRegex(BuildError, r"private must be an explicit boolean"):
            public_projects(self.catalog([project(private="false")]))

    def test_non_github_host_is_excluded_from_public_payload(self):
        result = public_projects(self.catalog([
            project(),
            project(name="Phishing", slug="phishing", github="https://github.com.evil.example/owner/demo"),
        ]))
        self.assertEqual([item["slug"] for item in result], ["demo"])

    def test_empty_catalog_after_filtering_is_an_explicit_error(self):
        with self.assertRaisesRegex(BuildError, "no valid public projects remain"):
            public_projects(self.catalog([project(private=True)]))


class BuiltArtifactContractTests(unittest.TestCase):
    def test_real_catalog_build_matches_frontend_payload_contract(self):
        repo_root = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "dist"
            build(repo_root, out)
            payload = json.loads((out / "data" / "projects.json").read_text(encoding="utf-8"))

        self.assertEqual(payload.get("schema"), PUBLIC_SCHEMA)
        self.assertIsInstance(payload.get("projects"), list)
        self.assertGreater(len(payload["projects"]), 0)
        self.assertEqual(payload.get("project_count"), len(payload["projects"]))
        slugs = [item["slug"] for item in payload["projects"]]
        self.assertEqual(len(slugs), len(set(slugs)))
        self.assertEqual(
            slugs,
            [item["slug"] for item in sorted(
                payload["projects"],
                key=lambda item: (item["name"].casefold(), item["slug"]),
            )],
        )
        for item in payload["projects"]:
            self.assertTrue(set(item).issubset(PUBLIC_FIELDS))
            self.assertTrue(item["name"].strip())
            self.assertTrue(item["slug"].strip())
            self.assertTrue(item["description"].strip())
            self.assertTrue(item["category"].strip())
            self.assertTrue(item["category_label"].strip())
            self.assertTrue(item["status"].strip())
            self.assertTrue(item["github"].startswith("https://github.com/"))
            if "website" in item:
                self.assertTrue(item["website"].startswith("https://"))


if __name__ == "__main__":
    unittest.main()
