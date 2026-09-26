# Matriz de migração

Baseada no código real de `28e88af` (14 scripts, 3.799 linhas). Estados:
`planned` · `ported` (port Rust com paridade provada pelo fixture, ainda fora do
caminho de publicação) · `shadow` (modo B) · `switched` (modo C) ·
`deprecated` · `removed` · `keep` · `retire`. Prioridade segue as fases do
[plano](PYTHON-TO-RUST.md#5-fases).

## 1. Por arquivo

| Python | Linhas | Função | Destino Rust | Comando | Prioridade | Fase | Estado |
|:---|---:|:---|:---|:---|:---|:--:|:---|
| `scripts/project_catalog.py` | 501 | verificação de sites, descoberta, `Presentation`, prioridade, catálogo JSON, CTAs | `site-monitor` (verificação) · `ecosystem-domain` (apresentação, prioridade, CTA, taxonomia) · `catalog` (JSON @1) · `profile-render` (badges/CTAs) | `check sites`, `catalog build` | **alta** | 2–3 | regras **ported** (`e39c9f2`); verificação HTTP **shadow** (`5daa524`); catálogo **shadow** (`catalog build`, *Core Shadow*); badges e CTAs **shadow** (`profile-render::badges`, Fase 4) |
| `scripts/check_websites.py` | 123 | CLI de saúde dos sites | `profile-core check sites` | `check sites` | **alta — primeira fatia** (D-014) | 2 | **shadow** desde 2026-09-26 (*Site Monitor Shadow*); sai com 14 execuções sem diferença |
| `.github/scripts/ecosystem_watch.py` | 280 | monitor horário, estado cumulativo | `github-client` + `store` (`commit_observations`, métricas) | `sync commits` (e `export legacy-state` na virada, D-030) | **alta** | 3 | **shadow** desde a Fase 3 (*Core Shadow*): mesmo estado, relatório e contadores |
| `scripts/update_profile.py` — coleta | ~250 | inventário, linguagens, exclusões | `github-client` + `catalog` | `sync github`, `catalog build` | **alta** | 3 | **shadow**: o catálogo sai igual (`catalog build`); o inventário vai para o banco (`sync github`) |
| `scripts/update_profile.py` — regras | ~150 | `classify`, `status_for`, `featured_score`, `describe` | `ecosystem-domain` | — | **alta** | 1 → 3 | **ported** (`e39c9f2`, D-023); entra no caminho de publicação com `sync github` |
| `scripts/update_profile.py` — render | ~700 | 13 blocos, `profile-snapshot.svg` | `profile-render` | `render readme` | alta, **por último** | 4 | **shadow** desde a Fase 4 (*Core Shadow* compara README, snapshot, catálogo e saída); validadores verdes sobre a saída do Rust (D-035) |
| `.github/scripts/lang_stats.py` | 555 | linguagens (duplicado), tipos de arquivo, 2 SVGs (único escritor — D-021) | coleta: `profile-core::assets` (`Retry::LangStats`); SVGs: `profile-render::lang_stats`; no banco, linguagens já vêm do `sync github` | `render lang-stats` | média | 4 | **shadow** desde a Fase 4 (D-036; A26 reproduzido) |
| `.github/scripts/profile_cards.py` | 207 | GraphQL de contribuições + 4 SVGs | coleta: `profile-core::assets`; SVGs: `profile-render::cards`; `profile-projects.svg` sai da curadoria na Fase 5 | `render cards` | média | 4 | **shadow** desde a Fase 4 (D-036; A27 reproduzido) |
| `scripts/update_contribution_timeline.py` | 178 | GraphQL mensal, JSON + HTML | coleta: `sync contributions`; HTML: o template do Python (`crates/profile-core/templates/`) | `sync contributions` | média | 3 | **shadow** desde a Fase 3: JSON e HTML iguais; amostras no banco (D-033) |
| `scripts/validate_project_links.py` | 120 | regra de CTA | `validate links` | `validate links` | média | 4 | **keep** como oráculo até a fase D (D-015); roda sobre a saída do Rust (`readme_validators.py`) |
| `scripts/validate_dynamic_sections.py` | 58 | só blocos mudaram | `validate readme` / `render readme --check` | `validate readme` | média | 4 | **keep** como oráculo; roda sobre a saída do Rust |
| `scripts/validate_exclusions.py` | 16 | exclusões ausentes | `validate exclusions` + view do banco | `validate exclusions` | baixa | 4 | **keep** como oráculo; roda sobre a saída do Rust |
| `scripts/validate_language_badges.py` | 123 | contrato dos badges + HTTP shields | `validate badges` | `validate badges` | baixa | 5 | **keep**; a parte offline roda sobre a saída do Rust |
| `scripts/validate_profile.py` | 208 | validador amplo, fora do CI, caminho fixo, grava arquivo | absorvido por `validate` | `validate` | baixa | 5 | a parte offline roda sobre a saída do Rust; → **retire** |
| `scripts/validate_restored_style.py` | 57 | componentes visuais | `validate readme --visual` | — | baixa | 4 | **keep** — no CI desde a Fase 0 (V2 Validation) |
| `scripts/restore_original_style.py` | 129 | migração única de agosto | — | — | — | 0 | **removed** na Fase 0 (`f8b5592`; histórico em `faff9ef`) |
| `tests/*.py` | — | 103 testes (57 + 25 da Fase 0, incluindo o golden, + 2 do fixture de domínio, 2 do monitor de sites, 12 da Fase 3 e 5 da Fase 4: `--out-dir` no golden e os fixtures do README e dos SVGs) | casos equivalentes em `cargo test`; o golden vira o teste de paridade do render | — | — | 1–4 | **keep** até o script coberto sair |

## 2. Por função — `update_profile.py`

| Função | Destino | Observação para a paridade |
|:---|:---|:---|
| `api_get`, `fetch_repositories`, `fetch_languages` | `github-client` ✅ + `catalog::inventory` ✅ | um cliente, a política de cada script como parâmetro (D-029); ETag fica para quando o custo de rate limit aparecer |
| `load_local_repositories`, `load_local_languages` | `catalog::inventory` ✅ (`--input-repos`, `--languages-dir`) | mesmo formato de arquivo |
| `normalize_name` | `ecosystem-domain::text` | |
| `classify` | `ecosystem-domain::classify::classify_py_v1` ✅ | **reproduz a correspondência por substring** (A6); `classifier@2` depois |
| `status_for` | `ecosystem-domain::lifecycle::status_py_v1` ✅ | `now` injetado; datas lidas pelo port do `fromisoformat` (D-023); nomes fixos viram `lifecycle_override` |
| `language_rows`, `format_bytes` | `profile-render::languages` ✅ / view `public_language_totals` | mesmo arredondamento e ordenação (`-bytes`, nome minúsculo; repetido no inventário conta duas vezes) |
| `featured_score`, `FEATURED_PRIORITY` | `ecosystem-domain::priority::featured_score_py_v1` ✅ | mesmos bits em `f64`; prioridade vira dado (`featured_entries.priority`) |
| `describe` | `ecosystem-domain::presentation::describe` ✅ | é dado (vai cru para o catálogo), não escape |
| `md_cell`, `repo_link`, `stack_for` | `profile-render::pytext::md_cell`, `languages::stack_for` ✅ | |
| `featured_order`, `featured_priority`, `build_presentations` | `ecosystem-domain::presentation` | |
| `live_site_map` | view `website_status_current` | |
| `replace_block` | `profile-render::blocks::replace_block` ✅ | defeito do `\` corrigido na Fase 0 (`0d64829`); o golden e o fixture cobrem |
| `render_dashboard` … `render_project_map` (13) | `profile-render::blocks::*` ✅ | uma função por marcador; `PROJECT-MAP` reproduz A20 até a decisão |
| `render_snapshot_svg`, `source_timestamp` | `profile-render::snapshot` ✅ | `render_svg` foi removido na Fase 0: `lang-stats.svg` é do `lang_stats` (D-021) |
| `load_json_object`, `load_excluded_names`, `load_site_overrides` | `profile-core import manifests` | validação de manifesto aborta a etapa |
| `build_data`, `main` | `profile-core::catalog_build::prepare` + `render readme` ✅ | recusa escrever sem inventário completo (mesma regra do `--write`); códigos de saída do Python (D-034) |

## 3. Por função — `project_catalog.py`

| Função / tipo | Destino |
|:---|:---|
| `WebsiteCheck`, `check_website`, `check_websites`, `LIVE_STATUSES` | `site-monitor::check` ✅ shadow (+ tentativas, redirects, tempo, tipo de erro); a semântica do `urllib` em `pyurl`/`pyrequest`/`redirect` (D-026) |
| `_looks_like_http_url` | `ecosystem-domain::url` ✅ (mesma expressão da `CHECK` em `websites.url`) |
| `normalize_site_overrides`, `discover_project_website` | `ecosystem-domain::discovery` ✅ + `import manifests` |
| `Presentation`, `resolve_presentation`, `marketing_priority` | `ecosystem-domain::presentation` ✅ |
| `CATEGORIES`, `CATEGORY_ALIASES`, `canonical_category` | `ecosystem-domain::taxonomy` ✅ + tabelas `categories` / `classification_labels` (migration 0003); `cargo test -p store` confere que as duas não divergem |
| `catalog_entry` | `ecosystem-domain::presentation::catalog_entry` ✅ (ordem de chaves do Python) |
| `build_catalog`, `write_catalog_if_changed` | `catalog::build`, `render`, `write_if_changed` ✅ (golden byte a byte) |
| `_badge`, `cta_buttons`, `status_pill`, `cta_cell` | `profile-render::badges` ✅ |

## 4. Dados embutidos no código → banco

| Constante | Arquivo | Destino |
|:---|:---|:---|
| `FEATURED_PRIORITY` | `update_profile.py` | `featured_entries.priority` |
| `FEATURED_SUMMARIES` | `update_profile.py` | `projects.summary` (`import legacy` ✅; cópia conferida por teste) |
| `LANGUAGE_DISPLAY`, `LANGUAGE_COLORS` | `update_profile.py` | `languages` (migration 0002) |
| `LANG_COLORS`, `FAMILY_COLORS`, `FAMILIA_EXT` | `lang_stats.py` | `languages.chart_color`; famílias de extensão ficam em código de render |
| `DOMAINS` | `update_profile.py` | `showcase_domains` (migration 0003) |
| palavras-chave de `classify()` | `update_profile.py` | `ecosystem-domain` (versionadas) |
| nomes fixos de `status_for()` | `update_profile.py` | `projects.lifecycle_override` (`import legacy` ✅) |
| `category_order` | `update_profile.render_arsenal_stack` | `stack_categories` (migration 0006) |
| lista de `projects_svg()` | `profile_cards.py` | derivada de `featured_entries` |
| `LEGACY_BASELINE = 1538` | `ecosystem_watch.py` | `metric_samples` (`legacy_import` ✅, com os contadores atuais) |

## 5. Workflows

| Workflow | Hoje | Alvo |
|:---|:---|:---|
| `update-profile.yml` | Python, pulado sem secret | `profile-core sync all && profile-core render all --write` (um commit) |
| `lang-stats.yml` | Python semanal | absorvido pelo job acima |
| `ecosystem-watch.yml` | Python horário | `profile-core sync commits` horário (grava no banco; commit só do export de compatibilidade enquanto existir) |
| `contributions-timeline.yml` | Python diário | `profile-core sync contributions` dentro do job diário |
| `snake.yml` | action externa | inalterado |
| `v2-validation.yml` | testes Python + validadores | + `cargo test` + `profile-core parity` enquanto houver Python |
| `db-validation.yml` | **novo** (Fase 1) | inalterado |
| `rust.yml` (Rust Core) | **novo** (Fase 1): fmt, clippy, `cargo test` com PostgreSQL 16, e2e do `profile-core`; Fase 2: e2e do `check sites` × `check_websites.py`; Fase 3: e2e da coleta × coletores Python (GitHub simulado); Fase 4: README e SVGs no mesmo e2e, e o README do Rust na árvore real sob os validadores | inalterado até a Fase 5 |
| `site-monitor-shadow.yml` | **novo** (Fase 2): modo B diário do monitor — Python e Rust sobre os sites reais | vira o monitor de produção (modo C), gravando histórico, quando sair do modo B |
| `core-shadow.yml` (Core Shadow) | **novo** (Fase 3): modo B diário da coleta — monitor de commits, contribuições e catálogo, Python e Rust sobre os dados reais; Fase 4: README, snapshot e os seis SVGs de `assets/` | some quando cada coletor passar ao modo C |
