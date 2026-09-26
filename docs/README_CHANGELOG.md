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


## 2026-08-26 — redesign field report

O README recebeu uma nova camada visual inspirada no formato de relatório de campo: `GITHUB SNAPSHOT`, métricas em painel, `LANGUAGE MATRIX`, continuidade, atividade e troféus com títulos técnicos consistentes. A estética anterior foi preservada — paleta Ouro de Fábula, banners, badges e blocos de missão — enquanto as seções ganharam hierarquia e espaçamento mais claros.

A seção visual de linguagens passou a apresentar o conjunto completo das 17 linguagens públicas auditadas, com badges de contagem e fallback textual para linguagens menos comuns. Ferramentas e plataformas foram reorganizadas no Arsenal, incluindo Git, GitHub, VS Code, Linux, Node.js, Vite, React, Tailwind CSS, MySQL, PostgreSQL, Blender, Electron, Docker, Supabase, Unity, Arduino, Obsidian, Vercel e Claude.

O painel `assets/profile-snapshot.svg` é gerado pelo mesmo script do catálogo e entra no commit automático junto com `README.md` e `assets/lang-stats.svg`. Nenhuma métrica privada ou repositório editorialmente excluído participa da publicação.


## 2026-08-26 — README V2 clean field report

A segunda versão reorganizou o README em uma sequência mais curta e orientada à leitura: snapshot, ficha de agente, missão principal, Arsenal, missões em destaque, mapa recolhido, matriz de linguagens, deployments, núcleo J.A.R.V.I.S., atividade, troféus e contato. Os assets visuais originais continuam presentes, agora distribuídos em blocos com hierarquia mais clara.

Os destaques passaram a aceitar somente repositórios públicos, enquanto o catálogo privado permanece separado e limitado ao recorte editorial permitido. As três exclusões permanentes continuam sendo aplicadas antes das métricas, catálogos, sites, destaques e assets gerados.

Os badges das 17 linguagens usam parâmetros codificados de `label` e `message`; a matriz gráfica mantém barras proporcionais e a tabela completa continua disponível como evidência textual. O gerador permanece idempotente e o workflow continua limitado aos blocos dinâmicos e assets autorizados.


## 2026-08-26 — Arsenal categorizado e métricas reconciliadas

O Arsenal passou a separar visualmente **Linguagens**, **Frameworks & Web**, **Infraestrutura & DevOps**, **IA & Conhecimento** e **Hardware & Simulação**. As 17 linguagens continuam sendo derivadas dos mapas públicos do GitHub, enquanto as ferramentas permanecem documentadas no manifesto `README_STACK.json` com função e evidência pública.

O workflow `v2-validation.yml` passou a executar `scripts/validate_language_badges.py` em pull requests e pushes na `main`. O validador confere labels únicos e legíveis, endpoint estático do Shields.io, coerência entre badges e matriz, presença das categorias e disponibilidade HTTP dos badges, com concorrência e retry curto.

Os cards de atividade agora distinguem `CONTRIBUIÇÕES TOTAIS`, `COMMITS DIRETOS` e `REPOS CRIADOS`. Na janela de 365 dias analisada, 1.807 contribuições são explicadas por 1.114 commits, 535 pull requests, 94 issues, 3 reviews e 61 contribuições de repositório. A diferença é semântica e esperada, não uma inconsistência do contador.

O relatório detalhado da alteração está em `docs/README_BADGES_METRICS_REPORT.md`.


## 2026-09-25 — auditoria do núcleo e Fase 0 de estabilização

A [auditoria técnica](audits/2026-09-25-ecosystem-core-audit.md) mostrou que o refresh do README não rodava desde 26/08 (o workflow pulava sem o secret `PROFILE_README_TOKEN` e ficava verde), que quatro coletores usam quatro definições de inventário e que dois scripts gravavam o mesmo `assets/lang-stats.svg`. A arquitetura alvo (núcleo com PostgreSQL, migração gradual para Rust) está em `docs/architecture/`, `docs/database/`, `docs/migration/` e `docs/DECISION-LOG.md`; o schema e seus testes, em `db/`.

