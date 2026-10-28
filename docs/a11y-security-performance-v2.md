# Portfolio V2 — accessibility, security and performance review

**Scope:** frontend files on `feat/profile-readme-website-v2`  
**Review branch:** `feat/profile-v2-a11y-performance`  
**Issue:** #34  
**Limitations:** this was a source-level review of the HTML, CSS and JavaScript. No browser session, screen reader, Lighthouse run, or automated test runner was available during this pass. The changes below should be verified in a browser before merge.

## Findings and changes

### Fixed — overlapping live announcements

- **Evidence:** `#project-grid` and `#project-count` both announced dynamic updates. Every keystroke rebuilt the grid, potentially causing assistive technology to announce a large list of cards as well as the count.
- **Change:** removed the live-region behavior from the whole grid. The count is now a polite, atomic status region and explicitly reports when no projects match. The catalog error remains a separate status message.
- **Verification:** source inspection confirms only the concise count and error message are live status regions; runtime announcement behavior still needs screen-reader testing.

### Fixed — GitHub destinations accepted any HTTPS host

- **Evidence:** the previous URL helper only checked the HTTPS protocol, so a catalog entry placed in the `github` field could direct the “Código-fonte” link to any HTTPS domain.
- **Change:** GitHub repository URLs are now accepted only for `github.com` and `www.github.com`. Website links remain HTTPS-only and are rendered only when `website_status` is `verified`.
- **Verification:** traced both the catalog filter and card-link creation to the same host allowlist. Runtime cases for malformed, non-HTTPS, and unexpected-host values still need automated tests.

### Hardened — catalog data remains text, not HTML

- **Evidence:** project names, descriptions, labels and statuses originate in fetched JSON and are rendered into the DOM.
- **Existing protection retained:** all these values are assigned through `textContent` or text nodes created with `document.createElement`; no `innerHTML` or HTML-string parsing is used.
- **Change:** documented this invariant beside catalog filtering. No new HTML rendering path was introduced.
- **Residual risk:** this does not validate the truthfulness or quality of catalog content, and it is not a guarantee of complete application security.

### Fixed — decorative hero artwork had a label without image semantics

- **Evidence:** `.hero-visual` had `aria-label` but no role, so the label did not reliably provide an accessible name or useful semantic.
- **Change:** marked the decorative visual `aria-hidden="true"`; hid the decorative pulse/loading glyphs from assistive technology. The visual does not contain interactive controls.
- **Verification:** checked source semantics; screen-reader verification remains outstanding.

### Improved — external-link isolation and in-page navigation

- **Evidence:** links opening new tabs used `target="_blank"` with `rel="noreferrer"`; the brand links used an empty fragment.
- **Change:** made `rel="noopener noreferrer"` explicit for external new-tab links and pointed the brand/footer home links at `#main`.
- **Verification:** source inspection only; confirm keyboard navigation and destinations in a browser.

## Existing safeguards retained

- A skip link to the main content exists and becomes visible on focus.
- Keyboard focus indicators already cover links, buttons, inputs and selects.
- The stylesheet already includes a `prefers-reduced-motion: reduce` rule.
- The search input and category selector are wrapped in labels; the selector has a visually hidden label.
- The page uses landmarks (`header`, labelled `nav`, `main`, `section`, `footer`) and a document language of `pt-BR`.
- No runtime dependency was added. Google Fonts are loaded externally with `display=swap`; external font availability and third-party request/privacy implications remain operational considerations.

## Tests and checks

- [x] Inspected current `index.html`, `assets/site.css` and `assets/site.js` from the integration branch.
- [x] Confirmed the updated HTML and JavaScript were retrievable from the review branch after commits.
- [x] Source-reviewed DOM construction: no `innerHTML` use in `assets/site.js`.
- [x] Source-reviewed link policy, status-region placement, skip link, focus styles and reduced-motion rule.
- [ ] Browser keyboard-only pass and focus-order check.
- [ ] Screen-reader check for filtering, empty results and catalog-load failure.
- [ ] Automated tests for malicious-looking text, unsafe URLs and catalog failure cases.
- [ ] Lighthouse/performance measurement and mobile viewport smoke test.

## Risks remaining / recommendations

- Add focused tests for `safeUrl`, catalog shape failures, empty results and text-only rendering.
- Test focus visibility, sticky navigation, zoom/reflow, and announcements with at least one desktop screen reader.
- Measure the actual built artifact with Lighthouse before changing fonts or CSS. No performance regression was proven by source inspection, so no speculative redesign or dependency was added.
- URL allowlisting reduces accidental/malicious misdirection in the GitHub field; it is not a full allowlist for website destinations, which intentionally support verified external HTTPS sites.
- This review is not a security certification and cannot guarantee that the site is free of vulnerabilities.
