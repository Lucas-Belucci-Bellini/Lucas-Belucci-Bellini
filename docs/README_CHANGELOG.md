# Profile README Changelog

## 2026-08-26 — Profile ecosystem overhaul

The profile README was rebuilt as a central portfolio hub. The update audits all repositories visible to the authenticated GitHub account, separates public and private projects, groups projects by domain, adds the Baluarte ecosystem map, gives Veritas and Digital Logic their own sections, and records only deployments verified during the audit.

The stale manual language list was replaced by a generated public-only language table and a visual distribution asset. A dashboard now reports repository, language, activity, visibility, deployment, and academic counts from the current inventory.

The refresh workflow was added at `.github/workflows/update-profile.yml`, with the deterministic generator in `scripts/update_profile.py` and the data contract in `docs/README_DATA.md`. The workflow is daily and manual-triggered; it commits only when generated content changes.

A backup branch named `backup/before-profile-readme-overhaul` was created before the feature branch `feature/profile-readme-overhaul`. The intended delivery path is feature branch, validation, pull request, CI, merge, and feature-branch deletion.

## Maintenance policy

Future profile changes should update the generator or its data contract rather than manually editing generated blocks. Each significant change should add a dated entry here and preserve the backup/branch/validation flow described in `docs/README_DATA.md`.


## 2026-08-26 — restauração da estética original

A estética anterior foi restaurada a partir da branch `backup/before-profile-readme-overhaul`. O README voltou a incluir o banner `capsule-render`, os títulos animados `readme-typing-svg`, badges, bloco ASCII de field manual, ícones `skillicons`, assets de estatísticas, núcleo visual do J.A.R.V.I.S., CPU build log, deployments, gaming log, fan fiction, activity graph, contribution snake, canais de contato, contador de visitas e footer.

Os dados auditados continuam presentes em blocos dinâmicos separados: dashboard, mapa completo, catálogo público, seção privada, estatísticas de linguagens, projetos em destaque e sites verificados. O gerador foi testado novamente com `generator-idempotency=pass`, e os assets visuais originais foram mantidos no diretório `assets/`.


## 2026-08-26 — missões escolhidas e Arsenal expandido

Foi adicionada a seção `CURATED-FEATURED`, com sete missões editoriais públicas controladas por `docs/README_FEATURED.json`. A seleção mantém o design restaurado — título animado, alinhamento central e tabela de missão — e ignora automaticamente projetos privados ou inexistentes.

O bloco `ARSENAL-STACK` agora mostra as 17 linguagens detectadas nos repositórios públicos e uma matriz editorial de ferramentas, plataformas e ambientes confirmados, incluindo Git, GitHub Actions, VS Code, Linux, Node.js, Vite, React, Tailwind CSS, Electron, Docker, Supabase, MCP, Unity, Arduino, Obsidian + Claude, MapLibre GL e Flowgorithm/Portugol.

O workflow passou a salvar uma cópia do README antes do refresh e a executar `scripts/validate_dynamic_sections.py`. O job falha se qualquer conteúdo fora dos blocos dinâmicos for alterado; também valida os novos manifestos JSON. A execução continua idempotente e só cria commit quando há mudança real nos arquivos gerados.


## 2026-08-26 — exclusões editoriais solicitadas

Os repositórios `sujok-brasil-backend`, `sujok-brasil-frontend` e `casa-de-apoio-mmg` foram adicionados a `docs/README_EXCLUDED.json` e deixaram de participar da renderização do README. O filtro é aplicado antes do dashboard, dos catálogos, do mapa, dos destaques, dos sites e das métricas, reduzindo o inventário publicado de 82 para 79 projetos, com 62 públicos e 17 privados visíveis no README.

O teste `scripts/validate_exclusions.py` foi incorporado ao workflow para impedir que os nomes retornem em refreshs futuros.
