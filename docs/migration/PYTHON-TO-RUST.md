# Migração Python → Rust

Plano da migração gradual do núcleo operacional. O "o quê, arquivo por
arquivo" está na [matriz](MIGRATION-MATRIX.md); aqui estão os princípios, a
forma do código Rust, a CLI, as fases e a estratégia de testes.

## 1. Princípios

1. **Rust pelo que traz de correção, não por desempenho** (D-012). O pipeline
   é limitado por I/O e termina em segundos; não há gargalo de CPU. O ganho
   é um modelo de domínio tipado único, SQL checado em compilação, um binário
   para CI e serviço, e concorrência estruturada.
2. **Nada é convertido mecanicamente.** Os 14 scripts viram 7 crates por
   responsabilidade (§3), não 14 módulos espelho.
3. **Paridade antes de correção** (D-006). O Rust reproduz a saída do Python
   byte a byte — defeitos conhecidos inclusive — e só depois corrige, com
   versão nomeada e diff revisado.
4. **Em paralelo, nunca no lugar.** O Python continua gerando o que é
   público até o equivalente Rust ter passado pelo modo sombra (§4).
5. **O menor raio de impacto primeiro.** Monitor de sites antes do README
   (D-014).
6. **Estabilizar antes de migrar.** A Fase 0 corrige no Python os defeitos que
   tornariam a comparação impossível (A1, A3–A5, A7).

## 2. O que vai para Rust, o que fica, o que sai

| Destino | Critério | Itens |
|:---|:---|:---|
| **Rust (núcleo)** | coleta, normalização, regra de negócio, estado, histórico, geração | inventário, linguagens, classificação/status/prioridade, sites, monitor de commits, contribuições, render do README/SVG/catálogo, CLI |
| **Rust (subcomando `validate`)** | contrato verificável do artefato | validadores de CTA, exclusões, blocos dinâmicos, badges |
| **Fica em Python, temporariamente** | oráculo independente durante a transição (D-015) | `validate_dynamic_sections`, `validate_project_links`, `validate_exclusions`, os 57 testes |
| **Fica como template** | apresentação sem lógica | HTML da timeline (Plotly) — passa a ler JSON exportado do banco |
| **Fica em YAML** | cola de CI | workflows (reduzidos a um pipeline no fim — D-018) |
| **Fica como action externa** | não é código deste repositório | `snake.yml` (Platane/snk) |
| **Sai** | obsoleto ou sem executor | `restore_original_style.py`; `validate_profile.py` (absorvido pelo `validate`); `validate_restored_style.py` (vira checagem do `validate readme` ou sai) |

## 3. Forma do código Rust (proposta — D-013)

```text
Cargo.toml                         workspace
crates/
├── ecosystem-domain/   lib  tipos e regras puras, sem I/O
│     Repository, Project, Website, CheckOutcome, Presentation,
│     classify (py-classify@1, classifier@2), lifecycle_status(now),
│     marketing_priority, cta, slug
├── github-client/      lib  REST + GraphQL: paginação, ETag, rate limit,
│                            backoff (hoje: 5 implementações de api())
├── site-monitor/       lib  HTTP: redirects contados, tempo, erro
│                            classificado, retry, concorrência por host
│                            (sem saber de README nem de GitHub: reutilizável)
├── catalog/            lib  inventário → snapshot normalizado;
│                            project-catalog.json (schema @1, paridade)
├── store/              lib  sqlx + migrations embutidas (db/migrations)
├── profile-render/     lib  13 blocos, SVGs, validadores
└── profile-core/       bin  CLI (clap) — orquestra os demais
```

Regras:

- `ecosystem-domain` não depende de nada de I/O (mesma disciplina do
  `src/engine/` do Project Vanguard): é o que permite testar toda regra sem
  rede e sem banco.
- `store` é o único crate que conhece SQL. Consultas com `sqlx::query!`
  (checadas contra o schema em compilação, com `SQLX_OFFLINE` no CI).
