# Arquitetura da API

Como outros sistemas (site do Projeto Baluarte, painéis, agentes) vão ler o
catálogo do ecossistema. Decisão de base: D-017 — **começar estático e só
subir um serviço quando houver quem o consuma**.

## 1. Consumidores prováveis

| Consumidor | Precisa de | Frequência |
|:---|:---|:---|
| README do perfil | tudo, mas é gerado no próprio pipeline | diária/horária |
| Site do Projeto Baluarte / portfólio | lista de projetos, sites no ar, destaques | a cada visita (com cache) |
| Painel do ecossistema | histórico de disponibilidade, métricas, execuções | sob demanda |
| Agentes (Claude, JARVIS) via MCP | "quais projetos existem, onde está o site, está no ar?" | sob demanda |

Nenhum deles precisa **escrever** — escrita é do pipeline e dos manifestos.

## 2. Estágios

| Estágio | Forma | Quando |
|:---|:---|:---|
| **v0 · hoje** | `docs/project-catalog.json` versionado no Git (`raw.githubusercontent.com`) | já existe |
| **v1 · estático com contrato** | JSON Schema publicado + arquivos estáticos derivados das views públicas: `projects.json`, `sites.json`, `languages.json` | Fase 3 (junto da coleta em Rust) |
| **v2 · HTTP read-only** | serviço `axum` sobre as views `public_*`, papel `ecosystem_public_reader` | quando um consumidor precisar de dado mais fresco que o commit, ou de histórico |
| **v3 · MCP** | servidor MCP com ferramentas de leitura sobre a mesma camada | quando um agente for consumidor recorrente |
| **v4 · administrativa** | escrita editorial autenticada | só se a autoria editorial sair do Git (D-002) |

O v1 resolve a maior parte dos casos sem servidor, sem custo e sem superfície
de ataque. O custo de pular direto para o v2 é manter um processo no ar para
servir dados que mudam poucas vezes por dia.

## 3. Contrato de dados (v1 e v2)

O contrato **é** a view `ecosystem.public_projects`. Nada fora dela é exposto
publicamente, e ela já aplica privado/excluído/site-só-se-verificado (D-010).

```json
{
  "schema": "lucas-belucci-bellini/projects@2",
  "generated_at": "2026-09-25T04:17:00Z",
  "sync_run": 1234,
  "projects": [
    {
      "slug": "veritas",
      "name": "Veritas",
      "repository": "Lucas-Belucci-Bellini/Veritas",
      "github_url": "https://github.com/Lucas-Belucci-Bellini/Veritas",
      "description": "…",
      "category": "Hardware & Simulation",
      "category_label": "Digital Logic / Hardware",
      "domain": "HARDWARE & LOGIC",
      "status": "active",
      "website": "https://veritas-opal-seven.vercel.app",
      "website_status": "verified",
      "website_checked_at": "2026-09-25T04:10:00Z",
      "primary_cta": "website",
      "secondary_cta": "github",
      "featured": { "position": 2, "label": "DIGITAL LOGIC / LOCAL-FIRST" }
    }
  ]
}
```

Regras do contrato:

- `schema` versionado; mudança incompatível = versão nova, a anterior
  continua publicada por um período anunciado.
- `project-catalog@1` (o arquivo atual) **continua existindo** enquanto houver
  consumidor; `projects@2` é aditivo.
- Campo ausente ≠ `null`: `website` só existe quando verificado (mesma regra
  do catálogo atual).
- Nenhum campo de repositório privado; nenhum dado de repositório excluído.
- `generated_at` só no envelope — nunca por projeto, para não gerar diff sem
  mudança de dado.

## 4. Endpoints (v2, proposta)

```text
GET /v1/projects                  lista (filtros: category, domain, featured, has_website)
GET /v1/projects/{slug}           um projeto
GET /v1/projects/{slug}/uptime    website_uptime_30d do site primário
GET /v1/sites                     website_status_current (só projetos públicos)
GET /v1/languages                 public_language_totals
GET /v1/metrics/{key}             metric_latest (só métricas de escopo profile/ecosystem)
GET /v1/health                    último sync_run de cada tipo, idade, status
```

- **Somente leitura**, `GET`, JSON. Sem autenticação para as rotas públicas;
  rate limit por IP.
- `ETag` = id do último `sync_run` que alterou o recurso; `Cache-Control:
  public, max-age=300`. Um CDN na frente absorve quase todo o tráfego.
- Paginação por cursor em listas (o volume atual cabe numa página, mas o
  contrato não deve depender disso).
- Erros no formato `{"error": {"code": "...", "message": "..."}}`.
- Conexão ao banco com o papel `ecosystem_public_reader`, que só tem `SELECT`
  nas views públicas (testado em `db/tests/sql/040_privileges.sql`). Um bug na
  API não consegue ler tabela.

`/v1/health` é o antídoto de A1: um consumidor (ou um monitor externo) vê que
o refresh parou de rodar mesmo quando o CI mostra verde.

## 5. O que a API não faz

- Não faz proxy para o GitHub nem expõe dado que o GitHub já serve.
- Não publica nada de repositório privado — nem o que o README publica por
  decisão do dono (`private_repository_listing` fica fora da API).
- Não escreve. Curadoria continua por PR nos manifestos até existir o v4.
- Não é pré-requisito do README (D-009).

## 6. Hospedagem (quando chegar o v2)

O mesmo binário `profile-core` com um subcomando `serve`, ou um crate
`ecosystem-api` separado se o tamanho justificar. Opções: container em
serviço gerenciado com escala a zero, ou função serverless que consulta as
views. A decisão depende da hospedagem do banco
([DATABASE-ARCHITECTURE.md §3](../database/DATABASE-ARCHITECTURE.md)) e de haver
consumidor — não antes.
