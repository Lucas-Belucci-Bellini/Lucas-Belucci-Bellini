# Auditoria técnica — núcleo do ecossistema do perfil

**Data:** 2026-09-25
**Repositório:** `Lucas-Belucci-Bellini/Lucas-Belucci-Bellini`
**Base auditada:** `main` em `28e88af` (`chore(bot): snapshot horário do ecossistema [skip ci]`)
**Objetivo:** levantar o estado real antes de qualquer migração Python → Rust ou
introdução de banco SQL. Nenhum comportamento em produção foi alterado nesta fase.

> Documentos derivados desta auditoria:
> [arquitetura](../architecture/SYSTEM-ARCHITECTURE.md) ·
> [fluxo de dados](../architecture/DATA-FLOW.md) ·
> [componentes](../architecture/COMPONENTS.md) ·
> [anatomia do README](../architecture/README-ANATOMY.md) ·
> [banco](../database/DATABASE-ARCHITECTURE.md) ·
> [modelo de dados](../database/DATA-MODEL.md) ·
> [migrations](../database/MIGRATIONS.md) ·
> [Python → Rust](../migration/PYTHON-TO-RUST.md) ·
> [matriz de migração](../migration/MIGRATION-MATRIX.md) ·
> [API](../api/API-ARCHITECTURE.md) ·
> [log de decisões](../DECISION-LOG.md)

---

## 1. Método

Tudo o que está afirmado abaixo foi lido no código ou medido, e a evidência
está citada ao lado. Nada foi inferido de nome de arquivo.

| Fonte | O que foi feito |
|:---|:---|
| Código | Leitura integral dos 14 scripts Python (3.799 linhas), 6 workflows, 5 arquivos de teste, 4 manifestos JSON, `AGENTS.md`, `ECOSYSTEM-INDEX.md`, todos os `docs/` e o `README.md` (902 linhas, 67.624 bytes). |
| Execução local | `py_compile` de todos os scripts; `python3 -m unittest discover -s tests` (**57 testes, todos passam**); `validate_exclusions`, `validate_project_links`, `validate_restored_style`, `validate_dynamic_sections` (todos passam). |
| GitHub Actions | Execuções reais de `update-profile.yml` lidas pela API (run `35980267145`, job `107570211889`). |
| Histórico Git | 441 commits no `main`: 223 snapshots horários, 124 análises de linguagem, 31 timelines — e **um único** `refresh repository catalog` (2026-08-26). |
| Reprodução | Divergências entre README e catálogo, classificação e o defeito de `replace_block` foram reproduzidos com scripts contra os módulos reais (seção 9). |

---

## 2. Resumo executivo

O repositório não é "um README": é um **sistema de publicação com cinco
coletores independentes**, cada um consultando o GitHub do seu jeito e
escrevendo artefatos versionados no próprio `main`. O desenho de cada peça é
cuidadoso — idempotência, "falha fecha", privacidade, CTA site-primeiro,
testes contra servidor HTTP local. O problema está **entre** as peças.

Os achados que mais pesam na decisão de arquitetura:

1. **O gerador principal não roda há 30 dias e o CI diz que está tudo verde.**
   `update-profile.yml` pula o passo de refresh quando o secret
   `PROFILE_README_TOKEN` não existe e termina `success`. Os 13 blocos
   dinâmicos do README estão congelados desde 2026-08-26 (A1).
2. **Não existe uma definição única de "o ecossistema".** Quatro coletores
   listam repositórios com quatro regras diferentes; o README público exibe
   ao mesmo tempo **79, 62, 69, 67 e 89** como tamanho do inventário (A3).
3. **README e `docs/project-catalog.json` discordam** — 11 projetos existem
   num e não no outro, 5 têm status diferente — apesar de a documentação
   garantir que "os dois nunca discordam" (A2).
4. **Dois scripts escrevem o mesmo `assets/lang-stats.svg`** com dados e
   design diferentes; a tabela de linguagens diz 17 linguagens / 16,59 MB e a
   imagem logo abaixo diz 19 / 23,34 MB (A4).
