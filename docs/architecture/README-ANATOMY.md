# Anatomia do README

Como o `README.md` do perfil é montado hoje, bloco por bloco, e como ele será
alimentado pelo núcleo sem perder o design. Referência para qualquer mudança:
**o que está fora dos marcadores é escrito à mão e nunca é tocado por
gerador**; o que está dentro é reescrito inteiro a cada refresh.

Estado medido em `28e88af`: 902 linhas, 67.624 bytes, 13 blocos gerados.

## 1. Mapa de seções

| Linhas | Seção | Tipo | Conteúdo |
|:---|:---|:---|:---|
| 1–24 | Hero | manual | banner `capsule-render`, título animado `readme-typing-svg`, 4 badges, frase, 4 botões com âncoras |
| 26–38 | Field manual | manual | bloco ASCII |
| 40–49 | `// COMECE POR AQUI` | manual | menu de âncoras |
| 51–61 | `// FICHA DE AGENTE` | manual | PT-BR + EN-US |
| 63–150 | `// O QUE EU CONSTRUO` | título manual + **`WHAT-I-BUILD`** | 6 domínios com contagem |
| 152–247 | `// PRODUTOS EM DESTAQUE` (`#produtos`) | intro manual + **`PRODUCT-CARDS`** | 6 cards |
| 249–270 | `// MISSÕES EM DESTAQUE` | intro manual + **`FEATURED-PROJECTS`** | tabela de até 10 |
| 272–313 | `// SITES NO AR` (`#sites`) | **`WEBSITE-DIRECTORY`** + **`LIVE-PROJECTS`** + critério manual | badges + tabela de verificação |
| 314–351 | `// ECOSSISTEMA-FAROL` (`#baluarte`) | manual | árvore do Baluarte, **aviso manual "HTTP 404"**, `profile-projects.svg` |
| 353–361 | `// NÚCLEO J.A.R.V.I.S.` | manual | `jarvis-console.svg` |
| 363–377 | `// DIGITAL LOGIC SIM` | manual | ASCII do CPU build log |
| 379–504 | `// ECOSSISTEMA DIGITAL` (`#ecossistema`) | **`ECOSYSTEM-MAP`** + **`PROJECT-MAP`** | árvore + mapa completo (`<details>`) |
| 506–612 | `// ARSENAL` (`#arsenal`) | ícones `skillicons` manuais + **`LANGUAGE-BADGES`** + **`ARSENAL-STACK`** | badges + tabelas de ferramentas |
| 614–644 | `// MATRIZ DE LINGUAGENS` | **`LANGUAGE-STATS`** | tabela + `lang-stats.svg` |
| 646–682 | `// ENGENHARIA` (`#engenharia`) | manual (referências a SVGs gerados) | `profile-stats`, `profile-top-langs`, `profile-streak`, link da timeline, snake, `profile-trophies` |
| 684–706 | `// GITHUB SNAPSHOT` | **`PROFILE-DASHBOARD`** | contagens + `profile-snapshot.svg` |
| 708–780 | `// REPOSITÓRIOS PÚBLICOS` | **`PUBLIC-PROJECTS`** | catálogo público (`<details>`) |
| 782–813 | `// REPOSITÓRIOS PRIVADOS` | **`PRIVATE-PROJECTS`** | listagem privada autorizada (`<details>`) |
| 815–842 | `// ARQUIVO PESSOAL` | manual | gaming e fan fiction (`<details>`) |
| 844–854 | `// KIT PÚBLICO` | manual | links do branch `template/readme-style-kit` |
| 856–902 | `// CANAL DE COMUNICAÇÃO` (`#contato`) + rodapé | manual | contatos, citação, contador externo, nota de métricas, banner |

## 2. Os 13 blocos gerados

Todos por `scripts/update_profile.py` → `replace_block()`, na ordem de `main()`.

