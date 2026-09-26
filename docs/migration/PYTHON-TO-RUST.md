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

**Existem:** `ecosystem-domain`, `store` e `profile-core` (`db migrate |
revert | status`) desde a Fase 1; `site-monitor` e `profile-core check sites`
desde a Fase 2. Os demais entram nas fases em que são usados.

Regras:

- `ecosystem-domain` não depende de nada de I/O (mesma disciplina do
  `src/engine/` do Project Vanguard): é o que permite testar toda regra sem
  rede e sem banco.
- `store` é o único crate que conhece SQL. Consultas ao schema `ecosystem` com
  `sqlx::query!` (checadas em compilação, com `SQLX_OFFLINE` no CI) quando os
  repositórios de dados chegarem; na Fase 1 a única tabela lida é a de controle
  do sqlx, com consulta em tempo de execução (D-024).
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
| **0 · estabilização** (Python) ✅ parcial | itens 0.2–0.11 feitos; faltam 0.1 (secret) e 0.12 (decisão) — [auditoria](../audits/2026-09-25-ecosystem-core-audit.md#12-recomendações) | refresh diário rodando de verdade (depende de 0.1); um escritor por arquivo ✅; fixtures versionadas ✅ |
| **1 · fundação** ✅ | auditoria, docs, schema, migrations testadas, CI de banco; workspace Cargo, `ecosystem-domain` com paridade (`e39c9f2`, D-023), `store` (`9e133d9`, D-024), `profile-core db migrate\|revert\|status` (`4527264`, `7f6bfe4`), workflow `Rust Core` (`441eaf2`) | `cargo test` verde ✅; migrations aplicadas pelo binário ✅ — e o schema resultante é idêntico ao do psql |
| **2 · monitor de sites** ✅ | `site-monitor` + `profile-core check sites` (`1d2e955`, `5daa524`, D-026 a D-028); modo B no workflow *Site Monitor Shadow* (`b00e33a`) | relatório JSON idêntico ao de `check_websites.py --json` (exceto `checked_at`) ✅ — texto e código de saída também, em 45 cenários locais e nos 16 sites reais; histórico no banco ✅ (`website_checks`, para os sites registrados — D-027). **Sai do modo B** com 14 execuções agendadas seguidas sem diferença |
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
| **Paridade de domínio** ✅ | `tests/test_parity_domain.py` → `tests/fixtures/parity/domain.json` ← `cargo test -p ecosystem-domain --test parity` | 640 casos em 13 grupos, gerados pelas funções Python reais no Python do CI (D-023) |
| **Integração Rust** | `cargo test` + servidor HTTP local (como `tests/test_project_catalog.py`) | `site-monitor`: 200, redirect, 404, 500, conexão recusada, DNS, timeout, URL inválida, URL repetida |
| **Parsing da API do GitHub** | respostas JSON gravadas em `tests/fixtures/github/` | paginação, campos ausentes/nulos, `304`, `409` (repo vazio), rate limit (`403` + `X-RateLimit-Remaining: 0`), GraphQL com `errors` |
| **Banco** | `db/tests/run.sh` + `cargo test -p store` ✅ | restrições, views, histórico append-only, privilégios; no Rust, cada teste cria e apaga o próprio banco (`STORE_TEST_DATABASE_URL`; `STORE_TESTS_REQUIRED=1` no CI impede que pulem) |
| **Binário** ✅ | `cargo test -p profile-core` + `db/tests/profile_core_e2e.sh` | CLI de ponta a ponta; schema do binário = schema do psql (`pg_dump`); testes SQL sobre ele; trava de perda de dados |
| **Paridade do monitor** ✅ | `tests/test_parity_site_monitor.py` → `tests/fixtures/parity/site_monitor.json` ← `cargo test -p site-monitor --test parity` e `-p profile-core --test parity_sites` | 111 casos gerados **chamando** o `urllib`/`http.client` do CPython 3.12.14 e o `check_websites.py` (D-028) |
| **Monitor de ponta a ponta** ✅ | `tests/e2e/check_sites_parity.py` (no Rust Core) | os dois programas contra o mesmo servidor local: 45 cenários, texto, JSON e código de saída |
| **Sombra do monitor** ✅ | workflow *Site Monitor Shadow* + `.github/scripts/compare_site_reports.py` | os dois ao mesmo tempo sobre os sites reais, diariamente |
| **Migrations** | `db/tests/run.sh` + `db-validation.yml` (já existe) | round-trip por migration, guardas, idempotência, sqlx |
| **Paridade (regressão)** | fixture golden: `repos.json` + `languages/` + `site-checks.json` + `now` → README, catálogo, SVGs | `diff` Python × Rust byte a byte |
| **Contrato do README** | validadores Python e Rust | só blocos mudaram, CTA site-primeiro, exclusões ausentes, badges legíveis |
| **Sombra em produção** | modo B no Actions | a mesma comparação, com dado real, a cada execução |

### Fixtures golden

Pronto desde a Fase 0 (`1156461`). O `update_profile.py` aceita, sem mudar o
comportamento padrão:

```text
--root DIR                   README, manifestos e saídas numa raiz alternativa
--input-repos / --languages-dir   inventário e linguagens de arquivo
--site-checks-fixture FILE   resultados de checagem em vez de HTTP real
--now 2026-09-25T12:00:00Z   relógio fixo para status_for/featured_score
```

`tests/fixtures/profile/input` é uma raiz sintética (13 repositórios em
`example.org`, cada um exercitando uma regra) e `expected/` guarda README,
catálogo, `profile-snapshot.svg` e o resumo. `tests/test_golden_profile.py`
compara byte a byte; `UPDATE_GOLDEN=1` regenera. O `profile-core parity` vai
rodar o binário Rust sobre a **mesma** raiz e comparar com o mesmo `expected/`.

As fixtures são **sintéticas** — nunca o inventário real com repositórios
privados.

### Armadilhas de paridade

O que um port mecânico erraria, e como o Rust reproduz. Cada item é caso do
fixture de paridade; a lista cresce a cada componente portado.

| Python | Rust ingênuo | Rust do núcleo |
|:---|:---|:---|
| `round(12.5) == 12` (empate para o par) | `f64::round` → 13 | `round_ties_even` (`priority::py_round`) |
| `str.isspace()`, `strip()`, `split()` incluem U+001C–U+001F | `char::is_whitespace` não inclui | `text::py_is_space`, conferido nos 1,1 milhão de code points |
| `\s` do `re` inclui U+001C–U+001F; `$` casa antes de um `\n` final | `regex` não faz nenhum dos dois | classe explícita e `\n?\z` (`url.rs`) |
| `timedelta.days` arredonda para baixo | truncar: −6 h dá 0 dia; o Python dá −1 | `div_euclid` (`timestamps::python_days`) |
| `datetime.fromisoformat` aceita `10.5`, `+00:60`, `2026-W38-7`, separador multibyte, caractere solto antes do fuso | RFC 3339 recusa | port do C do CPython 3.12 (`timestamps::py_fromisoformat`) |
| `"ai" in "plain"` (A6) | tokenizar "corrigiria" | substring, de propósito (`classify_py_v1`) |
| `str(True) == "True"` | `"true"` | `discovery::py_str` |
| `len(s)` conta code points | `str::len` conta bytes | `chars().count()` |

**Divergências conhecidas** (fora do fixture de propósito, sem efeito no que é
publicado): `website` float/lista/objeto no manifesto de sites vira texto em
JSON e não no `repr` do Python (D-023).

**Versão do Python:** o fixture é da versão do CI (3.12). O código atual do
ramo 3.13 do CPython muda duas bordas do `fromisoformat` (`.` sem dígitos;
fuso só com microssegundos); as duas estão no fixture, então a troca de versão
reprova o teste Python em vez de mudar a referência calada.

### Armadilhas de paridade — monitor de sites

O relatório do `check_websites.py` é o texto que o `urllib` produz, não uma
URL normalizada. Portado de `Lib/urllib` e `Lib/http/client.py` do 3.12.14
(`crates/site-monitor/src/{pyurl,pyrequest,redirect}.rs`).

| Python (`urllib`) | Cliente HTTP comum | `site-monitor` |
|:---|:---|:---|
| `final_url` sem redirect é o texto dado: `https://example.org` | normaliza para `https://example.org/` | `Request.full_url` portado |
| `Location` passa por `urlparse` → `urlunparse` → `quote(latin-1, safe=pontuação)` → `urljoin` | resolve pelo WHATWG | a mesma sequência: `/ção` em bytes vira `/%E7%E3o`, `..` além da raiz some, `//x` é host |
| laço: 4 visitas à mesma URL ou 10 destinos distintos → `HTTPError` do redirect | limite próprio (10 no `reqwest`) | `RedirectGuard`, com a mesma ordem de checagem |
| `HTTPError` com código em `LIVE_STATUSES` conta como no ar: laço, 302 sem `Location`, `mailto:` (A21) | redirect falho é erro | reproduzido (`py-check@1`) |
| falha de rede num salto: `final_url` é a URL **original**, HTTP 0 | a URL do salto | a original |
| `http.client.InvalidURL` e `BadStatusLine` derrubam o script (A22) | — | conta como fora do ar e avisa no stderr |
| caminho não-ASCII: `UnicodeEncodeError` antes da rede; host fora do latin-1 também | codifica e conecta | recusa antes da rede, como o Python |
| cabeçalho `URI` vale quando falta `Location` | ignora | aceito |
| corpo do redirect é lido antes de seguir | descartado | lido (o erro de leitura conta) |

**Divergências conhecidas** (fora do fixture; efeito só em casos que o
catálogo não tem):

- o `reqwest` sempre envia `Accept: */*`;
- um `#` interno no caminho (`/p#a#b`) vai na linha de pedido do Python, não
  na do Rust (o `final_url` é o mesmo);
- espaço no fim do `Location` é aparado pelo `httparse`: o Python segue para
  `/ok%20%20%20`, o Rust para `/ok` (conferido);
- redirect para `ftp://`: o Python tenta FTP, o Rust falha na hora (os dois
  saem `unreachable` 0, em tempos diferentes);
- porta com dígitos não-ASCII (`٨٠`) é número para o `int()` do Python;
- host latin-1 não-ASCII: IDNA 2003 no Python, UTS 46 no Rust;
- `--timeout` ≤ 0 e `--retries` negativo: o Python aceita e toda checagem
  falha; o Rust recusa como uso incorreto (código 2);
- catálogo malformado: o Python quebra com traceback (código 1); o Rust sai
  com 2 e o nome da exceção.

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

  check sites [--json] [--fail-on-down] [--timeout S] [--retries N] [--max-workers N]
              [--root DIR] [--no-db] [--trigger T]
                                ✅ Fase 2 — relatório do check_websites.py; grava histórico
                                (sem DATABASE_URL e sem --no-db, recusa)  (Website Monitor)

  import manifests              docs/README_*.json → banco (manifesto vence)
  import legacy                 carga única do estado atual (MIGRATIONS.md §6)

  catalog build [--out docs/project-catalog.json]
  render readme [--write | --check]    só entre marcadores; --check falha se mudaria
  render assets [--write | --check]    SVGs
  render all    [--write | --check]

  validate [readme|catalog|links|exclusions|badges]
  export legacy-state           ECOSYSTEM-COMMIT-STATE.json schema 4 (compatibilidade)

  db status [--json]            ✅ Fase 1 — só lê; sai com 1 se há migration alterada/pela metade/desconhecida
  db migrate                    ✅ Fase 1 — tudo ou nada
  db revert [--to V | --all] [--allow-data-loss]
                                ✅ Fase 1 — tudo ou nada; sem a flag, a trava das `down` recusa
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
  `--check` com diferença, banco inconsistente, trava de perda de dados), `2`
  erro de execução (inclui uso incorreto, que o clap sinaliza com 2). Nunca `0`
  quando uma etapa foi pulada por falta de credencial (lição de A1).
- A conexão vem de `DATABASE_URL`; `--database-url` existe, mas deixa a senha
  na lista de processos. Nenhuma mensagem — nem a ajuda — repete a URL.

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
| Python do CI muda de versão e a referência muda junto | fixture gerado na versão do CI, com as bordas conhecidas entre versões (D-023) |
| Clippy novo num stable novo | toolchain fixado em `rust-toolchain.toml`; subir é PR próprio (D-025) |
| Divergência sutil de formatação (floats, ordenação, Unicode) | fixture golden byte a byte; `format_bytes`/percentuais reproduzidos com os mesmos arredondamentos |
| Diferença de biblioteca HTTP (redirects, TLS) | os mesmos casos do servidor local rodam contra os dois |
| Dono do perfil precisa de Rust para contribuir | manifestos continuam em JSON; nada editorial exige Rust |
| Banco fora do ar | `--no-db` e D-009 |
| Contadores do monitor | importados com proveniência; o Python continua escrevendo o JSON até a paridade |