5. **Um `\` em qualquer descrição de repositório derruba o gerador** (A5).
6. **O `PROJECT-MAP` publica as linguagens de 6 repositórios privados**, contra
   o contrato de privacidade escrito em `docs/README_DATA.md` (A20).

Nenhum desses problemas é de desempenho ou de linguagem. Eles são de
**fonte de verdade**: não há um lugar onde o inventário, a classificação e o
estado dos sites sejam decididos uma vez e reutilizados. É isso que justifica
o núcleo com banco — e é também por isso que a migração para Rust **não deve
começar pelo README** (ver [PYTHON-TO-RUST.md](../migration/PYTHON-TO-RUST.md)).

---

## 3. Inventário do repositório

| Caminho | Linhas | Papel | Tipo |
|:---|---:|:---|:---|
| `scripts/update_profile.py` | 1130 | Gerador do README: inventário, linguagens, classificação, status, 13 blocos, 2 SVGs, catálogo | núcleo |
| `scripts/project_catalog.py` | 501 | Biblioteca: verificação de sites, descoberta, `Presentation`, prioridade, catálogo JSON, CTAs | núcleo |
| `.github/scripts/lang_stats.py` | 669 | Linguagens + tipos de arquivo (árvore git), 2 SVGs, bloco `LANG-STATS` | coletor |
| `.github/scripts/ecosystem_watch.py` | 280 | Monitor horário de commits; estado cumulativo | coletor com estado |
| `.github/scripts/profile_cards.py` | 207 | GraphQL de contribuições + 4 SVGs | coletor |
| `scripts/update_contribution_timeline.py` | 178 | GraphQL mensal + HTML Plotly | coletor |
| `scripts/check_websites.py` | 123 | CLI de saúde dos sites (não grava nada) | ferramenta |
| `scripts/validate_profile.py` | 208 | Validador amplo (fora do CI) | validador |
| `scripts/validate_language_badges.py` | 123 | Contrato dos badges + HTTP no shields.io | validador |
| `scripts/validate_project_links.py` | 120 | Regra de CTA no catálogo e no README | validador |
| `scripts/validate_dynamic_sections.py` | 58 | Só os blocos gerados mudaram | validador |
| `scripts/validate_restored_style.py` | 57 | Componentes visuais presentes (fora do CI) | validador |
| `scripts/validate_exclusions.py` | 16 | Exclusões ausentes do README | validador |
| `scripts/restore_original_style.py` | 129 | Migração única de agosto; caminho `/home/ubuntu/...` fixo | obsoleto |
| `tests/*.py` | 651 | 57 testes `unittest` | testes |
| `docs/README_*.json` (4) | — | Manifestos editoriais | dado manual |
| `docs/project-catalog.json` | 1611 | Catálogo gerado | dado derivado |
| `docs/ECOSYSTEM-COMMIT-STATE.json` | 496 | Estado do monitor | **dado com estado** |
| `assets/*.svg` (8) | — | Cards do README | 7 gerados, 1 manual |

Sem dependências externas: todo o Python usa só a biblioteca padrão
(`urllib`, `concurrent.futures`, `re`, `json`). Isso é uma qualidade a
preservar — o CI não instala nada.

---

## 4. Workflows

| Workflow | Gatilho | Token | Escreve | Commit | Concorrência | Estado real |
|:---|:---|:---|:---|:---|:---|:---|
| `update-profile.yml` | diário `17 4 * * *`, push em paths, manual | `PROFILE_README_TOKEN` (PAT) → senão **pula** | README, `lang-stats.svg`, `profile-snapshot.svg`, `project-catalog.json` | `chore(profile): refresh repository catalog` | **nenhuma**, sem `timeout-minutes` | ⚠️ **verde sem fazer nada** desde 2026-08-26 (A1) |
| `lang-stats.yml` | semanal `17 3 * * 0`, push em paths, manual | `GITHUB_TOKEN` (instalação → só público) | README, 6 SVGs | `chore(bot): atualiza análise de linguagens` | `lang-stats` | roda; bloco do README órfão (A9) |
| `ecosystem-watch.yml` | **horário** `17 * * * *` | `GITHUB_TOKEN` | `ECOSYSTEM-COMMIT-STATE.json`, `ECOSYSTEM-COMMIT-MONITOR.md` | `chore(bot): snapshot horário [skip ci]` | `ecosystem-watch` | roda; 223 commits |
| `contributions-timeline.yml` | diário `23 6 * * *`, push, manual | `PROFILE_README_TOKEN` \|\| `github.token` | `docs/assets/contributions-timeline-*` | `chore(profile): refresh contribution timeline [skip ci]` | `contribution-timeline` | roda |
| `snake.yml` | a cada 12 h, **todo push humano no `main`** | `GITHUB_TOKEN` | branch `output` | (action externa) | nenhuma | roda |
| `v2-validation.yml` | PR/push em paths, manual | — (read) | nada | — | por ref | roda |

Observações transversais:

- **Quatro workflows fazem `git push` direto no `main`**, cada um no seu grupo
  de concorrência e sem `git pull --rebase`. O horário `:17` do monitor
  coincide com o slot de `lang-stats` (03:17 de domingo) e de `update-profile`
  (04:17). O agendador atrasa execuções de forma imprevisível, então a colisão
  é probabilística: quem empurra depois recebe *non-fast-forward* e o job falha.
  Não há corrupção — só execução perdida (A8).
- Commits feitos com `GITHUB_TOKEN` **não disparam outros workflows**. Por
  isso o `v2-validation.yml` nunca valida o que os bots publicam — só o que
  humanos publicam.
- Python 3.11 em dois workflows, 3.12 nos outros (A13).

---

## 5. Scripts Python — o que cada um realmente faz

### 5.1 `scripts/update_profile.py` (gerador do README)

1. **Inventário:** `GET /user/repos?affiliation=owner,collaborator,organization_member`
   com PAT; sem PAT, `GET /users/{owner}/repos?type=owner` (só público). O modo
   `--write` recusa rodar sem PAT para não apagar a seção privada.
2. **Exclusões:** remove `README_EXCLUDED.json` por nome ou `full_name`.
3. **Linguagens:** `GET /repos/{full_name}/languages` para **cada** repositório
   (≈ 89 chamadas sequenciais).
4. **Classificação** `classify()`: palavras-chave por substring sobre
   `nome + descrição` → 9 rótulos em português.
5. **Status** `status_for()`: privado → arquivado → acadêmico → lista de nomes
   fixos → idade do `pushed_at` (≤ 60 dias ativo, ≤ 365 em desenvolvimento).
6. **Sites:** descoberta (`homepage` → `README_SITES.json` → nada) e
   verificação HTTP concorrente via `project_catalog.check_websites`.
7. **Apresentação:** `resolve_presentation` decide CTA, prioridade e selo.
8. **Render:** 13 funções `render_*`, uma por marcador, via `replace_block`.
9. **Saídas:** `README.md`, `assets/lang-stats.svg`, `assets/profile-snapshot.svg`,
   `docs/project-catalog.json` (este só se mudou).

Dados editoriais **embutidos no código**: `FEATURED_PRIORITY`,
`FEATURED_SUMMARIES`, `LANGUAGE_DISPLAY`, `LANGUAGE_COLORS`, `DOMAINS`, as
palavras-chave de `classify()` e os nomes fixos de `status_for()`.

### 5.2 `scripts/project_catalog.py` (biblioteca da vitrine)

A peça mais sólida do repositório: taxonomia fechada, descoberta que **nunca
deduz URL do nome**, verificação com `GET` (não `HEAD`), retry curto,
concorrência limitada, "falha fecha", catálogo sem carimbo de tempo e gravado
só quando muda. Coberta por 25 testes contra servidor HTTP local. É o melhor
candidato a primeira fatia em Rust justamente porque o comportamento já está
especificado por teste.

### 5.3 `.github/scripts/ecosystem_watch.py` (monitor horário)

Lista repositórios **públicos, não-fork, do dono**, exclui o próprio perfil,
lê o último commit do branch padrão e usa `compare` para contar commits novos.
Grava só quando há mudança semântica. Mantém contadores cumulativos que
**não podem ser recalculados**: `project_commits` parte de
`LEGACY_BASELINE = 1538` e `monitor_commits` conta snapshots publicados. É o
único dado do repositório cuja perda é irreversível (A15).

### 5.4 `.github/scripts/lang_stats.py` e `profile_cards.py` (bot semanal)

`lang_stats.py` repete o inventário (regra própria: `affiliation=owner`,
sem forks, sem exclusões), soma linguagens e **lê a árvore git de cada
repositório** para contar tipos de arquivo. Escreve `lang-stats.svg` e
`profile-top-langs.svg` e tenta substituir o bloco `LANG-STATS` — que não
existe mais no README. `profile_cards.py` consulta o GraphQL e escreve
`profile-stats`, `profile-streak`, `profile-trophies` e `profile-projects.svg`;
este último é uma lista fixa no código ("Stock Analyzer" não é nome de
nenhum repositório do inventário).

### 5.5 `scripts/update_contribution_timeline.py`

13 consultas GraphQL (uma por mês da janela de 365 dias) → JSON + HTML com
Plotly via CDN. Independente dos demais.

### 5.6 Validadores

| Validador | No CI? | Observação |
|:---|:---|:---|
| `validate_dynamic_sections.py` | `update-profile` | Garante que só o conteúdo entre marcadores mudou. Essencial. |
| `validate_project_links.py` | `update-profile`, `v2-validation` | Regra de CTA. Essencial. |
| `validate_exclusions.py` | `update-profile`, `contributions-timeline` | Substring simples no README. |
| `validate_language_badges.py` | `v2-validation`, `contributions-timeline` | Faz HTTP no shields.io — acopla a timeline a um serviço de terceiros (A10). |
| `validate_profile.py` | **não** | Lê `/home/ubuntu/profile_readme_audit/repos.json`; **grava** `docs/README_LINK_CHECK.json` (fora do `.gitignore`). |
| `validate_restored_style.py` | **não** | Passa hoje; ninguém o executa. |

---

## 6. Testes

57 testes, todos passando, em 0,5 s:

| Arquivo | Testes | Cobre |
|:---|---:|:---|
| `test_project_catalog.py` | 25 | verificação HTTP (servidor local), descoberta, CTA, privado, taxonomia, idempotência do catálogo |
| `test_profile_showcase.py` | 25 | cards, mapa, curadoria, tabela de destaques, diretório, domínios, slug |
| `test_ecosystem_watch.py` | 4 | exclusão do perfil, no-op, SHA novo, erro como estado |
| `test_profile_cards.py` | 2 | rótulos do card de estatísticas |
| `test_update_profile.py` | 1 | rótulos legíveis de `C#` e `PL/pgSQL` |

**Sem cobertura:** `classify()`, `status_for()`, `featured_score()`,
`replace_block()`, `render_dashboard`, `render_language_stats`,
`render_arsenal_stack`, `render_project_map`, os SVGs de `update_profile`,
todo o `lang_stats.py`, `update_contribution_timeline.py` e o fluxo de
`main()` de qualquer coletor. Não existe teste de ponta a ponta "entrada fixa →
README byte a byte" — e é exatamente o teste de que a migração precisa
(ver [PYTHON-TO-RUST.md §6](../migration/PYTHON-TO-RUST.md)).

---

## 7. Dados: manuais, derivados e com estado

| Arquivo | Classe | Quem escreve | Quem lê |
|:---|:---|:---|:---|
| `docs/README_SITES.json` | **manual** (editorial) | pessoa | `update_profile`, `check_websites` |
| `docs/README_FEATURED.json` | **manual** | pessoa | `update_profile` |
| `docs/README_STACK.json` | **manual** | pessoa | `update_profile` |
| `docs/README_EXCLUDED.json` | **manual** | pessoa | `update_profile`, `validate_*` |
| README fora dos marcadores | **manual** | pessoa | GitHub (renderização) |
| `AGENTS.md`, `ECOSYSTEM-INDEX.md` | **manual** | pessoa | agentes |
| Constantes no código (seção 5.1) | **manual escondido** | pessoa, via PR de código | geradores |
| 13 blocos do README | derivado | `update_profile` | GitHub |
| `docs/project-catalog.json` | derivado | `update_profile` | `check_websites`, validadores, consumidores externos |
| `assets/profile-snapshot.svg` | derivado | `update_profile` | README |
| `assets/lang-stats.svg` | derivado — **dois escritores** | `update_profile` **e** `lang_stats` | README |
| `assets/profile-top-langs.svg` | derivado | `lang_stats` | README |
| `assets/profile-{stats,streak,trophies,projects}.svg` | derivado (`projects` é fixo) | `profile_cards` | README |
| `docs/assets/contributions-timeline-*` | derivado | `update_contribution_timeline` | README (link) |
| branch `output` (snake) | derivado | action externa | README |
| `docs/ECOSYSTEM-COMMIT-STATE.json` | **derivado com estado** | `ecosystem_watch` | `ecosystem_watch` (entrada da próxima execução) |
| `docs/ECOSYSTEM-COMMIT-MONITOR.md` | derivado | `ecosystem_watch` | pessoas |
| `assets/jarvis-console.svg`, `docs/assets/*.png` | manual (arte) | pessoa | README, docs |

### Fontes externas

| Fonte | Endpoint | Consumidores | Autenticação |
|:---|:---|:---|:---|
| GitHub REST — inventário | `/user/repos`, `/users/{u}/repos` | `update_profile`, `lang_stats`, `ecosystem_watch` | PAT ou `GITHUB_TOKEN` |
| GitHub REST — linguagens | `/repos/{r}/languages` | `update_profile`, `lang_stats` | idem |
| GitHub REST — árvore | `/repos/{r}/git/trees/{branch}?recursive=1` | `lang_stats` | idem |
| GitHub REST — commits | `/repos/{r}/commits`, `/compare/{a}...{b}` | `ecosystem_watch` | idem |
| GitHub GraphQL | `contributionsCollection`, `repositories.totalCount` | `profile_cards`, `update_contribution_timeline` | token obrigatório |
| HTTP dos sites | URLs declaradas | `project_catalog`, `check_websites` | nenhuma |
| shields.io | badges | `validate_language_badges` | nenhuma |

---

## 8. Dependências entre scripts

```text
update_profile.py ──import──▶ project_catalog.py ◀──import── check_websites.py
        │                            ▲
        │ escreve                    │ lê
        ▼                            │
docs/project-catalog.json ───────────┘ ◀── validate_project_links.py, validate_profile.py

lang_stats.py ──escreve──▶ assets/lang-stats.svg ◀──escreve── update_profile.py   (conflito)

ecosystem_watch.py ──lê/escreve──▶ docs/ECOSYSTEM-COMMIT-STATE.json   (estado entre execuções)

tests/*  ──importlib──▶ update_profile, project_catalog, ecosystem_watch, profile_cards
```

Não há dependência entre workflows (nenhum `workflow_run`/`needs` entre eles);
o acoplamento é **pelos arquivos no `main`**.

---

## 9. Achados

Severidade: **crítico** (informação pública errada agora), **alto** (vai
errar ou já erra em silêncio), **médio** (defeito latente ou custo recorrente),
**baixo** (higiene), **info** (restrição a respeitar na migração).

### A1 · crítico — o gerador principal está desligado e o CI mostra verde

- **Evidência:** job `107570211889` (2026-09-24): passo *Refresh README and
  generated assets* = `skipped`; *Skip refresh when private-aware inventory is
  unavailable* = `success`. O job inteiro leva ~8 s. Em 441 commits do `main`,
  há **um** `chore(profile): refresh repository catalog`, de 2026-08-26.
- **Impacto:** os 13 blocos gerados, `profile-snapshot.svg` e a matriz de
  linguagens estão congelados há um mês. Sites que caíram continuariam
  anunciados; sites que voltaram continuam escondidos.
- **Causa:** `PROFILE_README_TOKEN` não está configurado; o workflow trata a
  ausência como caso normal em vez de falha.
- **Recomendação:** configurar o secret (PAT *fine-grained*, só leitura de
  metadados) **e** fazer o job falhar ou abrir aviso visível quando ele faltar
  por mais de N dias. Verde que não fez nada é o pior estado possível.

### A2 · alto — README e catálogo divergem

- **Evidência:** `docs/project-catalog.json` tem 89 projetos (70 públicos,
  19 privados); o README tem 79 (62/17). Onze projetos estão no catálogo e não
  no README (`BlockForge`, `Cosmos`, `Cronicas-da-Baluarte-Onde-os-Deuses-Sangram`,
  `FLUX`, `FanVerse`, `NEXORA`, `Projeto_01_endless_gnome`, `Projetos-Futuros`,
  `PromoRadar`, `Save-Games-Spartan-Gamer-BR-`, `Subnautica-Unhinged-mod-`);
  `Atividade4` está no README e não no catálogo; cinco projetos têm status
  diferente nos dois (`CodeVibe-Academy`, `Cookie-Clicker-Bot`,
  `Essence-Custom-Furniture`, `MOD-PACK-MINE-BACKUP`,
  `Portifolio-Baluarte-Lucas-Belucci-Bellini-`).
- **Causa provável:** o PR #21 gerou o catálogo e parte dos blocos a partir de
  entradas diferentes, e o refresh diário que reconciliaria os dois não roda (A1).
- **Impacto:** `docs/PROJECT-CATALOG.md` promete que "os dois nunca discordam";
  qualquer consumidor do JSON vê outra realidade que o visitante do README.

### A3 · alto — quatro definições de "ecossistema"

| Coletor | Regra de inventário | Número publicado |
|:---|:---|---:|
| `update_profile` (README) | owner + collaborator + org, com forks, com exclusões, inclui o próprio perfil | **79** (62 públicos) |
| `update_profile` (catálogo, outra execução) | idem | **89** (70 públicos) |
| `lang_stats` (`lang-stats.svg`, `profile-top-langs.svg`) | owner, sem forks, sem exclusões; com `GITHUB_TOKEN` cai para só públicos | **69** |
| `ecosystem_watch` | público, owner, sem forks, sem o perfil | **67** |
| `profile_cards` | GraphQL `privacy: PUBLIC, ownerAffiliations: OWNER` | (usado no card de troféus) |

Consequência visível: a seção *MATRIZ DE LINGUAGENS* mostra uma tabela com
"17 linguagens · 62 repositórios · 16,59 MB" e, logo abaixo, a imagem
`lang-stats.svg` com "19 linguagens · 69 repositórios · 23,34 MB". Nenhum dos
números está errado dentro da própria regra — o erro é não haver regra comum.

### A4 · alto — dois escritores para `assets/lang-stats.svg`

`update_profile.render_svg()` escreve uma matriz 1100×620 ("LANGUAGE MATRIX");
`lang_stats.build_svg()` escreve um painel 880×660 ("ARSENAL // ANÁLISE DO
ECOSSISTEMA"). Ambos os workflows fazem `git add` do arquivo. Hoje vence o
`lang_stats` porque o outro não roda (A1); quando o secret for configurado, o
arquivo vai alternar de design e de números entre segunda e domingo.

### A5 · médio — `replace_block` quebra com barra invertida

`replace_block()` passa o corpo renderizado como *string de substituição* para
`re.sub`, que interpreta `\U`, `\d`, `\g<…>`. Reproduzido:

```text
replace_block(texto, "X", r"C:\Users\lucas tool")  →  re.error: bad escape \U
```

Qualquer descrição de repositório com `\` derruba o refresh inteiro. O
`lang_stats.py` já conhece o problema e fatia a string em vez de usar `re.sub`
(comentário na linha 634). Correção de uma linha: `pattern.sub(lambda _: replacement, text, count=1)`.

### A6 · médio — `classify()` casa substrings, não palavras

`"ai" in texto` casa "d**ai**ly", "em**ai**l", "det**ai**ls", "m**ai**ntain";
`"java"` casa "**java**script"; `"game"` casa "save-**game**s". Reproduzido:

| Entrada | Rótulo atual |
|:---|:---|
| `DailyPlanner` — "Agenda diária em TypeScript/Vite" | **IA & Automação** (é o exemplo exibido no card *AI & AUTOMATION* do README) |
| `Mail-Helper` — "Email automation" | IA & Automação |
| `Portfolio` — "Personal site built with JavaScript" | **Academia** |
| `Project-Baluarte-DevFlow` | casa "ai" por substring (o rótulo final vem de "baluarte") |

Corrigir muda a saída pública — é uma decisão editorial, não um refactor.
Por isso a migração congela o comportamento atual como `py-classify@1` e a
correção entra como versão nova, com diff revisado (D-006).

### A7 · médio — o carimbo "determinístico" não é determinístico

`generated_at` é o maior `pushed_at` do inventário, para que dados iguais não
gerem commit. Mas o inventário **inclui o próprio repositório do perfil**, cujo
`pushed_at` muda a cada snapshot horário do monitor. Quando o refresh voltar a
rodar, a linha "atualizado em …" e `profile-snapshot.svg` mudarão todo dia,
com ou sem dado novo. O `ecosystem_watch` já exclui o próprio perfil; o
`update_profile` não.

### A8 · médio — corrida de `git push` entre workflows

Ver seção 4. Recomendação de curto prazo: um grupo de concorrência comum
(`profile-writes`) para todo workflow que escreve no `main` e
`git pull --rebase` antes do push. De longo prazo: um único pipeline que
coleta, grava no banco e publica uma vez.

### A9 · baixo — caminho órfão no `lang_stats.py`

O script procura `<!-- LANG-STATS:START -->`, que o README não tem desde a
restauração de agosto; imprime aviso e segue. O workflow ainda faz
`git add README.md`. A docstring manda configurar `LANG_STATS_TOKEN`, que o
workflow nunca repassa — o script sempre roda só com repositórios públicos.

### A10 · baixo — a timeline depende do shields.io

`contributions-timeline.yml` roda `validate_language_badges.py` (HTTP em ~17
badges do shields.io) antes de publicar. Uma instabilidade do shields.io
bloqueia a publicação de um gráfico que não tem badge nenhum.

### A11 · baixo — código morto e validadores soltos

`restore_original_style.py` (migração única, caminho fixo, marcadores que não
existem mais) e dois validadores que nenhum workflow executa. O
`validate_profile.py` ainda grava um arquivo não ignorado pelo Git.

### A12 · baixo — `update-profile.yml` sem limites

Sem `timeout-minutes` (padrão do GitHub: 6 h) e sem `concurrency`. A lista de
`grep -q` de marcadores omite `FEATURED-PROJECTS`, `PUBLIC-PROJECTS` e
`PRIVATE-PROJECTS` (o `validate_dynamic_sections.py` os cobre).

### A13 · baixo — duas versões de Python

3.11 em `update-profile` e `contributions-timeline`; 3.12 nos demais.

### A14 · info — dado editorial escondido em código

Mudar a prioridade de um projeto, o resumo do card ou a cor de uma linguagem
exige PR de código Python. Adicionar um projeto novo à vitrine curada exige
editar até três lugares (`README_FEATURED.json`, `FEATURED_PRIORITY`,
`FEATURED_SUMMARIES`). É o oposto do objetivo "adicionar projeto sem editar
dezenas de arquivos".

### A15 · info — o único estado irreversível

`docs/ECOSYSTEM-COMMIT-STATE.json` (schema 4) guarda `project_commits = 3118`
e `monitor_commits = 220`, acumulados desde `LEGACY_BASELINE`. O GitHub não
permite recalculá-los. Qualquer migração precisa importá-los com proveniência
(`legacy_import`) antes de desligar o monitor atual.

### A16 · info — o README depende de serviços de terceiros em tempo de leitura

75 badges do shields.io, `skillicons.dev` (10), `capsule-render.vercel.app`,
`readme-typing-svg.demolab.com`, `komarev.com`, `raw.githubusercontent.com`
(branch `output`) e o CDN do Plotly na timeline. Nenhum código do repositório
controla essas quedas; elas aparecem como imagem quebrada.

### A17 · info — três catálogos manuais/derivados não reconciliados

`AGENTS.md` e `ECOSYSTEM-INDEX.md` listam 6 repositórios "canônicos";
o catálogo gerado lista 89; o README, 79. O núcleo precisa de um campo
editorial para "projeto canônico/farol" em vez de uma terceira lista.

### A18 · info — privacidade: o que é publicado de privado

Por decisão do dono do perfil (documentada em `README_DATA.md`), o README
publica **nome, categoria, descrição pública e link** de repositórios privados
— nenhum conteúdo, site ou estrutura interna. (Na prática também publica as
linguagens de alguns privados no `PROJECT-MAP`, contra esse contrato: A20.)
As exclusões só são aplicadas pelo `update_profile`; os outros coletores só
não vazam porque, na prática, rodam sem acesso a privados. Um PAT futuro em
`lang_stats` mudaria isso. No núcleo, a regra fica numa view do banco
(`ecosystem.public_projects`), não em cada coletor.

### A19 · info — status depende do relógio

`status_for()` usa `now`: um projeto passa de 🟢 a 🟡 no 61º dia sem push,
sem nenhum dado novo. Comportamento aceitável, mas a paridade Python × Rust
precisa fixar `now` nos testes.

### A20 · médio — o `PROJECT-MAP` publica linguagens de repositórios privados

- **Evidência:** `render_project_map()` preenche a coluna *Stack* com
  `stack_for(repo, languages)` para **todos** os repositórios, privados
  inclusive. No README atual, seis linhas `🔒 Private` exibem linguagens
  (ex.: `taxforge` → `TypeScript HTML JavaScript Python CSS`;
  `-BANCO-DE-DADOS-` → `PowerShell Shell`).
- **Contrato violado:** `docs/README_DATA.md` — "Repositórios privados podem
  aparecer somente com nome, categoria, descrição pública disponível, status,
  marcador `Private repository` e link do GitHub."
- **Sensibilidade:** baixa (é o mapa de linguagens, não conteúdo), mas é
  exatamente o tipo de divergência entre regra escrita e código que o núcleo
  precisa eliminar. O dono decide qual lado muda: o contrato (autorizar
  linguagens de privados) ou o gerador (coluna `—` para privados). A view
  `ecosystem.private_repository_listing` segue o contrato escrito e **não**
  expõe linguagens — até a decisão, a paridade (D-006) deste bloco fica
  pendente.

### A21 · médio — o monitor conta como "no ar" redirecionamentos que falham

*Encontrado na Fase 2, ao portar a verificação (2026-09-26).*

- **Evidência:** `check_website()` trata como `verified` qualquer `HTTPError`
  cujo código esteja em `LIVE_STATUSES` (200, 301, 302, 307, 308). O `urllib`
  levanta `HTTPError` **com o código do redirect** em três situações em que o
  site não chega a responder: laço (4 visitas à mesma URL ou 11 destinos
  distintos), 3xx sem `Location`, e redirect para esquema proibido
  (`mailto:`, `javascript:`, `file:`, `data:`). Nos três casos o site é
  publicado como online e ganha o CTA primário. O 303 sem `Location` sai
  `unreachable` só porque 303 não está na lista — a regra não é nem coerente.
- **Prova:** cenários `/r/loop`, `/r/no-location-302`, `/r/mailto` e
  `/r/distinct/0` de `tests/e2e/check_sites_parity.py` — todos `verified`.
- **Paridade:** o Rust reproduz de propósito (`py-check@1`, D-006). Corrigir
  é decisão editorial (um laço de redirect deve tirar o site do ar?) e entra
  como versão nova, com o diff revisado.

### A22 · médio — uma URL ou resposta malformada derruba o `check_websites.py` inteiro

*Encontrado na Fase 2 (2026-09-26).*

- **Evidência:** `check_website()` captura `URLError`, `TimeoutError`,
  `ValueError` e `OSError`. Escapam as `http.client.HTTPException` que não são
  nenhuma delas: `InvalidURL` (porta não numérica `https://x.org:abc/`, senha
  na URL `https://u:p@x.org/`, caractere de controle no host ou no caminho) e
  `BadStatusLine`/`LineTooLong` de um servidor que responde fora do HTTP. A
  exceção sobe pelo `ThreadPoolExecutor.map` e derruba o script **sem
  relatório** — no `update_profile.py`, derruba a geração do catálogo. Um
  `project-catalog.json` com formato inesperado (lista na raiz, `projects`
  que não é lista) também quebra o `collect_urls()` com `AttributeError` ou
  `TypeError`.
- **Rust:** não reproduz a queda (não há saída com que ter paridade): a URL
  conta como fora do ar (`error_kind` `invalid_url` ou `other`) e o stderr
  avisa que o Python quebraria; catálogo malformado sai com código 2 e a
  exceção que o Python levantaria.

### A23 · médio — estado ilegível do monitor zera o contador acumulado

*Encontrado na Fase 3, ao portar o monitor de commits (2026-09-26).*

- **Evidência:** `ecosystem_watch.main()` lê o estado anterior com
  `try: json.loads(...) except Exception: previous_state = {}`. Um
  `ECOSYSTEM-COMMIT-STATE.json` truncado ou com um conflito de merge não
  derruba a varredura: ela recomeça do zero, com `project_commits = 1538`
  (`LEGACY_BASELINE`) e `monitor_commits = 0`, e **publica** esse estado — o
  contador exibido no perfil cai de 3.364 para ~1.540 sem nenhum aviso, e o
  histórico acumulado só existe no Git.
- **Prova:** cenário "estado ilegível volta ao baseline 1538" de
  `tests/fixtures/parity/monitor.json`, gerado pelo script real.
- **Paridade:** o `profile-core sync commits` reproduz (D-006). Com banco, os
  contadores ficam também em `metric_samples`, então a queda aparece como uma
  amostra nova e o valor anterior continua consultável. A correção
  (recusar estado ilegível, com código de saída) muda o comportamento e entra
  como versão nova do monitor.

### A24 · baixo — o estado do monitor grava o texto de exceções do Python

*Encontrado na Fase 3 (2026-09-26).*

- **Evidência:** qualquer exceção de `latest_commit()` vira
  `{"branch": …, "error": str(exc)[:180]}` no estado. Além do `HTTP Error 409:
  Conflict` esperado (repositório vazio), um formato inesperado da API grava
  mensagens internas do CPython: `"list index out of range"` (commit com
  mensagem vazia), `"'NoneType' object has no attribute 'get'"` (`commit:
  null`), `"string indices must be integers, not 'str'"`. O estado entra na
  comparação semântica: se uma versão nova do Python mudar a redação de uma
  dessas mensagens, a varredura seguinte vê "mudança", publica um snapshot e
  soma 1 ao contador do monitor sem nada ter mudado nos projetos.
- **Paridade:** o Rust grava as mesmas mensagens (as do 3.12, conferidas pelo
  fixture) enquanto o JSON for o estado de referência. No banco, o erro fica
  em `commit_observations.error_message` e o texto não conta como contador.

### A25 · baixo — uma falha passageira nas linguagens apaga as linguagens do repositório

*Encontrado na Fase 3 (2026-09-26).*

- **Evidência:** `update_profile.fetch_languages()` devolve `{}` para
  qualquer erro (HTTP, rede, JSON). Um 502 momentâneo tira as linguagens
  daquele repositório da tabela, dos badges, do `PROJECT-MAP` e das
  contagens da execução — que é publicada.
- **No núcleo:** `sync github` distingue "a consulta falhou" de "não há
  linguagem" e mantém o mapa anterior no banco (D-031). O README continua
  gerado pelo Python até a Fase 4, com o comportamento atual.

### Evidência adicional de A1/A2 — blocos de versões diferentes do gerador

O bloco `ARSENAL-STACK` no `main` não é o que o `render_arsenal_stack()` atual
produz: falta o `<details>` e a frase de abertura é outra. Ou seja, o README
publicado mistura blocos renderizados por versões diferentes do código — o
que só acontece quando o refresh não roda depois de o renderizador mudar.

---

## 10. O que pode quebrar o README

| Risco | Mecanismo | Proteção atual |
|:---|:---|:---|
| Marcador removido/duplicado | `replace_block` levanta `ValueError` | job falha antes do commit (fail-closed) ✔ |
| Edição manual dentro de marcador | sobrescrita no próximo refresh | documentação ✔ (só quando o refresh roda — A1) |
| Refresh sem PAT apagando privados | `--write` recusa sem token | ✔ |
| `\` em descrição do GitHub | `re.error` (A5) | ✘ |
| Inventário público-parcial | `lang_stats` com `GITHUB_TOKEN` | ✘ números divergem (A3) |
| Dois escritores do mesmo SVG | A4 | ✘ |
| Mudança fora dos blocos | `validate_dynamic_sections.py` | ✔ |
| Site caído anunciado | "falha fecha" + `validate_project_links.py` | ✔ |
| Exclusão reaparecendo | `validate_exclusions.py` | ✔ (substring; nomes curtos poderiam dar falso positivo) |
| Quedas de terceiros | A16 | ✘ (fora do controle do repositório) |
| Push concorrente | A8 | ✘ (execução perdida, sem corrupção) |

---

## 11. O que precisa continuar funcionando durante a migração

Invariantes que a migração **não pode** quebrar — cada um tem ou terá teste:

1. Os 13 marcadores `START/END` do README e tudo fora deles (identidade
   visual, textos, âncoras `#produtos`, `#sites`, `#baluarte`, `#ecossistema`,
   `#arsenal`, `#engenharia`, `#contato`).
2. Caminhos públicos: `assets/*.svg`, `docs/project-catalog.json`
   (`project-catalog@1`), `docs/ECOSYSTEM-COMMIT-STATE.json` (schema 4),
   `docs/ECOSYSTEM-COMMIT-MONITOR.md`, `docs/assets/contributions-timeline-*`,
   branch `output`.
3. Regra de CTA: site verificado primeiro, código sempre visível.
4. Nenhuma URL deduzida do nome do repositório.
5. Privado nunca anuncia site; exclusões nunca aparecem.
6. Dado igual → artefato igual → nenhum commit.
7. Contadores do monitor monotônicos e nunca reiniciados.
8. O CI continua sem precisar do banco até a fase em que o banco for
   explicitamente promovido (D-009).
9. Os 57 testes Python continuam passando até cada script ser formalmente
   descontinuado.

---

## 12. Recomendações

### Fase 0 — estabilização (Python, antes de qualquer Rust)

Pequenas e independentes. A recomendação original era um PR por item; como
o ambiente de trabalho só publica num branch, elas entraram no PR da
auditoria com **um commit por item**, para revisão e reversão separadas.
Estado em 2026-09-25:

| # | Ação | Achado | Estado |
|:--|:---|:---|:---|
| 0.1 | Configurar `PROFILE_README_TOKEN` (fine-grained, *Metadata: read*) | A1 | ⏳ **dono do perfil** (secret do repositório) |
| 0.2 | Fazer `update-profile.yml` falhar quando o secret faltar | A1 | ✅ `0a5c045` — testes e validações rodam, e o job termina vermelho com `::error::` |
| 0.3 | `replace_block` com função de substituição + teste com `\` | A5 | ✅ `0d64829` — `\g<0>` era pior que o erro: substituía em silêncio |
| 0.4 | Um único escritor de `lang-stats.svg` | A3, A4 | ✅ `29a2cc9` — **revisado:** o escritor que fica é o `lang_stats` (ver abaixo) |
| 0.5 | Excluir o próprio perfil do cálculo de `generated_at` | A7 | ✅ `5df54d3` |
| 0.6 | Push resistente à corrida nos 4 escritores | A8 | ✅ `0a5c045` — **revisado:** rebase com nova tentativa, sem grupo comum (D-022) |
| 0.7 | Remover o caminho `LANG-STATS` órfão e a menção a `LANG_STATS_TOKEN` | A9 | ✅ `29a2cc9` — e o `lang_stats` passou a aplicar as exclusões (A18) |
| 0.8 | Tirar `validate_language_badges.py` da timeline | A10 | ✅ `0a5c045` |
| 0.9 | Arquivar `restore_original_style.py`; `validate_restored_style.py` no CI | A11 | ✅ `f8b5592` |
| 0.10 | `timeout-minutes` e `concurrency` em `update-profile.yml`; Python único | A12, A13 | ✅ `0a5c045` — e os 13 marcadores conferidos num laço |
| 0.11 | Fixtures golden do Python (entrada fixa → README/JSON) | §6 | ✅ `1156461` — `--root`, `--now`, `--site-checks-fixture` |
| 0.12 | Linguagens de privados no `PROJECT-MAP`: mudar o contrato ou o gerador | A20 | ⏳ **decisão do dono** |

**Revisão de 0.4.** A recomendação original era manter o `update_profile`
como escritor. Ao executar, o histórico mostrou que o painel atual do
`lang-stats.svg` foi um redesign deliberado do `lang_stats.py` (`d2c0d58`,
20/08) e que o `update_profile.py` passou a gravar o mesmo arquivo cinco dias
depois (`a8ea902`). Manter o `update_profile` apagaria o design publicado —
contra a regra "não destrua o design existente". O `lang_stats` ficou como
único escritor (D-021). A divergência de números entre a tabela e o painel
(A3) continua até o inventário único do núcleo.

**Revisão de 0.6.** O grupo de concorrência comum foi descartado: no GitHub,
um run novo cancela o run *pendente* do mesmo grupo, e o monitor horário
poderia cancelar o refresh diário na fila. Como os quatro escritores
commitam arquivos disjuntos, `pull --rebase` com nova tentativa resolve a
corrida sem esse risco (D-022).

**Efeito colateral esperado de 0.2:** até o secret do item 0.1 existir, o
workflow *Refresh profile README* fica **vermelho** todo dia. É o objetivo:
antes ele ficava verde sem atualizar nada.

A correção de `classify()` (A6) **não** está na Fase 0: muda a saída pública e
entra como `classifier@2` com diff revisado, depois da paridade.

### Fases seguintes

Ver [PYTHON-TO-RUST.md](../migration/PYTHON-TO-RUST.md) e
[MIGRATION-MATRIX.md](../migration/MIGRATION-MATRIX.md).
