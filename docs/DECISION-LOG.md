# Log de decisões

Decisões de arquitetura do núcleo do ecossistema. Cada entrada registra o
contexto, o que foi decidido, o que foi descartado e quando revisitar.
Decisão **aceita** está implementada ou é regra em vigor; **proposta** aguarda
o dono do perfil. Decisão aceita não se re-litiga sem fato novo — abre-se uma
entrada nova que a substitui.

Origem de todas as entradas até D-020: [auditoria de 2026-09-25](audits/2026-09-25-ecosystem-core-audit.md).
D-021 e D-022 nasceram na execução da Fase 0 e revisam duas recomendações dela.
D-023 a D-025 nasceram na Fase 1 (fundação do núcleo em Rust); D-026 a D-028, na Fase 2 (monitor de sites);
D-029 a D-033, na Fase 3 (coleta).

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
| [D-014](#d-014) | Primeira fatia em Rust: o monitor de sites | aceita (Fase 2) |
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
| [D-026](#d-026) | Monitor de sites: o `urllib` do CPython 3.12.14 é a especificação, portado; o transporte é o `reqwest` | aceita |
| [D-027](#d-027) | `check sites` grava histórico ou recusa; o monitor não cria projetos nem sites | aceita |
| [D-028](#d-028) | Fixture de paridade preso à versão exata do Python do CI | aceita |
| [D-029](#d-029) | Um cliente do GitHub; a política de nova tentativa de cada coletor é parâmetro | aceita |
| [D-030](#d-030) | No modo B, o JSON versionado continua sendo o estado do monitor | aceita |
| [D-031](#d-031) | Inventário no banco: sumido só com inventário completo; erro não apaga dado | aceita |
| [D-032](#d-032) | Manifestos espelhados no banco; carga legada idempotente, só preenche o vazio | aceita |
| [D-033](#d-033) | Métrica coletada só entra quando o valor muda | aceita |

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

**Primeira fatia em Rust: o monitor de sites.** · aceita · 2026-09-25 (implementada em 2026-09-26: D-026 a D-028)

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

## D-026

**Monitor de sites: o `urllib` do CPython 3.12.14 é a especificação, portado; o transporte é o `reqwest`.** · aceita · 2026-09-26

- **Contexto:** o critério de saída da Fase 2 é o relatório do
  `check_websites.py --json` idêntico. Ele depende de detalhes do `urllib` que
  um cliente HTTP moderno faz diferente: o texto do `final_url` (sem
  normalizar: `https://example.org` sem barra, `/ção` de um `Location` vira
  `/%E7%E3o`), a resolução de `Location` relativo, o limite de laço e o que
  conta como "no ar" quando o redirect falha (A21).
- **Decisão:** o crate `site-monitor` porta do código da biblioteca padrão o
  que decide o relatório — `urllib.parse`, `Request`, as validações do
  `http.client` antes da rede e o `http_error_302` com a contagem de laço — e
  confere contra um fixture gerado **chamando** a biblioteca padrão
  (`tests/test_parity_site_monitor.py`, 111 casos). O transporte é o
  `reqwest` 0.13: HTTP/1.1, sem redirect automático, rustls com `ring` (o
  mesmo provedor do sqlx), certificados do sistema, `Host` igual ao do Python,
  timeout por conexão e por leitura (o `timeout` do socket).
- **Descartado:** escrever um cliente HTTP próprio para reproduzir a linha de
  pedido byte a byte (proxy, TLS e HTTP/1.1 corretos custam mais do que as
  diferenças valem); e usar o redirect automático do `reqwest` (normaliza a
  URL e não reproduz A21).
- **Divergências que sobram** (fora do fixture, documentadas em
  PYTHON-TO-RUST.md): o `reqwest` sempre envia `Accept: */*`; um `#` interno
  no caminho não vai na linha de pedido; espaço no fim do `Location` é aparado
  pelo `httparse`; redirect para `ftp://` é falha imediata em vez de uma
  tentativa FTP; dígitos não-ASCII de porta; IDNA 2008 × 2003 em host
  latin-1. E o que derruba o Python (A22) aqui é "fora do ar".
- **Verificação:** e2e contra servidor local com 45 cenários (texto, JSON e
  código de saída idênticos); mutações 17 de 19 mortas — uma sobrevivente
  virou cenário novo (hub→spoke), a outra é o controle; e o modo sombra sobre
  os 16 sites reais, idêntico na primeira execução.

## D-027

**`check sites` grava histórico ou recusa; o monitor não cria projetos nem sites.** · aceita · 2026-09-26

- **Contexto:** o histórico vai para `website_checks`, que referencia
  `websites`, que referencia `projects`. Quem cria projetos é a coleta
  (`sync github`, Fase 3); quem registra sites é ela e a importação de
  manifestos (DATA-MODEL §5, MIGRATIONS §6).
- **Decisão:** `check sites` grava uma linha por checagem **para cada site
  ativo com aquela URL**, numa execução `websites` de `sync_runs`, tudo ou
  nada (falha vira execução `failed` com o motivo). URL sem site registrado é
  relatada, não inventada. E, pela lição do A1, sem `DATABASE_URL` e sem
  `--no-db` o comando recusa (código 2) em vez de pular o histórico calado.
- **Consequência aceita:** até a Fase 3 povoar `projects` e `websites` (e o
  dono provisionar o banco de produção), o histórico em produção fica vazio;
  a capacidade está provada nos testes. O workflow sombra grava só quando o
  secret `DATABASE_URL` existir, e não roda migrations.

## D-028

**Fixture de paridade preso à versão exata do Python do CI.** · aceita · 2026-09-26

- **Fato:** o `urllib` muda **dentro** da série 3.12: o `urlunsplit` do
  3.12.3 (o do Ubuntu 24.04) difere do 3.12.14 (o que o `setup-python`
  instala). Um fixture gerado com o Python do sistema poderia divergir do CI
  sem ninguém ter mudado nada.
- **Decisão:** `site_monitor.json` é gerado com o CPython 3.12.14 (o do CI,
  lido no log do job) e registra a versão; o teste compara só em 3.12.x e
  pula em outra versão menor. O Rust Core usa o mesmo Python no e2e. Quando o
  CI trocar de patch e a biblioteca padrão mudar um caso, o teste Python
  reprova: é o sinal para regenerar e portar a mudança no mesmo PR.
- **Como regenerar:** `uv python install 3.12.14` e
  `UPDATE_PARITY=1 python3.12 -m unittest tests.test_parity_site_monitor`
  com esse interpretador.

## D-029

**Um cliente do GitHub; a política de nova tentativa de cada coletor é parâmetro.** · aceita · 2026-09-26

- **Contexto:** três scripts Python falam com a API, cada um com o seu
  `api()`: o `update_profile.py` tenta uma vez; o `ecosystem_watch.py` tenta
  até 4 vezes (429, 5xx, 403 com `X-RateLimit-Remaining: 0`, falha de
  conexão, timeout), esperando `Retry-After` só quando é número; a timeline
  faz um POST GraphQL sem nova tentativa. E o monitor grava o texto do erro
  no estado (A24).
- **Decisão:** um crate `github-client` (reqwest, rustls) com `Retry::Never`
  e `Retry::Watch` como argumento de cada chamada, a paginação comum aos
  coletores (página de 100 pede a próxima) e erros cujo `Display` é o
  `str(exc)` do `urllib` (`HTTP Error 409: Conflict`, `<urlopen error [Errno
  111] Connection refused>`, `timed out`). `GITHUB_API_URL` e
  `GITHUB_GRAPHQL_URL` (que o Actions já define com os valores públicos)
  apontam os dois lados para um GitHub simulado nos testes.
- **Descartado:** uma política única "melhor" para todos — mudaria quando o
  monitor registra erro e, com isso, os contadores (D-006).
- **Divergência que sobra:** texto de erro que não é HTTP (DNS, TLS, JSON
  inválido) é aproximado; a frase de status é a canônica do código.

## D-030

**No modo B, o JSON versionado continua sendo o estado do monitor.** · aceita · 2026-09-26

- **Contexto:** o contador do monitor não pode ser recalculado a partir do
  GitHub (parte de 1538 e soma só snapshots publicados). Enquanto o Python
  publica, o `ECOSYSTEM-COMMIT-STATE.json` é a verdade.
- **Decisão:** `profile-core sync commits` lê o mesmo arquivo como estado
  anterior e produz o mesmo estado, relatório e saída — conferidos pelo
  fixture (`monitor.json`, 12 cenários do `main()` real, inclusive a sequência
  de chamadas à API) e pelo e2e de 5 varreduras. Com banco, grava a
  varredura (`sync_runs`), as transições em relação a esse estado
  (`commit_observations`, D-020) e os contadores (`metric_samples`) quando o
  snapshot é publicado.
- **Consequência:** exportar o estado a partir do banco (`export
  legacy-state`) só é necessário na virada (modo C) e fica para ela.

## D-031

**Inventário no banco: sumido só com inventário completo; erro não apaga dado.** · aceita · 2026-09-26

- **Decisão:** `sync github` grava donos, repositórios (identidade
  `github_id`: renomear atualiza a linha), linguagens e um projeto por
  repositório (rótulo da heurística `py-classify@1`; o editorial nunca é
  sobrescrito; slug do nome, com o dono em caso de colisão). Marca
  `gone_at` em quem sumiu **só** quando o inventário é completo
  (`PROFILE_GITHUB_TOKEN` ou arquivo): sem token, recusa gravar. Repositório
  excluído nem é consultado (D-019). Uma consulta de linguagens que falhou
  mantém o mapa anterior (A25). A `homepage` de repositório público vira
  site `github_homepage`; o primário segue a ordem da descoberta do Python
  (homepage, manifesto, editorial).
- **Por quê:** "nunca destruir dados existentes": um erro passageiro ou um
  inventário parcial não pode apagar o que o banco sabe.

## D-032

**Manifestos espelhados no banco; carga legada idempotente, só preenche o vazio.** · aceita · 2026-09-26

- **`import manifests`:** deixa `repository_exclusions`, `featured_entries`,
  `stack_tools` e os sites `manifest` iguais aos `docs/README_*.json`, numa
  transação. O que sai do manifesto sai do banco — exceto site, que é
  aposentado (`retired_at`), porque as checagens dele são histórico. Uma
  referência que não casa (projeto que o `sync github` não trouxe, categoria
  que não existe) recusa a importação inteira.
- **`import legacy`:** MIGRATIONS.md §6, passos 3b–7. Cada passo grava só o
  que ainda não existe (resumo e sobreposição vazios, site sem checagem,
  repositório sem observação, métrica sem amostra importada). Rodar de novo
  não duplica nada e não sobrescreve o que a coleta já gravou.
- **Constantes do código:** `FEATURED_SUMMARIES` e os nomes fixos do
  `status_for()` foram copiados para o `ecosystem-domain`; um teste lê o
  `update_profile.py` e reprova se as cópias divergirem.

## D-033

**Métrica coletada só entra quando o valor muda.** · aceita · 2026-09-26

- **Contexto:** a timeline consulta 13 janelas × 7 métricas por dia; as
  janelas fechadas quase nunca mudam. Gravar todas, todo dia, é duplicar o
  GitHub sem informação nova ("não duplicar indiscriminadamente").
- **Decisão:** `sync contributions` grava em `metric_samples` só a amostra
  cujo valor difere da mais recente da mesma série (métrica × janela). As
  janelas da ponta mudam de data todo dia e, por isso, entram diariamente.

