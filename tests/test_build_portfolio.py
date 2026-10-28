"""Contract tests for the static portfolio builder (stdlib unittest only)."""
from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "build_portfolio", ROOT / "scripts" / "build_portfolio.py"
)
assert SPEC and SPEC.loader
BUILDER = importlib.util.module_from_spec(SPEC)
sys.modules["build_portfolio"] = BUILDER
SPEC.loader.exec_module(BUILDER)

PUBLIC_FIELDS = {
    "name", "description", "category", "category_label", "status",
    "github", "github_visible", "website_status", "website",
}
EXPECTED_DIST_FILES = {
    "index.html",
    "assets/site.css",
    "assets/site.js",
    "assets/favicon.svg",
    "data/projects.json",
    ".nojekyll",
}


class BuildPortfolioTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.dist = self.root / "dist"
        self.catalog = self.root / "docs" / "project-catalog.json"
        (self.root / "docs").mkdir()
        (self.root / "assets").mkdir()
        (self.root / "index.html").write_text("<main>portfolio</main>", encoding="utf-8")
        for name in ("site.css", "site.js", "favicon.svg"):
            (self.root / "assets" / name).write_text("fixture", encoding="utf-8")
        self.write_catalog([
            {
                "name": "Public",
                "description": "Public project",
                "category": "Web",
                "github": "https://github.com/example/public",
                "github_visible": True,
                "website": "https://example.com",
                "website_status": "verified",
                "private": False,
                "internal_notes": "must never ship",
            },
            {
                "name": "Private project",
                "github": "https://github.com/example/private",
                "github_visible": True,
                "private": True,
            },
            {
                "name": "Hidden project",
                "github": "https://github.com/example/hidden",
                "github_visible": False,
                "private": False,
            },
            {
                "name": "Invalid URL",
                "github": "javascript:alert(1)",
                "github_visible": True,
                "private": False,
            },
        ])

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_catalog(self, projects: object) -> None:
        self.catalog.write_text(
            json.dumps({"schema": "fixture", "projects": projects}),
            encoding="utf-8",
        )

    def run_builder(self) -> int:
        with (
            patch.object(BUILDER, "ROOT", self.root),
            patch.object(BUILDER, "DIST", self.dist),
            patch.object(BUILDER, "CATALOG", self.catalog),
        ):
            return BUILDER.main()

    def test_build_generates_expected_files_and_public_json(self) -> None:
        self.assertEqual(0, self.run_builder())
        actual = {
            p.relative_to(self.dist).as_posix()
            for p in self.dist.rglob("*")
            if p.is_file()
        }
        self.assertEqual(EXPECTED_DIST_FILES, actual)
        payload = json.loads((self.dist / "data" / "projects.json").read_text(encoding="utf-8"))
        self.assertEqual("lucas-belucci-bellini/public-portfolio@1", payload["schema"])
        self.assertEqual(1, payload["project_count"])
        self.assertEqual(["Public"], [p["name"] for p in payload["projects"]])
        self.assertEqual(PUBLIC_FIELDS.intersection({"name", "description", "category", "github", "github_visible", "website", "website_status"}), set(payload["projects"][0]))
        self.assertNotIn("internal_notes", (self.dist / "data" / "projects.json").read_text(encoding="utf-8"))
        self.assertNotIn("private", payload["projects"][0])

    def test_private_and_hidden_projects_are_excluded(self) -> None:
        self.run_builder()
        payload = json.loads((self.dist / "data" / "projects.json").read_text(encoding="utf-8"))
        names = {project["name"] for project in payload["projects"]}
        self.assertNotIn("Private project", names)
        self.assertNotIn("Hidden project", names)
        self.assertNotIn("Invalid URL", names)

    def test_public_field_allowlist_drops_unapproved_metadata(self) -> None:
        self.run_builder()
        payload = json.loads((self.dist / "data" / "projects.json").read_text(encoding="utf-8"))
        for project in payload["projects"]:
            self.assertLessEqual(set(project), PUBLIC_FIELDS)

    def test_invalid_url_schemes_are_not_published(self) -> None:
        self.write_catalog([
            {"name": "HTTP", "github": "http://github.com/example/a"},
            {"name": "FTP", "github": "ftp://github.com/example/a"},
            {"name": "No host", "github": "https://"},
            {"name": "Valid", "github": "https://github.com/example/valid"},
        ])
        self.run_builder()
        payload = json.loads((self.dist / "data" / "projects.json").read_text(encoding="utf-8"))
        self.assertEqual(["Valid"], [project["name"] for project in payload["projects"]])

    def test_malformed_json_fails_without_claiming_success(self) -> None:
        self.catalog.write_text('{"projects": [', encoding="utf-8")
        with self.assertRaises(json.JSONDecodeError):
            self.run_builder()

    def test_catalog_without_projects_array_is_rejected(self) -> None:
        self.catalog.write_text('{"projects": {}}', encoding="utf-8")
        with self.assertRaisesRegex(SystemExit, "projects must be an array"):
            self.run_builder()

    def test_empty_public_catalog_is_rejected(self) -> None:
        self.write_catalog([
            {"name": "Private", "private": True, "github": "https://github.com/example/a"},
        ])
        with self.assertRaisesRegex(SystemExit, "empty public project catalog"):
            self.run_builder()

    def test_missing_catalog_is_rejected(self) -> None:
        self.catalog.unlink()
        with self.assertRaisesRegex(SystemExit, "Catalog not found"):
            self.run_builder()


if __name__ == "__main__":
    unittest.main()