A Fase 0 corrigiu o que tornava o pipeline atual imprevisível, sem mexer no design do README nem no conteúdo fora dos marcadores:

- `replace_block` trata o corpo como texto literal: uma descrição com `\` não derruba mais o refresh, e `\g<0>` não é mais substituído em silêncio.
- `assets/lang-stats.svg` tem um escritor só, o `lang_stats.py`, preservando o painel publicado; o bot parou de procurar o bloco `LANG-STATS`, que não existe desde agosto, e passou a aplicar as exclusões editoriais.
- O carimbo "atualizado em" não é mais ditado pelo push horário do próprio perfil.
- `update-profile.yml` termina **vermelho** quando o secret falta (testes e validações ainda rodam); ganhou `timeout-minutes`, `concurrency` e a conferência dos 13 marcadores.
- Os quatro workflows que escrevem no `main` fazem push com rebase e nova tentativa.
- A timeline não depende mais do shields.io para publicar.
- `restore_original_style.py` foi aposentado; `validate_restored_style.py` passou a rodar no CI.
- Teste golden do gerador: entrada sintética fixa → README, catálogo e SVG byte a byte (`tests/test_golden_profile.py`).

Pendências do dono do perfil: configurar `PROFILE_README_TOKEN` e decidir se as linguagens de repositórios privados continuam no mapa completo de projetos.


## 2026-09-25 — Fase 1: fundação do núcleo em Rust

O README e os assets não mudaram. Entrou o começo do núcleo que vai gerá-los:

- **Workspace Cargo** com três crates: `ecosystem-domain` (regras puras do catálogo, sem rede nem banco), `store` (PostgreSQL) e `profile-core` (a CLI).
- **Paridade provada, não suposta.** As regras de classificação, status, prioridade, apresentação e descoberta de site foram portadas para Rust e conferidas contra as funções Python reais em 640 casos de um fixture compartilhado (`tests/fixtures/parity/domain.json`), gerado pelo Python do CI. O port reproduz de propósito as peculiaridades do Python — inclusive o defeito A6 e a leitura de datas do `fromisoformat`, portada do C do CPython.
- **`profile-core db migrate | revert | status`** aplica as migrations de `db/migrations`, embutidas no binário. Tudo ou nada; reverter com dado exige `--allow-data-loss`; o schema aplicado pelo binário é idêntico ao aplicado pelo psql.
- Novo workflow **Rust Core**: formato, lints, testes (com PostgreSQL 16) e o binário de ponta a ponta.

Decisões em `docs/DECISION-LOG.md` (D-023 a D-025); plano atualizado em `docs/migration/PYTHON-TO-RUST.md`.


## 2026-09-26 — Fase 2: monitor de sites em Rust, em modo sombra

O README e os assets não mudaram. O `scripts/check_websites.py` ganhou um equivalente em Rust, que por enquanto só roda ao lado dele:

- **`profile-core check sites`** imprime o mesmo relatório (texto, `--json` e código de saída), byte a byte, exceto o horário da checagem. Para chegar lá, o crate `site-monitor` porta do Python a parte que decide o relatório — a forma como o `urllib` monta a URL final, resolve redirecionamentos e detecta laços — e confere contra um fixture gerado pela própria biblioteca padrão, na versão do CI.
- **Histórico:** cada checagem pode ser gravada em `ecosystem.website_checks` (nunca sobrescrita). Sem banco configurado, o comando recusa em vez de pular o histórico sem avisar.
- **Prova:** Python e Rust rodam contra o mesmo servidor local com 45 cenários (redirects, laços, erros, timeout, TLS) e dão o mesmo resultado. O novo workflow *Site Monitor Shadow* repete a comparação todo dia sobre os sites reais.
- **Achados:** o monitor Python publica como "no ar" sites cujo redirecionamento falha em laço ou aponta para `mailto:` (A21), e uma URL com porta inválida derruba a verificação inteira (A22). O Rust reproduz o primeiro de propósito, até a decisão editorial, e não reproduz a queda.

Decisões em `docs/DECISION-LOG.md` (D-026 a D-028).
