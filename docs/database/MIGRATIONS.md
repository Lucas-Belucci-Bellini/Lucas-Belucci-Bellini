# Migrations

Como o schema `ecosystem` evolui sem perder dado. Decisões de base: D-003,
D-004, D-007, D-008 e D-011 no [log de decisões](../DECISION-LOG.md).

## 1. Estrutura

```text
db/
├── migrations/                       ← versionadas, reversíveis, formato sqlx
│   ├── 0001_foundation.{up,down}.sql         schema, utilitários, sync_runs
│   ├── 0002_repositories.{up,down}.sql       owners, languages, repositories, exclusões
│   ├── 0003_projects.{up,down}.sql           taxonomia, projects, curadoria
│   ├── 0004_websites.{up,down}.sql           websites, website_checks, views de estado
│   ├── 0005_activity_metrics.{up,down}.sql   commit_observations, métricas
│   ├── 0006_editorial_stack.{up,down}.sql    arsenal
│   └── 0007_public_views.{up,down}.sql       projeção pública
├── seeds/dev/0001_demo_ecosystem.sql ← dados sintéticos (example.org)
└── tests/
    ├── run.sh                        ← harness: round-trip, seed, testes, guardas, sqlx
    └── sql/                          ← 000 helpers · 010 restrições · 020 views
                                        030 histórico · 040 privilégios
```

## 2. Convenções

| Regra | Por quê |
|:---|:---|
| Nome `NNNN_descricao.up.sql` + `NNNN_descricao.down.sql` | formato reversível do `sqlx-cli`; `NNNN` vira a versão (`0001` → 1) |
| Toda `up` tem `down` | o harness recusa migration sem reversão |
| Migration aplicada **não se edita** | o sqlx confere checksum; corrigir = migration nova |
| Uma responsabilidade por migration | revisão e reversão por assunto |
| Nada que não rode em transação (`CREATE INDEX CONCURRENTLY`, `ALTER TYPE … ADD VALUE` antigo) | sqlx e o harness aplicam cada arquivo numa transação; quando for inevitável, arquivo próprio com `-- no-transaction` |
| `down` de migration com tabela chama `ecosystem.assert_data_loss_allowed()` antes de apagar | D-008; o harness reprova o `down` que apaga dado sem opt-in |
| Dado de referência (taxonomia, definições de métrica, linguagens conhecidas) entra na `up` | o código depende dele (D-011) |
| Dado editorial e dado real **nunca** entram em migration | chegam por `import manifests` e pela coleta |
| Tabela nova precisa de linha no seed de desenvolvimento | é o que faz o teste de guarda do `down` ter o que proteger |
| Sem extensões | roda em qualquer PostgreSQL 15+ gerenciado |

## 3. Rodando localmente

```bash
# tudo: cluster temporário, round-trip, seed, testes SQL, guardas, sqlx
db/tests/run.sh

# contra um servidor existente (cria e apaga um banco próprio; precisa de CREATEDB)
PGHOST=localhost PGUSER=postgres PGPASSWORD=... db/tests/run.sh
```

O harness nunca toca num banco existente: cria `ecosystem_test_<pid>_<n>` e o
apaga ao sair. Sem `PGHOST`, sobe um cluster em diretório temporário com
`initdb`/`pg_ctl` (procura em `PG_BIN_DIR`, `pg_config --bindir` e
`/usr/lib/postgresql/*/bin`).

Com o binário do núcleo (Fase 1) — mesma tabela de controle do `sqlx-cli`,
então os dois são intercambiáveis:

```bash
export DATABASE_URL=postgres://usuario@localhost/ecosystem_dev
cargo run -p profile-core -- db status            # só lê
cargo run -p profile-core -- db migrate           # tudo ou nada
cargo run -p profile-core -- db revert            # só a última; --to V ou --all para mais
cargo run -p profile-core -- db revert --all --allow-data-loss   # quando há dado: exporte antes
```

`migrate` e `revert` rodam numa transação só: se uma `down` recusa apagar dado,
nenhuma migration é revertida (D-024).

Com o `sqlx-cli`:

```bash
cargo install sqlx-cli --no-default-features --features postgres,rustls --locked
export DATABASE_URL=postgres://usuario@localhost/ecosystem_dev
sqlx migrate run    --source db/migrations
sqlx migrate info   --source db/migrations
sqlx migrate revert --source db/migrations        # uma por vez, da mais nova para a mais antiga
```

## 4. O que o harness prova

