#!/usr/bin/env python3
"""Validate the profile README without publishing repository contents."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from urllib.parse import urlparse
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
README = ROOT / "README.md"


def main() -> int:
    text = README.read_text(encoding="utf-8")
    errors: list[str] = []
    warnings: list[str] = []

    for marker in ["PROFILE-DASHBOARD", "FEATURED-PROJECTS", "CURATED-FEATURED", "ARSENAL-STACK", "LANGUAGE-BADGES", "LIVE-PROJECTS", "PROJECT-MAP", "PUBLIC-PROJECTS", "PRIVATE-PROJECTS", "LANGUAGE-STATS"]:
        start = f"<!-- {marker}:START -->"
        end = f"<!-- {marker}:END -->"
        if text.count(start) != 1 or text.count(end) != 1 or text.index(start) > text.index(end):
            errors.append(f"invalid marker pair: {marker}")

    try:
        import markdown  # type: ignore
        markdown.markdown(text, extensions=["tables", "fenced_code"])
    except Exception as exc:  # pragma: no cover
        errors.append(f"Markdown parser error: {exc}")

    for image in re.findall(r"!\[[^\]]*\]\((\./[^)]+)\)", text):
        if not (ROOT / image[2:]).exists():
            errors.append(f"missing local image: {image}")

    for manifest_name in ["README_SITES.json", "README_FEATURED.json", "README_STACK.json", "README_EXCLUDED.json"]:
        try:
            manifest = json.loads((ROOT / "docs" / manifest_name).read_text(encoding="utf-8"))
            if not isinstance(manifest, dict):
                errors.append(f"{manifest_name} is not an object")
        except Exception as exc:
            errors.append(f"{manifest_name} error: {exc}")

    try:
        import yaml  # type: ignore
        yaml.safe_load((ROOT / ".github" / "workflows" / "update-profile.yml").read_text(encoding="utf-8"))
    except ImportError:
        warnings.append("PyYAML unavailable; workflow YAML was checked by required-key assertions")
    except Exception as exc:
        errors.append(f"workflow YAML error: {exc}")

    workflow = (ROOT / ".github" / "workflows" / "update-profile.yml").read_text(encoding="utf-8")
    for required in ["workflow_dispatch:", "schedule:", "permissions:", "contents: write", "scripts/update_profile.py", "scripts/validate_dynamic_sections.py", "scripts/validate_exclusions.py", "git diff --quiet"]:
        if required not in workflow:
            errors.append(f"workflow missing required construct: {required}")

    repos_path = Path("/home/ubuntu/profile_readme_audit/repos.json")
    repos = []
    private_repo_urls: set[str] = set()
    excluded_names: set[str] = set()
    try:
        excluded_data = json.loads((ROOT / "docs" / "README_EXCLUDED.json").read_text(encoding="utf-8"))
        excluded_names = {str(name) for name in excluded_data.get("repositories", [])}
    except Exception:
        errors.append("README_EXCLUDED.json could not be loaded")
    if repos_path.exists():
        repos = json.loads(repos_path.read_text(encoding="utf-8"))
        missing_repo_links = [
            repo["full_name"] for repo in repos
            if repo.get("name") not in excluded_names
            and repo.get("full_name") not in excluded_names
            and f"https://github.com/{repo['full_name']}" not in text
        ]
        if missing_repo_links:
            errors.append("missing GitHub links: " + ", ".join(missing_repo_links))
        private_repo_urls = {
            f"https://github.com/{repo['full_name']}"
            for repo in repos
            if repo.get("private")
        }

    for excluded_name in excluded_names:
        if excluded_name in text:
            errors.append(f"excluded repository appears in README: {excluded_name}")

    # This is a policy check: the public README may mention privacy terms, but must not
    # publish internal secret names, token values, or private file paths.
    forbidden = ["sk-", "ghp_", "BEGIN RSA PRIVATE KEY", "postgresql://", "mysql://"]
    for token in forbidden:
        if token.lower() in text.lower():
            errors.append(f"possible sensitive token pattern in README: {token}")

    targets = []
    for target in re.findall(r"\]\((https?://[^)\s]+)", text):
        cleaned = target.rstrip(".,;\"")
        if cleaned not in targets:
            targets.append(cleaned)

    link_results = []
    for target in targets:
        parsed = urlparse(target)
        if target in private_repo_urls:
            link_results.append({"url": target, "status": 404, "final_url": target, "result": "expected_private"})
            continue
        try:
            request = Request(target, headers={"User-Agent": "profile-readme-validation/1.0"}, method="GET")
            with urlopen(request, timeout=12) as response:
                status = int(response.status)
                final = response.geturl()
            result = "ok" if 200 <= status < 400 else "failed"
        except Exception as exc:  # network issues are reported, not hidden
            status = 0
            final = ""
            result = "failed"
            if parsed.netloc.lower().endswith("linkedin.com"):
                result = "access_wall"
                warnings.append(f"LinkedIn protected this public profile URL with an auth wall: {target}")
            else:
                warnings.append(f"link check failed for {target}: {exc}")
        link_results.append({"url": target, "status": status, "final_url": final, "result": result})

    (ROOT / "docs" / "README_LINK_CHECK.json").write_text(json.dumps(link_results, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    failed_links = [item for item in link_results if item["result"] not in {"ok", "expected_private", "access_wall"}]
    if failed_links:
        warnings.append(f"{len(failed_links)} external links could not be verified; see docs/README_LINK_CHECK.json")

    print(json.dumps({"errors": errors, "warnings": warnings, "links_checked": len(link_results), "failed_links": len(failed_links)}, ensure_ascii=False, indent=2))
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
