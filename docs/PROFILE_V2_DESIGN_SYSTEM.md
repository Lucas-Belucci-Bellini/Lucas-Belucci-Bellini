# V2 Design System — Field Manual

This layer extends the existing dark/gold Baluarte identity; it does not replace the page structure or project-catalog behavior.

## Tokens

Defined in `assets/site.css` under `:root`:

- **Color roles:** `--color-canvas`, `--color-surface`, `--color-surface-raised`, `--color-border`, `--color-text`, `--color-text-muted`, `--color-gold`, `--color-gold-bright`, `--color-success`, `--color-warning`, and `--color-danger`.
- **Compatibility aliases:** `--bg`, `--panel`, `--panel-2`, `--line`, `--text`, `--muted`, `--gold`, `--gold-light`, and `--green` remain available for current components.
- **Typography:** `--font-body` (Manrope), `--font-code` (DM Mono), and `--text-xs/sm/md`.
- **Spacing:** `--space-1` through `--space-12`, using a 4px-based rhythm.
- **Shape and effects:** `--radius-sm/md/pill`, `--shadow-panel`, `--focus-ring`, and `--motion-fast/normal`.

## Component patterns

- **Actions:** use `.button` with `.button-primary` or `.button-quiet`; interactive targets should be at least 44px high where practical.
- **Surfaces:** use `.project-card` and `.stack-card`; use border changes and restrained elevation for hover, not a new visual theme.
- **Inputs:** `.search-box` and `.filter-label` share a border and a visible focus-within ring.
- **Catalog states:** `.loading-card` is the loading state, `.empty-card` represents empty/error fallback content, and `.catalog-message` carries the explanatory error status. The existing JavaScript owns which state appears.
- **Keyboard:** links and controls have visible focus outlines; the skip link remains available.
- **Motion:** transitions and loading animation are minimized when `prefers-reduced-motion: reduce` is enabled. Forced-colors mode receives explicit borders/focus treatment.

## Responsive behavior

The existing breakpoints at 850px and 600px are retained. The project grid moves from three columns to two and then one; navigation remains horizontally scrollable on small screens; project search and category filter stack on mobile; action links can wrap rather than overflow.

## Verification notes

The CSS changes are additive and do not alter `assets/site.js`, catalog data, README generation, or workflows. The repository connector can update GitHub files, but it does not provide a browser viewport or screenshot runner in this workflow. Before merging, verify at 375px, 768px, and 1440px widths and test keyboard navigation, empty search results, catalog-load failure, and reduced-motion in a browser. Also run the repository's existing test/build commands locally or in CI; no visual pass is claimed by this document.