| Etapa | Garantia |
|:---|:---|
| 1 · round-trip por migration | para cada *k*: schema antes de *k* = schema depois de `up k` + `down k` (comparado por `pg_dump --schema-only`) |
| 2 · seed | o seed sintético carrega sobre o schema completo |
| 3 · testes SQL | 73 asserções: restrições recusam dado inválido, views públicas aplicam privacidade e CTA, histórico é append-only, leitor público só lê views |
| 4 · guardas | todo `down` com tabela **recusa** sem `ecosystem.allow_data_loss=on` e passa com ele; no fim não sobra nenhum objeto |
| 5 · idempotência do ciclo | reaplicar tudo reproduz exatamente o mesmo schema |
| 6 · sqlx | `sqlx migrate run` aplica as 7 e `revert` volta a zero |

O binário tem a própria prova, no workflow `Rust Core`:
`db/tests/profile_core_e2e.sh` confere que o schema de `profile-core db migrate`
é idêntico (`pg_dump --schema-only`) ao das migrations aplicadas pelo psql, roda
o seed e os testes SQL sobre ele e exercita a trava de perda de dados pela CLI.
`cargo test -p store` cobre o resto: tudo ou nada, migration alterada, pela
metade ou desconhecida, e a taxonomia da 0003 contra o crate de domínio.

Os testes foram validados por **mutação**: remover o filtro de exclusão, o
filtro de privados, uma guarda de `down`, um trigger de histórico, a
restrição de estado único do monitor, o trigger de escopo das métricas, o
`DEFERRABLE` da unicidade de nome, a ordenação da "última checagem", ou
voltar a filtrar exclusões por função — cada mutação faz o harness falhar pelo
motivo certo.

No CI: [`db-validation.yml`](../../.github/workflows/db-validation.yml), com
`postgres:16` como serviço e o sqlx-cli em cache. Só roda quando `db/**` muda.

## 5. Procedimento em produção

1. PR com a migration nova (`up` + `down` + seed + testes). CI verde.
2. **Backup/snapshot** do banco antes de aplicar (ou branch do provedor, se
   houver: Neon/Supabase).
3. `profile-core db migrate` (ou `sqlx migrate run`) com o papel
   `ecosystem_migrator`, num job manual protegido.
4. Produção é **forward-only**: um erro se corrige com migration nova. O
   `down` existe para desenvolvimento, CI e emergência; em produção, só com
   exportação prévia e opt-in explícito.

## 6. Carga inicial do estado existente (planejada)

Executada uma vez, por `profile-core import legacy`, dentro de um `sync_run`
do tipo `legacy_import`. Ordem e checagens:

| Passo | Origem | Destino | Verificação |
|:--|:---|:---|:---|
| 1 | `docs/README_EXCLUDED.json` | `repository_exclusions` | contagem = tamanho da lista |
| 2 | GitHub (`sync github`) | `github_owners`, `repositories`, `repository_languages`, `projects` | contagens batem com o inventário do `update_profile.py` na mesma hora |
| 3 | `README_FEATURED.json`, `README_STACK.json`, `README_SITES.json`, constantes do código (`FEATURED_SUMMARIES`, nomes fixos de `status_for`) | tabelas editoriais | `project-catalog.json` regenerado a partir do banco = o do Python |
| 4 | `docs/project-catalog.json` (`website_declared`, `website_status`, `website_http_status`) | `websites` + uma checagem inicial | nº de sites verificados igual |
| 5 | `docs/ECOSYSTEM-COMMIT-STATE.json` — `repositories.*` | `commit_observations` (uma por repositório, `commits_since_previous = NULL`) | SHA atual de cada repositório igual |
| 6 | `docs/ECOSYSTEM-COMMIT-STATE.json` — `metrics.project_commits`, `monitor_commits`, `tracked_commits` | `metric_samples` (`legacy_import`) | valores idênticos; o monitor Python continua rodando até a paridade |
| 7 | `docs/assets/contributions-timeline-data.json` | `metric_samples` (`profile.contributions.*`, janela mensal) | 13 janelas × 7 métricas |

Nenhum arquivo de origem é apagado: eles continuam sendo gerados pelo Python
até a fase C do componente correspondente ([PYTHON-TO-RUST.md](../migration/PYTHON-TO-RUST.md)).

## 7. Próximas migrations previstas

| Versão | Conteúdo | Gatilho |
|:---|:---|:---|
| `0008_deployments` | tabela `deployments` (D-016) | existir o coletor da API do provedor |
| `0009_website_policy` | política de exibição com histerese por site | decisão editorial de adotar histerese |
| — | papéis e `GRANT`s | fora das migrations: script do provedor ([DATABASE-ARCHITECTURE.md §5](DATABASE-ARCHITECTURE.md)) |
