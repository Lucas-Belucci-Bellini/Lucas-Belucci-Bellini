# Profile README Changelog

## 2026-08-26 — Profile ecosystem overhaul

The profile README was rebuilt as a central portfolio hub. The update audits all repositories visible to the authenticated GitHub account, separates public and private projects, groups projects by domain, adds the Baluarte ecosystem map, gives Veritas and Digital Logic their own sections, and records only deployments verified during the audit.

The stale manual language list was replaced by a generated public-only language table and a visual distribution asset. A dashboard now reports repository, language, activity, visibility, deployment, and academic counts from the current inventory.

The refresh workflow was added at `.github/workflows/update-profile.yml`, with the deterministic generator in `scripts/update_profile.py` and the data contract in `docs/README_DATA.md`. The workflow is daily and manual-triggered; it commits only when generated content changes.

A backup branch named `backup/before-profile-readme-overhaul` was created before the feature branch `feature/profile-readme-overhaul`. The intended delivery path is feature branch, validation, pull request, CI, merge, and feature-branch deletion.

## Maintenance policy

Future profile changes should update the generator or its data contract rather than manually editing generated blocks. Each significant change should add a dated entry here and preserve the backup/branch/validation flow described in `docs/README_DATA.md`.