- `github-client` e `site-monitor` usam `reqwest` + `tokio`; timeouts e
  concorrência são configuração, não constantes.
- Toda dependência de tempo recebe um relógio injetável (`now`) — a paridade
  com o Python exige fixá-lo (A19).

## 4. Modos de cada componente

Cada componente anda sozinho pelos modos da
[arquitetura](../architecture/SYSTEM-ARCHITECTURE.md#4-modos-de-operação-durante-a-migração):

```text
A · atual        Python gera e publica.
B · sombra       Rust roda no mesmo job, grava no banco, escreve a saída em
                 arquivo temporário; o CI compara com a do Python.
C · virada       Rust publica; validadores Python conferem a saída do Rust.
D · consolidado  script Python marcado deprecated → removido num PR próprio.
```

**Critério para sair de B:** `diff` vazio em 14 execuções agendadas seguidas
(ou diferenças todas explicadas por decisão registrada). **Critério para sair
de C:** 30 dias sem regressão e equivalente Rust de cada validador.

## 5. Fases

| Fase | Entrega | Critério de saída |
|:---|:---|:---|
| **0 · estabilização** (Python) | itens 0.1–0.12 da [auditoria](../audits/2026-09-25-ecosystem-core-audit.md#12-recomendações); fixtures golden | refresh diário rodando de verdade; um escritor por arquivo; fixtures versionadas |
| **1 · fundação** ✅ parcial | auditoria, docs, schema, migrations testadas, CI de banco (**este PR**); depois: workspace Cargo, `ecosystem-domain`, `store`, `profile-core db migrate` | `cargo test` verde; migrations aplicadas pelo binário |
| **2 · monitor de sites** | `site-monitor` + `profile-core check sites` em modo B | relatório JSON idêntico ao de `check_websites.py --json` (exceto `checked_at` e tempo); histórico no banco |
| **3 · coleta** | `github-client` + `catalog` + `sync github`, `sync commits`, `sync contributions`, `import manifests`, `import legacy` | `project-catalog.json` do Rust = do Python; contadores do monitor idênticos |
| **4 · geração** | `profile-render` + `render readme/assets` em modo B → C | README byte a byte igual sobre as mesmas entradas; validadores Python verdes contra a saída do Rust |
| **5 · consolidação** | um workflow, um commit (D-018); scripts Python removidos; correções editoriais (`classifier@2`, A20) | nenhum Python no caminho de publicação |

## 6. Estratégia de testes

```text
OLD PYTHON ──▶ saída esperada (fixture golden, versionada) ◀── NEW RUST
```

| Camada | Ferramenta | O que cobre |
|:---|:---|:---|
| **Unitário Rust** | `cargo test` | `ecosystem-domain`: classificação, status com `now` fixo, prioridade, CTA, slug; renderizadores bloco a bloco |
| **Integração Rust** | `cargo test` + servidor HTTP local (como `tests/test_project_catalog.py`) | `site-monitor`: 200, redirect, 404, 500, conexão recusada, DNS, timeout, URL inválida, URL repetida |
| **Parsing da API do GitHub** | respostas JSON gravadas em `tests/fixtures/github/` | paginação, campos ausentes/nulos, `304`, `409` (repo vazio), rate limit (`403` + `X-RateLimit-Remaining: 0`), GraphQL com `errors` |
| **Banco** | `db/tests/run.sh` (já existe) + `#[sqlx::test]` | restrições, views, histórico append-only, privilégios; no Rust, cada teste num banco descartável |
| **Migrations** | `db/tests/run.sh` + `db-validation.yml` (já existe) | round-trip por migration, guardas, idempotência, sqlx |
| **Paridade (regressão)** | fixture golden: `repos.json` + `languages/` + `site-checks.json` + `now` → README, catálogo, SVGs | `diff` Python × Rust byte a byte |
| **Contrato do README** | validadores Python e Rust | só blocos mudaram, CTA site-primeiro, exclusões ausentes, badges legíveis |
| **Sombra em produção** | modo B no Actions | a mesma comparação, com dado real, a cada execução |

### Fixtures golden

O `update_profile.py` já aceita `--input-repos`, `--languages-dir` e
`--skip-site-check`. Falta um jeito de injetar **resultados** de checagem de
site e o relógio, para que a saída seja determinística com sites "no ar". A
Fase 0 (item 0.11) acrescenta ao Python, sem mudar o comportamento padrão:

```text
--site-checks-fixture FILE   resultados de checagem em vez de HTTP real
--now 2026-09-25T00:00:00Z   relógio fixo para status_for/featured_score
```

As fixtures são **sintéticas** ou anonimizadas — nunca o inventário real com
repositórios privados.

### Testes Python existentes

Os 57 testes **não são removidos** para facilitar a migração. Cada um vira
também um caso do teste Rust equivalente (mesmo nome, mesma entrada); o teste
Python só sai junto com o script que ele cobre, na fase D daquele componente.

## 7. A CLI `profile-core`

```text
profile-core [--database-url URL | --no-db] [--offline --fixtures DIR] [--dry-run] <comando>

  sync all                      github → manifests → commits → contributions → sites
  sync github                   inventário, linguagens, classificação  (Repository Catalog)
  sync commits                  monitor do ecossistema                 (Ecosystem Activity)
  sync contributions            GraphQL por janela mensal              (Contribution Data)

  check sites [--json] [--fail-on-down] [--timeout S] [--workers N]
                                verifica e grava histórico             (Website Monitor)

  import manifests              docs/README_*.json → banco (manifesto vence)
  import legacy                 carga única do estado atual (MIGRATIONS.md §6)

  catalog build [--out docs/project-catalog.json]
  render readme [--write | --check]    só entre marcadores; --check falha se mudaria
  render assets [--write | --check]    SVGs
  render all    [--write | --check]

  validate [readme|catalog|links|exclusions|badges]
  export legacy-state           ECOSYSTEM-COMMIT-STATE.json schema 4 (compatibilidade)

  db migrate | db revert | db status
  parity --against python       roda os dois sobre as mesmas fixtures e mostra o diff
```

Convenções:

- **Sem `--write`, nada é gravado em arquivo** — mesmo padrão do
  `update_profile.py` atual.
- `--no-db` monta o snapshot direto do GitHub (D-009): o README pode ser
  gerado com o banco fora do ar.
- `--offline --fixtures DIR` substitui GitHub e HTTP por arquivos — é o modo
  dos testes de paridade.
- Saída legível por padrão; `--json` em tudo que reporta.
- Código de saída: `0` ok, `1` verificação falhou (ex.: `--fail-on-down`,
  `--check` com diferença), `2` erro de execução. Nunca `0` quando uma etapa
  foi pulada por falta de credencial (lição de A1).

## 8. Descontinuação de um script Python

1. Equivalente Rust em modo B, `diff` vazio pelo critério da §4.
2. Virada (C): o workflow passa a chamar o Rust; o script ganha no topo
   `# DEPRECATED: substituído por profile-core <comando> (PR #…)` e continua no
   repositório.
3. Após o critério de C, PR próprio remove o script, seus testes Python e a
   entrada no workflow, e atualiza a [matriz](MIGRATION-MATRIX.md).

## 9. Riscos

| Risco | Mitigação |
|:---|:---|
| Tempo de compilação no CI | cache do `target/` e do binário; o binário só recompila quando `crates/**` muda |
| Divergência sutil de formatação (floats, ordenação, Unicode) | fixture golden byte a byte; `format_bytes`/percentuais reproduzidos com os mesmos arredondamentos |
| Diferença de biblioteca HTTP (redirects, TLS) | os mesmos casos do servidor local rodam contra os dois |
| Dono do perfil precisa de Rust para contribuir | manifestos continuam em JSON; nada editorial exige Rust |
| Banco fora do ar | `--no-db` e D-009 |
| Contadores do monitor | importados com proveniência; o Python continua escrevendo o JSON até a paridade |