| Marcador | Função | Entradas | Público/privado | Observação |
|:---|:---|:---|:---|:---|
| `PROFILE-DASHBOARD` | `render_dashboard` | inventário inteiro, linguagens públicas, sites verificados, exclusões | **conta** privados | + `profile-snapshot.svg` |
| `WHAT-I-BUILD` | `render_what_i_build` | apresentações públicas, `DOMAINS` | só público | domínio vazio some |
| `PRODUCT-CARDS` | `render_product_cards` | apresentações públicas, `marketing_priority`, `FEATURED_SUMMARIES` | só público | 6 cards, site no ar primeiro |
| `FEATURED-PROJECTS` | `render_featured_projects` | `README_FEATURED.json` + `featured_score` (`FEATURED_PRIORITY`) | só público | até 10; curadoria antes da heurística |
| `WEBSITE-DIRECTORY` | `render_website_directory` | sites verificados | só público | badges |
| `ARSENAL-STACK` | `render_arsenal_stack` | linhas de linguagem, `README_STACK.json` | só público | **no `main` está numa versão antiga do renderizador** (auditoria, A1/A2) |
| `LANGUAGE-BADGES` | `render_language_badges` | linhas de linguagem, `LANGUAGE_COLORS` | só público | validado por `validate_language_badges.py` |
| `LANGUAGE-STATS` | `render_language_stats` | linhas de linguagem, `generated_at` | só público | a imagem logo abaixo é do `lang_stats.py`, com outro inventário (A3; dono do arquivo: D-021) |
| `PUBLIC-PROJECTS` | `render_public_projects` | apresentações | só público | |
| `PRIVATE-PROJECTS` | `render_private_projects` | repositórios privados | **privado autorizado** | nome, categoria, descrição, status, link |
| `LIVE-PROJECTS` | `render_live_projects` | sites verificados | só público | coluna site antes da coluna código |
| `ECOSYSTEM-MAP` | `render_ecosystem_map` | apresentações públicas | só público | 4 nomes por ramo, `●` = site no ar |
| `PROJECT-MAP` | `render_project_map` | **todos** os repositórios + linguagens | **inclui privados** | publica linguagens de privados (A20) |

## 3. Dados que o README consome

| Arquivo | Classe | Blocos |
|:---|:---|:---|
| `docs/README_SITES.json` | manual | todos que mostram site |
| `docs/README_FEATURED.json` | manual | `FEATURED-PROJECTS`, prioridade de todos |
| `docs/README_STACK.json` | manual | `ARSENAL-STACK` |
| `docs/README_EXCLUDED.json` | manual | todos (filtro) + contagem em `PROFILE-DASHBOARD` |
| constantes em `update_profile.py` | manual (em código) | `FEATURED-PROJECTS`, `PRODUCT-CARDS`, `LANGUAGE-*`, `WHAT-I-BUILD` |
| `assets/*.svg` | derivado (exceto `jarvis-console.svg`) | seções manuais e blocos |

## 4. Workflows que escrevem no README

| Workflow | O que toca |
|:---|:---|
| `update-profile.yml` | os 13 blocos (quando o secret existe — sem ele, o job falha desde a Fase 0) |
| nenhum outro | — (o `lang-stats.yml` deixou de tentar o bloco órfão `LANG-STATS` na Fase 0) |

As seções manuais só mudam por PR humano. `validate_dynamic_sections.py`
garante, a cada refresh, que nada fora dos 13 blocos mudou.

## 5. Dependências externas em tempo de leitura

| Serviço | Uso | Se cair |
|:---|:---|:---|
| `img.shields.io` | 75 badges (CTAs, linguagens, contato) | imagens quebradas; links continuam |
| `skillicons.dev` | 10 grades de ícones | idem |
| `capsule-render.vercel.app` | banner e rodapé | idem |
| `readme-typing-svg.demolab.com` | título animado | idem |
| `komarev.com` | contador de visualizações | idem |
| `raw.githubusercontent.com` (branch `output`) | snake | idem |
| CDN do Plotly | timeline interativa (HTML fora do README) | gráfico não renderiza |

Os cards de estatística já foram trazidos para SVG local
(`profile_cards.py`) exatamente para reduzir essa lista.

## 6. Como o README será alimentado pelo núcleo

O design não muda. O que muda é **de onde vêm os dados** e **quem escreve**.

1. **Marcadores são o contrato.** Os 13 pares `START/END` continuam; o
   renderizador Rust reescreve só entre eles, com a mesma verificação de
   "nada mudou fora" (hoje `validate_dynamic_sections.py`).
2. **Uma fonte por bloco.** Cada bloco lê de uma view pública
   (`public_projects`, `public_language_totals`, `website_status_current`…) ou
   de `private_repository_listing` para o bloco privado. Acaba o caso "tabela
   diz 17, imagem diz 19" (A3/A4): tabela e imagem saem da mesma consulta.
3. **Seções manuais continuam manuais.** Hero, ficha, Baluarte, JARVIS, CPU
   log, arquivo pessoal, kit e contato não viram template. Uma exceção
   proposta: o aviso "O deployment público do Baluarte respondeu HTTP 404" é
   uma afirmação sobre estado de site escrita à mão — deveria virar um bloco
   gerado pequeno (ou sair), pela mesma regra que vale para os cards.
4. **Paridade primeiro.** O renderizador Rust reproduz os 13 blocos byte a
   byte a partir das mesmas entradas (D-006), com duas exceções que dependem
   de decisão: `generated_at` sem o próprio perfil (A7) e a coluna *Stack* de
   privados (A20).
5. **Um commit por execução.** README, SVGs e `project-catalog.json` saem do
   mesmo snapshot, no mesmo commit (D-018).
