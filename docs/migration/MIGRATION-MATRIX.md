# Matriz de migração

Baseada no código real de `28e88af` (14 scripts, 3.799 linhas). Estados:
`planned` · `shadow` (modo B) · `switched` (modo C) · `deprecated` ·
`removed` · `keep` · `retire`. Prioridade segue as fases do
[plano](PYTHON-TO-RUST.md#5-fases).

## 1. Por arquivo

| Python | Linhas | Função | Destino Rust | Comando | Prioridade | Fase | Estado |
|:---|---:|:---|:---|:---|:---|:--:|:---|
| `scripts/project_catalog.py` | 501 | verificação de sites, descoberta, `Presentation`, prioridade, catálogo JSON, CTAs | `site-monitor` (verificação) · `ecosystem-domain` (apresentação, prioridade, CTA, taxonomia) · `catalog` (JSON @1) · `profile-render` (badges/CTAs) | `check sites`, `catalog build` | **alta** | 2–3 | planned |
| `scripts/check_websites.py` | 123 | CLI de saúde dos sites | `profile-core check sites` | `check sites` | **alta — primeira fatia** (D-014) | 2 | planned |
| `.github/scripts/ecosystem_watch.py` | 280 | monitor horário, estado cumulativo | `github-client` + `store` (`commit_observations`, métricas) | `sync commits`, `export legacy-state` | **alta** | 3 | planned |
| `scripts/update_profile.py` — coleta | ~250 | inventário, linguagens, exclusões | `github-client` + `catalog` | `sync github` | **alta** | 3 | planned |
| `scripts/update_profile.py` — regras | ~150 | `classify`, `status_for`, `featured_score` | `ecosystem-domain` | — | **alta** | 3 | planned |
| `scripts/update_profile.py` — render | ~700 | 13 blocos, 2 SVGs | `profile-render` | `render readme`, `render assets` | alta, **por último** | 4 | planned |
| `.github/scripts/lang_stats.py` | 555 | linguagens (duplicado), tipos de arquivo, 2 SVGs (único escritor — D-021) | linguagens: absorvidas por `sync github`; tipos de arquivo: métrica opcional; SVGs: `profile-render` | `render assets` | média | 3–4 | planned (bloco `LANG-STATS` órfão removido na Fase 0) |
| `.github/scripts/profile_cards.py` | 207 | GraphQL de contribuições + 4 SVGs | `github-client` + `metrics`; SVGs em `profile-render`; `profile-projects.svg` passa a sair do catálogo | `sync contributions`, `render assets` | média | 3–4 | planned |
| `scripts/update_contribution_timeline.py` | 178 | GraphQL mensal, JSON + HTML | coleta: `sync contributions`; HTML: template mantido, lendo JSON exportado | `sync contributions` | média | 3 | planned |
| `scripts/validate_project_links.py` | 120 | regra de CTA | `validate links` | `validate links` | média | 4 | **keep** como oráculo até a fase D (D-015) |
| `scripts/validate_dynamic_sections.py` | 58 | só blocos mudaram | `validate readme` / `render readme --check` | `validate readme` | média | 4 | **keep** como oráculo |
| `scripts/validate_exclusions.py` | 16 | exclusões ausentes | `validate exclusions` + view do banco | `validate exclusions` | baixa | 4 | **keep** como oráculo |
| `scripts/validate_language_badges.py` | 123 | contrato dos badges + HTTP shields | `validate badges` | `validate badges` | baixa | 4 | planned |
| `scripts/validate_profile.py` | 208 | validador amplo, fora do CI, caminho fixo, grava arquivo | absorvido por `validate` | `validate` | baixa | 4 | planned → **retire** |
| `scripts/validate_restored_style.py` | 57 | componentes visuais | `validate readme --visual` | — | baixa | 4 | **keep** — no CI desde a Fase 0 (V2 Validation) |
| `scripts/restore_original_style.py` | 129 | migração única de agosto | — | — | — | 0 | **removed** na Fase 0 (`f8b5592`; histórico em `faff9ef`) |
| `tests/*.py` | — | 82 testes (57 + 25 da Fase 0, incluindo o golden) | casos equivalentes em `cargo test`; o golden vira o teste de paridade | — | — | 2–4 | **keep** até o script coberto sair |

## 2. Por função — `update_profile.py`

| Função | Destino | Observação para a paridade |
|:---|:---|:---|
| `api_get`, `fetch_repositories`, `fetch_languages` | `github-client` | unifica 5 clientes HTTP; ETag e rate limit |
| `load_local_repositories`, `load_local_languages` | `profile-core --offline --fixtures` | mesmo formato de arquivo |
| `normalize_name` | `ecosystem-domain::text` | |
| `classify` | `ecosystem-domain::classify::py_v1` | **reproduz a correspondência por substring** (A6); `classifier@2` depois |
| `status_for` | `ecosystem-domain::lifecycle` | `now` injetado; nomes fixos viram `lifecycle_override` |
| `language_rows`, `format_bytes` | `catalog` / view `public_language_totals` | mesmo arredondamento e ordenação (`-bytes`, nome minúsculo) |
| `featured_score`, `FEATURED_PRIORITY` | `ecosystem-domain::priority` | prioridade vira dado (`featured_entries.priority`) |
| `describe`, `md_cell`, `repo_link`, `stack_for` | `profile-render::markdown` | |
| `featured_order`, `featured_priority`, `build_presentations` | `ecosystem-domain::presentation` | |
| `live_site_map` | view `website_status_current` | |
| `replace_block` | `profile-render::markers` | defeito do `\` corrigido na Fase 0 (`0d64829`); o golden cobre |
| `render_dashboard` … `render_project_map` (13) | `profile-render::blocks::*` | um módulo por marcador; `PROJECT-MAP` depende da decisão A20 |
| `render_snapshot_svg` | `profile-render::svg` | `render_svg` foi removido na Fase 0: `lang-stats.svg` é do `lang_stats` (D-021) |
| `load_json_object`, `load_excluded_names`, `load_site_overrides` | `profile-core import manifests` | validação de manifesto aborta a etapa |
| `build_data`, `main` | `profile-core` (`sync github`, `render …`) | recusa escrever sem inventário completo (mesma regra do `--write`) |

## 3. Por função — `project_catalog.py`

| Função / tipo | Destino |
|:---|:---|
| `WebsiteCheck`, `check_website`, `check_websites`, `LIVE_STATUSES` | `site-monitor` (+ tempo de resposta, contagem de redirects, tipo de erro) |
| `_looks_like_http_url` | `ecosystem-domain::url` (mesma expressão da `CHECK` em `websites.url`) |
| `normalize_site_overrides`, `discover_project_website` | `ecosystem-domain::discovery` + `import manifests` |
| `Presentation`, `resolve_presentation`, `marketing_priority` | `ecosystem-domain::presentation` |
| `CATEGORIES`, `CATEGORY_ALIASES`, `canonical_category` | tabelas `categories` / `classification_labels` (migration 0003) |
| `catalog_entry`, `build_catalog`, `write_catalog_if_changed` | `catalog::json_v1` |
| `_badge`, `cta_buttons`, `status_pill`, `cta_cell` | `profile-render::badges` |

## 4. Dados embutidos no código → banco

| Constante | Arquivo | Destino |
|:---|:---|:---|
| `FEATURED_PRIORITY` | `update_profile.py` | `featured_entries.priority` |
| `FEATURED_SUMMARIES` | `update_profile.py` | `projects.summary` |
| `LANGUAGE_DISPLAY`, `LANGUAGE_COLORS` | `update_profile.py` | `languages` (migration 0002) |
| `LANG_COLORS`, `FAMILY_COLORS`, `FAMILIA_EXT` | `lang_stats.py` | `languages.chart_color`; famílias de extensão ficam em código de render |
| `DOMAINS` | `update_profile.py` | `showcase_domains` (migration 0003) |
| palavras-chave de `classify()` | `update_profile.py` | `ecosystem-domain` (versionadas) |
| nomes fixos de `status_for()` | `update_profile.py` | `projects.lifecycle_override` |
| `category_order` | `update_profile.render_arsenal_stack` | `stack_categories` (migration 0006) |
| lista de `projects_svg()` | `profile_cards.py` | derivada de `featured_entries` |
| `LEGACY_BASELINE = 1538` | `ecosystem_watch.py` | `metric_samples` (`legacy_import`) |

## 5. Workflows

| Workflow | Hoje | Alvo |
|:---|:---|:---|
| `update-profile.yml` | Python, pulado sem secret | `profile-core sync all && profile-core render all --write` (um commit) |
| `lang-stats.yml` | Python semanal | absorvido pelo job acima |
| `ecosystem-watch.yml` | Python horário | `profile-core sync commits` horário (grava no banco; commit só do export de compatibilidade enquanto existir) |
| `contributions-timeline.yml` | Python diário | `profile-core sync contributions` dentro do job diário |
| `snake.yml` | action externa | inalterado |
| `v2-validation.yml` | testes Python + validadores | + `cargo test` + `profile-core parity` enquanto houver Python |
| `db-validation.yml` | **novo** (este PR) | inalterado |
