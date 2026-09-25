# Log de decisões

Decisões de arquitetura do núcleo do ecossistema. Cada entrada registra o
contexto, o que foi decidido, o que foi descartado e quando revisitar.
Decisão **aceita** está implementada ou é regra em vigor; **proposta** aguarda
o dono do perfil. Decisão aceita não se re-litiga sem fato novo — abre-se uma
entrada nova que a substitui.

Origem de todas as entradas até D-020: [auditoria de 2026-09-25](audits/2026-09-25-ecosystem-core-audit.md).
D-021 e D-022 nasceram na execução da Fase 0 e revisam duas recomendações dela.

| ID | Decisão | Status |
|:---|:---|:---|
| [D-001](#d-001) | Primeiro auditoria, desenho e schema; nenhum comportamento de produção muda | aceita |
| [D-002](#d-002) | Fontes de verdade: GitHub, banco, README | aceita |
| [D-003](#d-003) | PostgreSQL ≥ 15, fora do Git, schema `ecosystem` | aceita |
| [D-004](#d-004) | Migrations reversíveis no formato do sqlx, em `db/migrations` | aceita |
| [D-005](#d-005) | Identidade de repositório é `github_id`; nada é apagado | aceita |
| [D-006](#d-006) | Paridade antes de correção | aceita |
| [D-007](#d-007) | Histórico append-only; "estado atual" é view | aceita |
| [D-008](#d-008) | Reverter com perda de dados exige opt-in | aceita |
| [D-009](#d-009) | Banco opcional até ser promovido | proposta |
| [D-010](#d-010) | A fronteira de privacidade é uma view do banco | aceita |
| [D-011](#d-011) | Taxonomia por migration; editorial por importação | aceita |
| [D-012](#d-012) | Rust pelo que traz de correção, não por desempenho | proposta |
| [D-013](#d-013) | Workspace Rust neste repositório, crates por responsabilidade | proposta |
| [D-014](#d-014) | Primeira fatia em Rust: o monitor de sites | proposta |
| [D-015](#d-015) | Validadores Python ficam como oráculo durante a transição | proposta |
| [D-016](#d-016) | `deployments` só quando houver quem escreva nela | aceita |
| [D-017](#d-017) | API começa como JSON estático versionado | proposta |
| [D-018](#d-018) | Um pipeline, um escritor no `main` | proposta |
| [D-019](#d-019) | Repositório excluído não é armazenado | aceita |
| [D-020](#d-020) | O monitor grava transições, não varreduras | aceita |
| [D-021](#d-021) | `lang-stats.svg` pertence ao `lang_stats.py` | aceita |
| [D-022](#d-022) | Corrida de push: rebase com nova tentativa, sem grupo comum | aceita |

---

## D-001

**Primeiro auditoria, desenho e schema; nenhum comportamento de produção muda.** · aceita · 2026-09-25

- **Contexto:** o pedido original proíbe migração automática antes de uma
  auditoria completa. A auditoria encontrou defeitos em produção (A1–A5).
- **Decisão:** esta fase entrega documentação, schema SQL, seed sintético,
  testes de migration e um workflow de CI **aditivo** (`db-validation.yml`).
  Nenhum script Python, workflow existente, README ou asset foi alterado
  nesse commit. As correções da Fase 0 vieram depois, **um commit por item**
  no mesmo PR (o ambiente só publica num branch), com o estado registrado na
  tabela da auditoria.
- **Descartado:** corrigir os defeitos no mesmo PR — misturaria "o que
  mudou no comportamento" com "o que é só desenho" e dificultaria a revisão.

## D-002

**Fontes de verdade.** · aceita · 2026-09-25

| Dado | Fonte de verdade | O banco guarda |
|:---|:---|:---|
| Nome, descrição, visibilidade, linguagem, homepage, branch, datas, commits | **GitHub** | o recorte usado pelo catálogo + quando foi visto/sincronizado |
| Classificação, curadoria, resumo, exclusões, sites declarados à mão, ferramentas do arsenal | **editorial** — escrito em Git (`docs/README_*.json`) durante a transição | cópia efetiva, com origem |
| Verificações de site, observações de commit, métricas, execuções | **banco** (histórico que não existe em lugar nenhum) | tudo, append-only |
| README, SVGs, `project-catalog.json` | **derivados** | nada — são renderizados a partir do banco ou do snapshot |

- **Por que o editorial continua em Git por enquanto:** revisão por PR,
  histórico e diff de graça, sem painel administrativo para construir. O
  comando `profile-core import manifests` copia para o banco a cada execução;
  o manifesto vence.
- **Revisitar quando:** existir uma interface de edição (API administrativa
  ou painel). Aí o banco vira o ponto de autoria e os manifestos viram
  exportação.

## D-003

**PostgreSQL ≥ 15, fora do Git, schema `ecosystem`.** · aceita · 2026-09-25

- **Decisão:** PostgreSQL é o banco principal. O Git guarda migrations,
  seeds sintéticos, testes e documentação — nunca o banco nem uma string de
  conexão. A conexão chega por `DATABASE_URL` (secret do Actions / variável
  local). Tudo vive no schema `ecosystem`, para o banco poder ser
  compartilhado (Supabase, Neon) sem colidir com `public`.
- **Por que ≥ 15:** `UNIQUE NULLS NOT DISTINCT` (nome ativo único com soft
  delete, D-005). Testado em 16.
- **Descartado:** SQLite versionado no Git (histórico cresce sem limite,
  merge impossível, contradiz o pedido); ENUMs do PostgreSQL (difíceis de
  evoluir) — `text` + `CHECK`.
- **Pendente do dono:** onde hospedar. Ver [DATABASE-ARCHITECTURE.md §3](database/DATABASE-ARCHITECTURE.md).

## D-004

**Migrations reversíveis no formato do sqlx, em `db/migrations`.** · aceita · 2026-09-25

- **Decisão:** `NNNN_descricao.up.sql` + `NNNN_descricao.down.sql`, formato
  que o `sqlx-cli` reconhece como reversível e que o `sqlx::migrate!()` embute
  no binário Rust. Diretório `db/` (e não `migrations/` na raiz) para agrupar
  migrations, seeds e testes.
- **Verificação:** `db/tests/run.sh` aplica cada migration, reverte, compara o
  schema byte a byte e reaplica; depois roda `sqlx migrate run` e `revert` até
  zero com o sqlx-cli 0.9.0. O mesmo roda no CI (`db-validation.yml`).
- **Regra:** migration aplicada não se edita (o sqlx confere checksum);
  muda-se com uma migration nova.
- **Descartado:** refinery (sem down), diesel (ORM que não será usado), Flyway
  (JVM no CI de um repositório sem JVM).

## D-005

**Identidade de repositório é `github_id`; nada é apagado.** · aceita · 2026-09-25

- **Contexto:** o sistema atual identifica tudo pelo nome. Renomear um
  repositório criaria um "projeto novo" e perderia o histórico.
- **Decisão:** `repositories.github_id` é a identidade; `full_name` é atributo.
  Repositório que some do inventário recebe `gone_at` — a linha e o histórico
  ficam. O nome ativo é único (sem diferenciar maiúsculas) por uma restrição
  `DEFERRABLE`, para que uma troca de nomes (A→B e um novo A) caiba numa
  transação.

## D-006

**Paridade antes de correção.** · aceita · 2026-09-25

- **Decisão:** cada componente migrado para Rust reproduz a saída do Python
  **byte a byte** sobre as mesmas entradas fixas — inclusive os defeitos
  conhecidos, como a classificação por substring (A6). Correção vem depois,
  como versão nomeada (`py-classify@1` → `classifier@2`), com o diff da saída
  pública revisado antes do merge.
- **Por quê:** se a migração e a correção chegam juntas, uma diferença na
  saída não diz se é regressão ou melhoria.

## D-007

**Histórico append-only; "estado atual" é view.** · aceita · 2026-09-25

- **Decisão:** `website_checks`, `commit_observations` e `metric_samples`
  recusam `UPDATE`, `DELETE` e `TRUNCATE` por trigger. O estado atual
  (`website_status_current`, `repository_head_current`, `metric_latest`) é
  derivado da linha mais recente.
- **Por quê:** o pedido é explícito — "evite sobrescrever o estado anterior,
  queremos histórico". Uma coluna `current_status` atualizada em paralelo
  poderia divergir do histórico; uma view não.
- **Custo:** volume. ~20 sites × 24 checagens/dia ≈ 175 mil linhas/ano —
  trivial para o PostgreSQL. Retenção, se um dia for preciso, é um job
  explícito com opt-in (D-008).

## D-008

**Reverter com perda de dados exige opt-in.** · aceita · 2026-09-25

- **Decisão:** todo `down` que apaga tabela chama
  `ecosystem.assert_data_loss_allowed()`, que recusa se a tabela tiver linhas,
  a menos que a sessão tenha `ecosystem.allow_data_loss=on`.
- **Verificação:** o harness confere que cada down com tabela recusa sem o
  opt-in e passa com ele; um teste de mutação que remove a guarda falha.

## D-009

**Banco opcional até ser promovido.** · proposta · 2026-09-25

- **Decisão proposta:** o núcleo Rust monta um `CatalogSnapshot` em memória a
  partir do GitHub **ou** do banco. Até a Fase 4, o README continua sendo
  gerado sem depender do banco estar acessível; o banco recebe as escritas em
  paralelo (modo sombra) e é comparado.
- **Por quê:** o perfil é a vitrine pública; uma indisponibilidade do banco
  não pode congelar o README — já basta A1.

## D-010

**A fronteira de privacidade é uma view do banco.** · aceita · 2026-09-25

- **Decisão:** `ecosystem.public_projects`, `public_repositories` e
  `public_language_totals` aplicam as regras de privado, excluído e
  site-só-se-verificado. Um papel de leitura pública recebe `SELECT` **só
  nessas views**. A divulgação de privados que o dono autorizou fica isolada em
  `private_repository_listing` (nome, categoria, descrição; sem site).
- **Por quê:** hoje cada coletor reimplementa o filtro, e só não vaza porque
  roda sem acesso a privados (A18).
- **Consequência:** as views são "security definer" (padrão do PostgreSQL).
  O schema `ecosystem` **não** deve ser exposto pela API automática do
  Supabase (PostgREST); a API é o serviço próprio (D-017).
- **Armadilha já paga:** a primeira versão filtrava exclusões chamando uma
  função SQL dentro da view. Função roda com o privilégio de **quem consulta**,
  então o leitor público recebia `permission denied for table
  repository_exclusions`. O filtro passou a ser SQL puro numa view base
  (`listed_repositories`), e `db/tests/sql/040_privileges.sql` cria um papel
  com `SELECT` só nas views e confere o que ele lê e o que ele não lê — o teste
  reprova o desenho antigo. Regra: views da fronteira pública não chamam
  função que leia tabela.

## D-011

**Taxonomia por migration; editorial por importação.** · aceita · 2026-09-25

- **Decisão:** categorias canônicas, rótulos, domínios da vitrine, categorias
  do arsenal, definições de métrica e o rótulo/cor das linguagens conhecidas
  são **dado de referência** — o código depende deles, então entram por
  migration. Curadoria, ferramentas, exclusões e sites manuais são
  **editoriais** e entram por `import manifests`.
- **Efeito:** "todo rótulo cai num domínio" deixa de ser só um teste Python e
  vira `NOT NULL REFERENCES`.

## D-012

**Rust pelo que traz de correção, não por desempenho.** · proposta · 2026-09-25

- **Medição:** o pipeline inteiro é limitado por I/O (≈ 200 chamadas à API do
  GitHub e ≈ 20 requisições HTTP) e termina em segundos. Não há gargalo de
  CPU para o Rust resolver. Mesma lição registrada no Projeto Baluarte: meça
  antes de culpar a linguagem.
- **O que justifica o Rust aqui:**
  1. **um modelo de domínio tipado** compartilhado por coletor, renderizador,
     validador e API — hoje são `dict[str, Any]` e quatro definições de
     inventário (A3);
  2. **SQL checado em tempo de compilação** (`sqlx::query!`) contra o schema:
     drift entre código e banco vira erro de build;
  3. **um binário estático** para o CI e para um serviço de longa duração
     (monitor de sites contínuo, API), com memória baixa e sem runtime;
  4. **concorrência estruturada** (tokio) para rate limit, retries e
     verificações paralelas, hoje reimplementadas em cada script.
- **O que fica em Python:** o que não ganha nada com isso — ver
  [MIGRATION-MATRIX.md](migration/MIGRATION-MATRIX.md).

## D-013

**Workspace Rust neste repositório, crates por responsabilidade.** · proposta · 2026-09-25

- **Decisão proposta:** `Cargo.toml` de workspace na raiz, crates em `crates/`,
  divididos por responsabilidade (domínio, cliente GitHub, monitor de sites,
  catálogo, armazenamento, renderização, CLI) — **não** um crate por script
  Python. Ver [PYTHON-TO-RUST.md §3](migration/PYTHON-TO-RUST.md).
- **Alternativa:** repositório separado. Descartada por ora: o núcleo e a
  vitrine mudam juntos, e o `AGENTS.md` pede contrato explícito para
  dependência entre repositórios.

## D-014

**Primeira fatia em Rust: o monitor de sites.** · proposta · 2026-09-25

- **Por quê ele:** comportamento já especificado por 25 testes; entrada e
  saída pequenas e comparáveis (lista de URLs → relatório JSON); não escreve no
  README; e entrega a capacidade nova mais pedida — histórico de
  disponibilidade.
- **Por que não o README primeiro:** é o componente de maior raio de impacto e
  o único sem teste de ponta a ponta.

## D-015

**Validadores Python ficam como oráculo durante a transição.** · proposta · 2026-09-25

- **Decisão proposta:** quando o Rust passar a gerar o README, os validadores
  Python (`validate_dynamic_sections`, `validate_project_links`,
  `validate_exclusions`) continuam no CI checando a saída do Rust. Uma segunda
  implementação independente conferindo a primeira é justamente o que se quer
  numa migração. Saem só depois de equivalentes Rust validados.

## D-016

**`deployments` só quando houver quem escreva nela.** · aceita · 2026-09-25

- **Contexto:** o pedido lista `deployments`. Hoje não existe nenhum dado de
  deployment — só URLs verificadas por HTTP.
- **Decisão:** site (URL pública conferida por HTTP) e deployment (evento do
  provedor: Vercel, GitHub Pages) são coisas diferentes. A tabela de
  deployments está desenhada em [DATA-MODEL.md](database/DATA-MODEL.md), mas
  só vira migration junto com o coletor que a alimenta (API da Vercel ou
  GitHub Deployments). Tabela sem escritor é schema morto.

## D-017

**API começa como JSON estático versionado.** · proposta · 2026-09-25

- **Decisão proposta:** o `docs/project-catalog.json` já é, na prática, a API
  v0. Próximo passo: JSON Schema publicado e arquivos estáticos derivados das
  views públicas. Serviço HTTP só quando houver um consumidor real (site do
  Baluarte, painel, agente via MCP). Ver [API-ARCHITECTURE.md](api/API-ARCHITECTURE.md).

## D-018

**Um pipeline, um escritor no `main`.** · proposta · 2026-09-25

- **Contexto:** quatro workflows empurram para o `main` de forma independente
  (A8) e cada um refaz o inventário (A3).
- **Decisão proposta:** no curto prazo, push com rebase e nova tentativa
  (Fase 0, implementado — D-022; o grupo comum foi descartado lá). No fim da migração, um único job agendado roda
  `profile-core sync all && profile-core render all` e faz um commit.

## D-019

**Repositório excluído não é armazenado.** · aceita · 2026-09-25

- **Decisão:** `repository_exclusions` guarda só o nome casado. O coletor não
  insere nem enriquece o repositório excluído. As views públicas filtram por
  nome também, como defesa em profundidade (testado: um repositório excluído
  inserido por engano não aparece em nenhuma view).

## D-020

**O monitor grava transições, não varreduras.** · aceita · 2026-09-25

- **Decisão:** `commit_observations` recebe uma linha quando o estado de um
  repositório muda (SHA, erro, vazio). A varredura em si é uma linha em
  `sync_runs`. Mesma regra do `ecosystem_watch.py` atual: sem mudança
  semântica, nada gravado — e sem 67 linhas por hora de ruído.

## D-021

**`lang-stats.svg` pertence ao `lang_stats.py`.** · aceita · 2026-09-25

- **Contexto:** dois scripts gravavam o arquivo (A4). A auditoria recomendou
  manter o `update_profile.py` como escritor (item 0.4).
- **Fato novo:** o painel publicado é um redesign deliberado do
  `lang_stats.py` (`d2c0d58`, 20/08, "redesign ecosystem analysis as a
  dashboard"); o `update_profile.py` só passou a gravar o mesmo caminho em
  `a8ea902` (25/08), com outro desenho.
- **Decisão:** o `lang_stats.py` é o único escritor de `lang-stats.svg` e
  `profile-top-langs.svg`; o `update_profile.py` só referencia o arquivo no
  bloco `LANGUAGE-STATS`. `render_svg` foi removido (commit `29a2cc9`).
- **Consequência aceita:** a tabela (inventário do `update_profile`) e o
  painel (inventário do `lang_stats`) continuam com números diferentes até o
  inventário único do núcleo (A3, Fase 3). O `lang_stats` passou a aplicar
  as mesmas exclusões editoriais.

## D-022

**Corrida de push: rebase com nova tentativa, sem grupo comum.** · aceita · 2026-09-25

- **Contexto:** quatro workflows empurram para o `main` (A8). A auditoria
  sugeria grupo de concorrência comum + rebase (item 0.6).
- **Fato:** no GitHub, quando um run entra num grupo de concorrência com
  outro já rodando, ele fica pendente — e um run novo **cancela o pendente**.
  Com um grupo comum, o monitor horário cancelaria o refresh diário ou a
  análise semanal que estivessem na fila.
- **Decisão:** cada workflow mantém o próprio grupo; o push passa por
  `.github/scripts/push_with_rebase.sh` (tenta, e se recusado faz
  `pull --rebase` e tenta de novo, até 4 vezes). Como cada workflow commita
  arquivos que só ele escreve, o rebase não conflita; se conflitar, é defeito
  de posse de arquivo e o script falha alto.
- **Verificação:** corrida simulada com dois clones rasos (como o
  `actions/checkout`) — o segundo push é recusado, rebaseia e entra; um
  conflito real termina com código 1.
- **Revisitar quando:** D-018 (um pipeline, um escritor) for implementada; aí
  o script deixa de ser necessário.
