# Projects section — Profile V2

## Source of truth

The curated catalog lives in `data/projects.json`. Keep project content separate from presentation. UI components should consume these records rather than hard-code project copy.

## Selection criteria

- Include projects only when a repository or first-party project context can be verified.
- Prefer projects with a clear public repository and a useful summary.
- Never show private repositories in the public portfolio. GitSafe-AI was intentionally excluded because its repository is private.
- Do not publish stars, downloads, release versions, adoption counts, or completion claims unless fetched from a trustworthy source and refreshed at render time.
- Empty `technologies` means no technology list is asserted yet; do not render an empty badge row.
- Status labels are editorial and conservative: `in-development` means ongoing work, `experimental` means early-stage framework work, and `planned` should only be used for ideas without an implementation repository.

## Component contract

A reusable project card should display:
- `name` and `description` (fall back to “Descrição ainda não disponível.” if absent);
- optional `technologies` and `category`;
- a localized status label;
- an external repository link using `target="_blank"` and `rel="noopener noreferrer"`.

Only add category/technology filters when the final layout has enough projects to make them useful. Keep keyboard focus visible, preserve a logical heading order, and make the card link understandable without relying on color alone.

## Link verification

On 2026-10-27, GitHub repository metadata was successfully retrieved for the public repositories listed in the catalog. This confirms the repositories were accessible through the connected GitHub integration at the time of review; it does not guarantee future availability or validate every README claim. Recheck links during CI or before release.

## Integration and validation

1. Import `data/projects.json` in the projects section.
2. Render cards from the data with a stable `id` key.
3. Add a unit test for the empty-description fallback and optional technology list.
4. Add a link-format test requiring HTTPS and a GitHub repository URL.
5. Run the repository's existing validation commands; do not introduce a dependency just for this section.

## Current limitation

The repository is primarily a Rust profile/ecosystem catalog and README generator; this branch does not yet contain a dedicated frontend application in which to implement and run browser-level card tests. This change provides the verified data contract and integration guidance without altering generated README sections or unrelated systems.
