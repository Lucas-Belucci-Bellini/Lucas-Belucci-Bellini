# Componentes

Os componentes do núcleo, cada um com responsabilidade única, e o que hoje faz
o papel de cada um. Crates e comandos são propostas (D-013); tabelas e views
já existem em `db/migrations`.

| Componente | Responsabilidade | Hoje (Python) | Crate proposto | Comando | Tabelas / views |
|:---|:---|:---|:---|:---|:---|
| [Repository Catalog](#repository-catalog) | inventário único do GitHub | `fetch_repositories` + 3 cópias | `github-client`, `catalog` | `sync github` | `github_owners`, `repositories`, `repository_exclusions` |
| [Project Metadata](#project-metadata) | classificação, status, curadoria, CTA | `classify`, `status_for`, `resolve_presentation` | `ecosystem-domain` | `sync github`, `import manifests` | `projects`, `project_repositories`, `featured_entries`, taxonomia |
| [Website Monitor](#website-monitor) | descobrir, verificar, historiar sites | `project_catalog.check_website(s)`, `check_websites.py` | `site-monitor` | `check sites` | `websites`, `website_checks`, `website_status_current`, `website_uptime_30d` |
| [Language Statistics](#language-statistics) | bytes por linguagem, matriz pública | `language_rows`, `lang_stats.collect` | `catalog` | `sync github` | `languages`, `repository_languages`, `public_language_totals` |
| [Contribution Data](#contribution-data) | contribuições do perfil por janela | `update_contribution_timeline`, `profile_cards.request_data` | `github-client`, `metrics` | `sync contributions` | `metric_samples` (`profile.contributions.*`) |
| [Ecosystem Activity](#ecosystem-activity) | último commit por repositório, contadores | `ecosystem_watch.py` | `github-client`, `metrics` | `sync commits` | `commit_observations`, `repository_head_current`, `metric_samples` (`ecosystem.commits.*`) |
| [Deployment Status](#deployment-status) | eventos de deploy do provedor | — (não existe) | futuro | futuro | planejada (D-016) |
| [Metrics](#metrics) | séries numéricas com janela e proveniência | espalhado em JSON e SVG | `metrics` | — | `metric_definitions`, `metric_samples`, `metric_latest` |
| [Profile Generator](#profile-generator) | README, SVGs, catálogo JSON | 13 `render_*`, `render_svg`, `build_svg`, `*_svg` | `profile-render` | `render readme`, `render assets`, `catalog build` | views públicas |
| [Sync Runs](#sync-runs) | proveniência e saúde de cada execução | `scanned_at`, prints | `store` | todos | `sync_runs`, `sync_run_errors` |
| [Validation](#validation) | contratos do README e do catálogo | 6 `validate_*.py` | `profile-render` (`validate`) | `validate` | — |

---

## Repository Catalog

- **Faz:** uma chamada paginada a `/user/repos` (com ETag), upsert por
  `github_id`, marca `gone_at` em quem sumiu, pula excluídos antes de gravar.
- **Substitui:** as quatro listagens de hoje (A3). A regra de inventário passa
  a ser uma só e explícita: *owner + collaborator + organization_member, forks
  incluídos, excluídos removidos* (paridade com o README). Coletores que
  precisam de um recorte (ex.: monitor: só públicos não-fork) filtram a partir
  dela, não refazem a consulta.
- **Não faz:** ler conteúdo de repositório privado; copiar campos que o
  catálogo não usa.

## Project Metadata

- **Faz:** traduz repositório em projeto. Classificação heurística versionada
  (`py-classify@1` reproduz o Python, inclusive a correspondência por
  substring — D-006), rótulo editorial quando houver, status de ciclo de vida
  calculado com `now` injetado, prioridade de vitrine 0–100, regra de CTA.
- **Dado editorial:** `summary` (hoje `FEATURED_SUMMARIES`), `lifecycle_override`
  (hoje nomes fixos em `status_for`), `publication`, curadoria
  (`featured_entries`, hoje `README_FEATURED.json` + `FEATURED_PRIORITY`).
- **Modelo:** projeto ≠ repositório. Hoje 1:1; aceita um projeto com vários
  repositórios (ex.: Projeto Baluarte + domínios `baluarte-*`), um primário.

## Website Monitor

- **Descobrir:** `homepage` do GitHub → manifesto → nada. Nunca deduz URL do
  nome (regra de `docs/WEBSITE-DISCOVERY.md`, mantida).
- **Verificar:** `GET` com redirecionamentos seguidos e contados, tempo de
  resposta, classificação do erro (`timeout`, `dns`, `connect`, `tls`,
  `http_status`, `invalid_url`), 1 retry com espera curta, concorrência
  limitada por host.
- **Registrar:** uma linha em `website_checks` por checagem, sempre — o que
  muda em relação a hoje é que nada é sobrescrito.
- **Decidir se aparece como online:** paridade — última checagem `verified`.
  Evolução proposta: histerese (ex.: só esconder após 2 falhas seguidas),
  possível agora que há histórico; exige decisão editorial.
- **Alimentar o catálogo:** `website_status_current` → `public_projects`.
- **Reutilizável:** o crate não conhece README nem GitHub; recebe URLs,
  devolve resultados. Serve ao CLI, a um serviço contínuo e a outros projetos
  do ecossistema.

## Language Statistics

- Bytes por linguagem vêm do GitHub; percentual é **calculado**, nunca
  guardado. `public_language_totals` inclui forks e exclui privados/excluídos
  (paridade com a tabela `LANGUAGE-STATS`).
- Rótulo público e cores ficam em `languages` (hoje há duas paletas em dois
  scripts). A contagem de tipos de arquivo do `lang_stats.py` (árvore git)
  continua possível só para repositórios públicos; é dado de apresentação, não
  de catálogo, e entra como métrica se for mantida.

## Contribution Data

Consultas GraphQL por janela mensal → `metric_samples` com `window_start` /
`window_end`. O HTML interativo (Plotly) continua sendo um template: ele só
passa a ler do banco (ou de um JSON exportado dele).

## Ecosystem Activity

Paridade com `ecosystem_watch.py`: último commit do branch padrão de cada
repositório público não-fork, exceto o próprio perfil; `compare` para contar
commits novos; erro por repositório é estado. Os contadores cumulativos são
importados uma vez (`legacy_import`) e passam a ser derivados:
`project_total = baseline + Σ commits_since_previous`. O arquivo
`ECOSYSTEM-COMMIT-STATE.json` continua sendo **exportado** no schema 4 até
nenhum consumidor depender dele.

## Deployment Status

Planejado, não criado (D-016). Site é a URL pública verificada por HTTP;
deployment é o evento do provedor (Vercel, GitHub Pages) com SHA, ambiente e
estado do build. Desenho em [DATA-MODEL.md §4](../database/DATA-MODEL.md).

## Metrics

Uma tabela genérica de amostras com definição tipada (`scope`, `unit`,
`source`) em vez de uma tabela por métrica. Toda amostra tem janela,
proveniência (`collected`, `derived`, `legacy_import`) e a execução que a
produziu. O escopo da definição é imposto por trigger.

## Profile Generator

- Lê **só** das views públicas (e de `private_repository_listing` para o bloco
  privado autorizado). Nunca das tabelas.
- Reescreve só o conteúdo entre marcadores; o resto do README é intocável
  (mesma garantia do `validate_dynamic_sections.py`).
- Byte a byte igual ao Python na paridade (D-006). Sem carimbo de tempo que
  mude sem dado novo — corrigindo A7.
- Detalhe de cada bloco: [README-ANATOMY.md](README-ANATOMY.md).

## Sync Runs

Toda execução abre um `sync_run` e o fecha com `succeeded`, `partial` ou
`failed`, contagens e mensagem. Toda linha de histórico aponta para o seu. É o
que permite responder "por que este site sumiu do README?" com a checagem,
a execução e o SHA do código que decidiu.

## Validation

Os validadores Python viram subcomandos `profile-core validate …`, mas
**continuam rodando** no CI como oráculo independente até a Fase 5 (D-015).
