# Agent Navigation Rules

## Canonical repositories

- TaxForge: https://github.com/Lucas-Belucci-Bellini/taxforge.git
- Ark Initiative: https://github.com/Lucas-Belucci-Bellini/Ark-Initiative.git
- DailyPlanner: https://github.com/Lucas-Belucci-Bellini/DailyPlanner.git
- AEGIS: https://github.com/Lucas-Belucci-Bellini/AEGIS.git
- Projeto Baluarte: https://github.com/Lucas-Belucci-Bellini/Projeto-Baluarte.git
- Veritas: https://github.com/Lucas-Belucci-Bellini/Veritas.git

## Rules for local agents

1. Treat the URLs above as the canonical repository locations.
2. Do not assume that a repository is present on the local filesystem.
3. Do not create cross-repository dependencies without an explicit documented contract.
4. Before editing a project, inspect its current README, AGENTS.md/CLAUDE.md (if present), architecture documentation, branch and working state.
5. Run the project's relevant tests and validation commands before reporting a change as complete.
6. Keep project-specific implementation details in the project's own repository; this repository is primarily navigation/context.
7. Never put credentials, API keys, tokens or private connection strings in these documents.

## Ecosystem core (this repository)

This repository is evolving from a profile README into the ecosystem's catalog core
(GitHub → collector → PostgreSQL → README/JSON/API). Before changing scripts, workflows,
the README generator or the database, read:

- `docs/audits/2026-09-25-ecosystem-core-audit.md` — current state, known defects, invariants.
- `docs/DECISION-LOG.md` — accepted decisions are not re-litigated without new facts.
- `docs/migration/PYTHON-TO-RUST.md` and `docs/migration/MIGRATION-MATRIX.md` — what migrates, in which order.
- `docs/database/MIGRATIONS.md` — schema rules; run `db/tests/run.sh` after touching `db/`.

Rust core (`crates/`, toolchain pinned in `rust-toolchain.toml`): run `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`. Database
tests need `STORE_TEST_DATABASE_URL` (each test creates and drops its own database). If you change
a Python rule covered by `tests/test_parity_domain.py`, regenerate the fixture with the CI's Python
(`UPDATE_PARITY=1 python3.12 -m unittest tests.test_parity_domain`) and update the Rust port in
the same PR — the two sides are checked against the same file. The site-monitor fixture
(`tests/test_parity_site_monitor.py`) is tied to the CI's exact Python (3.12.14; `uv python
install 3.12.14`), because `urllib` changes within 3.12. `tests/e2e/check_sites_parity.py
target/release/profile-core` runs both site monitors against the same local server.

The collectors (`update_profile.py`, `ecosystem_watch.py`, `update_contribution_timeline.py`)
have Rust equivalents in shadow mode (`profile-core catalog build`, `sync commits`,
`sync contributions`). If you change one, regenerate its fixture with the CI's Python
(`tests/test_parity_monitor.py`, `tests/test_parity_contributions.py`; the catalog is checked
against the golden fixture) and port the change in the same PR. `tests/e2e/github_parity.py
target/release/profile-core` runs both sides against the same simulated GitHub
(`tests/fake_github.py`, selected through `GITHUB_API_URL`/`GITHUB_GRAPHQL_URL`) — no test
talks to the real GitHub.

Hard rules: never edit content between `<!-- NAME:START -->` / `<!-- NAME:END -->` markers in
`README.md` by hand; never edit an applied migration (add a new one); never commit a real
database, connection string or non-synthetic seed data.

## Owner-controlled synchronization

The owner may synchronize local copies manually with Git:

```bash
git pull <canonical-repository-url>
```

Agents should report the repository, branch, commit and files changed so the owner can synchronize and review the result.
