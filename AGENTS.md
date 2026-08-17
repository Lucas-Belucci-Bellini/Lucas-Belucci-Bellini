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

## Owner-controlled synchronization

The owner may synchronize local copies manually with Git:

```bash
git pull <canonical-repository-url>
```

Agents should report the repository, branch, commit and files changed so the owner can synchronize and review the result.
