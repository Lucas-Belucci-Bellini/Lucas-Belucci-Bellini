# Ecosystem Index

This repository is the profile/navigation repository for the Lucas Belucci Bellini ecosystem.

## Official repositories

| Project | Repository | Primary role |
|---|---|---|
| TaxForge | https://github.com/Lucas-Belucci-Bellini/taxforge.git | Tax/reform intelligence platform |
| Ark Initiative | https://github.com/Lucas-Belucci-Bellini/Ark-Initiative.git | Environmental / resilience initiative |
| DailyPlanner | https://github.com/Lucas-Belucci-Bellini/DailyPlanner.git | Planning and productivity system |
| AEGIS | https://github.com/Lucas-Belucci-Bellini/AEGIS.git | Ocean mapping, acoustic perception and scientific reconstruction |
| Projeto Baluarte | https://github.com/Lucas-Belucci-Bellini/Projeto-Baluarte.git | Ecosystem hub and orchestration/context layer |
| Veritas | https://github.com/Lucas-Belucci-Bellini/Veritas.git | Verification, logic and evidence-oriented system |

## Local workflow

Repositories are intentionally maintained locally by the owner. Agents should use these links to identify the canonical repository, but must not assume access to the owner's local filesystem.

```text
GitHub repository
    ↓
git pull
    ↓
local working copy
    ↓
Claude / Codex / other development agent
    ↓
changes + tests
    ↓
commit / push / PR
    ↓
GitHub
```

For the Baluarte repository:

```bash
git pull https://github.com/Lucas-Belucci-Bellini/Projeto-Baluarte.git
```

## Architecture rule

The existence of these repositories does not imply that they share a database, credentials, deployment, or runtime process. Cross-project integration must happen through explicit, documented contracts.

## Agent rule

Before changing another project, read that project's own README, agent instructions, architecture documents and current branch state. Do not infer implementation details from this index alone.
