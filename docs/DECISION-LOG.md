# Log de decisões

Decisões de arquitetura do núcleo do ecossistema. Cada entrada registra o
contexto, o que foi decidido, o que foi descartado e quando revisitar.
Decisão **aceita** está implementada ou é regra em vigor; **proposta** aguarda
o dono do perfil. Decisão aceita não se re-litiga sem fato novo — abre-se uma
entrada nova que a substitui.

Origem de todas as entradas até D-020: [auditoria de 2026-09-25](audits/2026-09-25-ecosystem-core-audit.md).
D-021 e D-022 nasceram na execução da Fase 0 e revisam duas recomendações dela.
D-023 a D-025 nasceram na Fase 1 (fundação do núcleo em Rust).

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
| [D-023](#d-023) | Paridade de domínio: fixture gerado pelo Python do CI, conferido pelo Rust | aceita |
| [D-024](#d-024) | Migrations pelo binário: tudo ou nada, tabela do sqlx-cli, perda de dado por conexão | aceita |
| [D-025](#d-025) | Toolchain Rust fixado (1.94.1) e versão mínima real (1.94) | aceita |

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

## D-023

**Paridade de domínio: fixture gerado pelo Python do CI, conferido pelo Rust.** · aceita · 2026-09-25

- **Contexto:** D-006 manda o Rust reproduzir o Python antes de corrigir. O
  golden da Fase 0 (0.11) compara o README inteiro, mas só serve quando o
  render for portado (Fase 4); as regras de domínio chegam antes.
- **Decisão:** `tests/test_parity_domain.py` roda as funções Python reais sobre
  640 casos em 13 grupos (classify, status, prioridades, apresentação,
  catálogo, descoberta de site, leitura de datas…) e grava
  `tests/fixtures/parity/domain.json`; `crates/ecosystem-domain/tests/parity.rs`
  exige a mesma saída, caso a caso. Um grupo que o Python passe a gerar sem
  conferência no Rust reprova o teste. O fixture é gerado com o **Python do CI
  (3.12)** e o teste Python roda no V2 Validation: um lado não muda sem o outro
  perceber.
- **O que isso obrigou a reproduzir:** `round()` com empate para o par;
  `str.isspace()` com U+001C–U+001F (conferido nos 1,1 milhão de code points);
  `$` do `re` antes de `\n` final; `timedelta.days` para baixo; o defeito A6; e
  `datetime.fromisoformat`, **portado do C do CPython 3.12** em vez de
  aproximado — `10.5` é 10:00:00,5, `+00:60` é uma hora, qualquer caractere
  (até multibyte) separa data e hora.
- **Descartado:** expectativas digitadas à mão — o primeiro teste unitário do
  port esperava "Software & Ferramentas" para `plain`, e o Python responde
  "IA & Automação" (p-l-**ai**-n, o próprio A6); e o RFC 3339 do `chrono`, que
  recusaria formas que o Python aceita.
- **Verificação:** 43 mutações no port, 38 mortas pelo fixture; as 5 restantes
  são equivalentes (analisadas uma a uma) e uma delas é morta por teste
  unitário.
- **Divergência registrada:** `website` não-texto (float, lista, objeto) no
  manifesto de sites vira texto em JSON, não no `repr` do Python; não muda a
  descoberta, só o texto que `check_websites.py` relataria — a portar com ele
  (Fase 2).
- **Revisitar quando:** o CI trocar de Python (o fixture já tem as duas bordas
  que o ramo 3.13 do CPython muda) e quando `classifier@2` corrigir A6 — versão
  nova, grupo novo, o `py-classify@1` continua conferido.

## D-024

**Migrations pelo binário: tudo ou nada, tabela do sqlx-cli, perda de dado por conexão.** · aceita · 2026-09-25

- **Contexto:** critério de saída da Fase 1 ("migrations aplicadas pelo
  binário"), sobre D-004 (formato sqlx) e D-008 (guarda das `down`).
- **Decisão:** o crate `store` embute `db/migrations` (`sqlx::migrate!`) e:
  - usa a **mesma tabela de controle do `sqlx-cli`** (`_sqlx_migrations`,
    mesmos checksums) — os dois são intercambiáveis;
  - roda `migrate` e `revert` numa **transação externa** (o sqlx abre um
    savepoint por migration): ou o comando inteiro entra, ou nada muda. Um
    `revert --all` que esbarra na guarda da 0006 não reverte nem as views da
    0007;
  - liga `ecosystem.allow_data_loss=on` **só** com `--allow-data-loss`, como
    opção de inicialização da conexão dedicada daquele comando — nunca numa
    conexão compartilhada;
  - `status` só lê (nem cria a tabela de controle);
  - migration alterada, pela metade ou desconhecida (banco à frente do
    binário) é recusada **antes** de qualquer mudança — inclusive no `revert`,
    que o sqlx não confere.
- **Descartado:** `#[sqlx::test]` — aplica as migrations sozinho, e os testes
  precisam de banco vazio, parcialmente migrado e adulterado; cada teste cria e
  apaga o próprio banco (`STORE_TESTS_REQUIRED=1` no CI impede que pulem
  calados). Também adiado: `query!` com `SQLX_OFFLINE` — nesta fase a única
  tabela lida é a de controle do sqlx; entra com os repositórios de dados.
- **Verificação:** 6 testes de integração, 5 de ponta a ponta da CLI, e
  `db/tests/profile_core_e2e.sh`: o schema do binário é idêntico por
  `pg_dump --schema-only` ao das migrations aplicadas pelo psql, e os testes
  SQL passam sobre ele. 10 de 10 mutações mortas no `store`.
- **Consequência aceita:** uma migration `-- no-transaction` quebraria o tudo
  ou nada; o crate cai para uma transação por migration e um teste unitário
  reprova, forçando a decisão a ser consciente.

## D-025

**Toolchain Rust fixado (1.94.1) e versão mínima real (1.94).** · aceita · 2026-09-25

- **Contexto:** o workspace nasceu declarando `rust-version = 1.85`, mínimo da
  edição 2024 — mas o sqlx 0.9 exige 1.94. O número declarado era falso.
- **Decisão:** `rust-version = 1.94`; `rust-toolchain.toml` fixa o 1.94.1 com
  `rustfmt` e `clippy`, e o CI instala exatamente essa versão (sem depender da
  instalação automática do rustup, que mudou entre versões).
- **Motivo:** uma lint nova do clippy num stable novo não deixa o CI vermelho
  sem mudança no código; subir o compilador é um PR que muda um arquivo.
- **Revisitar quando:** sair um stable novo (a cada ~6 semanas), num PR
  próprio que rode o `Rust Core` inteiro.
