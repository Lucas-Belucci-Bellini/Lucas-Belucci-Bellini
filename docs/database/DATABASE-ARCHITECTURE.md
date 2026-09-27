# Arquitetura do banco

## 1. Papel do banco

O banco guarda o que **não existe em lugar nenhum** hoje — histórico de
verificação de sites, transições de commit, séries de métricas, execuções — e
uma cópia mínima e ressincronizável dos fatos do GitHub, para que todas as
saídas (README, catálogo JSON, API) partam de um único estado.

Ele **não** é:

- um espelho do GitHub (D-002): `repositories` guarda só o recorte que o
  catálogo usa;
- dependência de leitura do README (D-009, proposta): até a Fase 4, o README
  pode ser gerado sem o banco no ar;
- versionado no Git (D-003): o Git guarda migrations, seeds sintéticos,
  testes e documentação.

## 2. Por que PostgreSQL

| Necessidade | Recurso usado |
|:---|:---|
| Taxonomia fechada e integridade entre projeto, repositório e site | chaves estrangeiras, `CHECK`, `UNIQUE` parcial |
| Renomear repositório sem violar unicidade no meio da troca | `UNIQUE … DEFERRABLE INITIALLY DEFERRED` |
| Nome ativo único com soft delete | `UNIQUE NULLS NOT DISTINCT (full_name_key, gone_at)` (PG ≥ 15) |
| Histórico que não pode ser reescrito | triggers `BEFORE UPDATE/DELETE/TRUNCATE` |
| "Estado atual" sem coluna redundante | `DISTINCT ON` em views |
| Fronteira de privacidade para qualquer consumidor | views com privilégio do dono (D-010) |
| Cliente Rust com SQL checado em compilação | `sqlx` (suporte de primeira classe a PostgreSQL) |
| Fila de trabalho futura sem broker (se o monitor virar serviço) | `SELECT … FOR UPDATE SKIP LOCKED` — mesma escolha registrada no Projeto Baluarte |

SQLite foi descartado (arquivo no Git contradiz o pedido; sem acesso
concorrente do CI e de um serviço). Nenhuma extensão é exigida — o schema roda
em qualquer PostgreSQL 15+ gerenciado.

## 3. Hospedagem — decisão pendente do dono

O schema não depende da escolha. Critérios: acessível pelo GitHub Actions,
backups automáticos, custo zero ou quase no volume esperado.

| Opção | Prós | Contras |
|:---|:---|:---|
| **Supabase** (já usado pelo Veritas) | conhecido no ecossistema; backups; painel | expõe `public` via PostgREST por padrão — manter o schema `ecosystem` **fora** da API automática (D-010); projetos gratuitos pausam por inatividade |
| **Neon** | PostgreSQL serverless, branching por PR, escala a zero | cold start de alguns segundos no primeiro acesso do job |
| **Self-hosted** | controle total | backup, atualização e exposição de rede viram trabalho do dono |

Recomendação: começar por um provedor gerenciado com branching (Neon ou
Supabase) e **não** abrir API automática sobre o schema.

## 4. Ambientes

| Ambiente | Banco | Dados | Quem cria |
|:---|:---|:---|:---|
| teste (local e CI) | efêmero, criado e apagado pelo harness | seed sintético | `db/tests/run.sh`, `db-validation.yml` |
| desenvolvimento | local (`initdb` ou container) | seed sintético ou cópia anonimizada | desenvolvedor |
| sombra (Fase 2–3) | o de produção | real, escrito em paralelo ao Python | `profile-core` no Actions |
| produção | gerenciado | real | idem |

Seeds de desenvolvimento **nunca** contêm dado real (nomes, URLs, números):
usam `example.org` e ids fictícios (`db/seeds/dev/`).

## 5. Papéis e privilégios (a criar no provedor, fora das migrations)

Papéis são objetos do cluster e cada provedor os gerencia de um jeito; por isso
não entram nas migrations. Desenho mínimo:

| Papel | Privilégio | Usado por |
|:---|:---|:---|
| `ecosystem_migrator` | dono do schema `ecosystem` | `profile-core db migrate` (job manual/protegido) |
| `ecosystem_collector` | `INSERT/UPDATE` nas tabelas de fato e editoriais; `INSERT` no histórico; `SELECT` geral | `sync *`, `check sites`, `import manifests` |
| `ecosystem_renderer` | `SELECT` nas views públicas e em `private_repository_listing` | `render *` |
| `ecosystem_public_reader` | `SELECT` **só** nas views `public_*` | API pública futura |

```sql
-- exemplo, executado uma vez pelo dono do banco
GRANT USAGE ON SCHEMA ecosystem TO ecosystem_public_reader;
GRANT SELECT ON ecosystem.public_projects, ecosystem.public_repositories,
                ecosystem.public_language_totals TO ecosystem_public_reader;
```

Nenhum papel de aplicação recebe `DELETE` no histórico: as triggers já
recusariam, e o privilégio ausente é a segunda barreira.

## 6. Conexão a partir do GitHub Actions

- `DATABASE_URL` como *repository secret*, com o papel da etapa (o job de
  render não precisa do papel de coleta).
- TLS obrigatório (`sslmode=require` ou superior).
- `concurrency` comum aos jobs que escrevem (D-018): duas coletas simultâneas
  não corrompem nada (upsert + append-only), mas duplicariam trabalho.
- Falha de conexão falha o job de forma visível — nunca "verde sem fazer
  nada" (lição de A1).

## 7. Volume e retenção

| Tabela | Crescimento estimado | Retenção proposta |
|:---|:---|:---|
| `repositories`, `projects`, `websites` | dezenas de linhas | permanente (soft delete) |
| `website_checks` | ~20 sites × 24/dia ≈ 175 mil/ano | permanente no início; se crescer, agregar por dia após 1 ano (job explícito, D-008) |
| `commit_observations` | só transições: centenas/mês | permanente |
| `metric_samples` | contribuições: ~7 séries × 13 janelas/dia ≈ 33 mil/ano | permanente |
| `sync_runs` | ~30/dia ≈ 11 mil/ano | permanente |

Tudo cabe com folga em qualquer plano gratuito por anos. Particionamento não
se justifica.

## 8. Backup, exportação e recuperação

- **Backup do provedor** (PITR quando disponível) cobre falha de infraestrutura.
- **Exportações públicas continuam no Git:** `project-catalog.json`,
  `ECOSYSTEM-COMMIT-STATE.json` (schema 4, enquanto houver consumidor) e o JSON
  da timeline. Elas são simultaneamente o contrato público e uma cópia legível
  do estado público.
- **Recuperação do zero:** fatos do GitHub são recoletáveis; editorial está
  nos manifestos em Git; o que **não** é recuperável sem backup é o histórico
  (checagens, observações, métricas) — por isso ele é append-only e protegido
  por trigger.

## 9. Relação com os outros repositórios do ecossistema

O banco pertence ao perfil. Outros projetos (Projeto Baluarte, Veritas…) podem
**ler** o catálogo pela API ou pelo JSON estático, nunca pelas tabelas: o
`AGENTS.md` exige contrato explícito entre repositórios, e a view pública é
esse contrato.
