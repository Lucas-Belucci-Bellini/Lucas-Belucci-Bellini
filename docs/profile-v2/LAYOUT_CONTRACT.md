# Profile V2 — Layout Contract

Status: initial implementation contract  
Owner: layout workstream (Conversation 02/10)  
Branch: `feat/profile-v2-layout`

## Scope

This workstream owns the shared page shell, global navigation, footer, content container, responsive behavior, design tokens, and accessibility conventions. Feature sections (personal introduction, projects, statistics, and technologies) remain owned by their respective workstreams.

## Repository findings

The repository currently exposes a profile `README.md` with a dark, technical visual identity. During initial inspection, these expected frontend files were not found on the architecture reference branch: `package.json`, `src/App.tsx`, `src/main.tsx`, `index.html`, and `vite.config.ts`.

Do not assume a frontend framework or overwrite the README. Confirm the architecture workstream's intended delivery format and stack before adding framework-specific components.

## Visual direction

- Preserve the existing dark, technical identity as a reference: near-black/plum surfaces, warm gold accents, muted secondary text.
- Use semantic structure: `header`, `nav`, `main`, named sections, and `footer`.
- Keep a consistent centered content container with a readable maximum width and fluid side padding.
- Establish spacing, typography, color, border-radius, and focus-ring tokens centrally.
- Prefer restrained surfaces and clear hierarchy over decorative effects that reduce readability.

## Shared shell requirements

1. Header with a brand/home link and primary navigation.
2. Mobile navigation that is keyboard-operable, exposes its expanded state, and closes after navigation.
3. Main content slot that does not impose section-specific markup.
4. Footer with a small set of relevant links and copyright/profile identity.
5. Responsive layout that avoids horizontal overflow at narrow widths.
6. Visible focus indicators, sufficient text contrast, reduced-motion support, and meaningful accessible names.
7. Use native HTML and CSS where possible; add dependencies only when the confirmed stack requires them.

## Integration contract for other workstreams

- Feature sections must not redefine global tokens or the page container.
- Sections should use the shared container and spacing scale, with headings in a consistent hierarchy.
- Avoid editing the global shell, tokens, navigation, or footer from feature-specific branches without coordinating with this workstream and Conversation 01.
- Keep section styling scoped to the section/component instead of global selectors.
- Do not merge or deploy from this workstream without review and explicit coordination.

## Validation checklist

- [ ] Confirm the target frontend location and stack with Conversation 01.
- [ ] Implement the shell and tokens in the confirmed frontend.
- [ ] Test keyboard-only navigation and visible focus.
- [ ] Test mobile and desktop layouts, including a narrow viewport.
- [ ] Run the repository's actual build and test scripts after they exist.
- [ ] Document exact commands and results; never claim deployment without deployment evidence.
