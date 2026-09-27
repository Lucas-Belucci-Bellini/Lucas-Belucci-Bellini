# Arquitetura do sistema

Como o repositório de perfil funciona hoje, para onde ele vai e por que.
O estado atual está medido na [auditoria de 2026-09-25](../audits/2026-09-25-ecosystem-core-audit.md);
as decisões estão no [log de decisões](../DECISION-LOG.md).

## 1. Hoje: cinco coletores, um arquivo de cada vez

```text
                              GitHub (REST + GraphQL)
        ┌─────────────┬──────────────┼──────────────┬──────────────────┐
        │             │              │              │                  │
 update_profile   lang_stats   profile_cards   ecosystem_watch   contribution_timeline
  (diário*)       (semanal)     (semanal)        (horário)          (diário)
        │             │              │              │                  │
        │ inventário  │ inventário   │ GraphQL      │ inventário       │ GraphQL
        │ próprio     │ próprio      │              │ próprio          │
        ▼             ▼              ▼              ▼                  ▼
  README (13 blocos)  lang-stats.svg ◀── conflito ──┐  STATE.json      timeline JSON/HTML
  project-catalog.json profile-top-langs.svg        │  MONITOR.md
  profile-snapshot.svg profile-*.svg (4)            │
  lang-stats.svg ───────────────────────────────────┘
        │
        └── cada workflow: git commit && git push no main   (* hoje pulado: A1)
```

Características:

- **Sem estado central.** Cada coletor refaz o inventário com sua própria
  regra (A3); o único estado entre execuções é um JSON versionado
  (`ECOSYSTEM-COMMIT-STATE.json`).
- **O Git é o banco.** Todo resultado vira commit; o histórico do Git é o
  único histórico. Por isso a regra "dado igual → nenhum commit" é tão
  importante — e por isso não há histórico de verificação de sites: guardá-lo
  exigiria um commit por checagem.
- **O README é ao mesmo tempo saída e armazenamento.** Os blocos gerados são
  reescritos no lugar; o que estava antes só existe no `git log`.

## 2. Alvo: um núcleo, um estado, várias saídas

```text
GitHub (REST + GraphQL)          manifestos editoriais (docs/README_*.json)
        │                                   │
        ▼                                   ▼
┌──────────────────────── profile-core (Rust) ────────────────────────┐
│  github-client ──▶ Collector ──▶ Normalizer ──▶ ecosystem-domain     │
│   (paginação, ETag,               (um inventário,   (classificação,   │
│    rate limit)                     uma regra)        status, CTA)     │
│                                        │                              │
│  site-monitor ◀────────── websites ────┤                              │
│   (HTTP, redirects, tempo, histórico)  │                              │
│                                        ▼                              │
│                               store (sqlx + migrations)               │
└────────────────────────────────────────┬─────────────────────────────┘
                                         ▼
                              PostgreSQL · schema ecosystem
             repositories · projects · websites · website_checks · metrics
             commit_observations · sync_runs · views públicas
                                         │
          ┌──────────────────────────────┼──────────────────────────────┐
          ▼                              ▼                              ▼
   profile-render                  JSON estático                   API read-only
   README (13 blocos) + SVGs       project-catalog.json            (quando houver
   ↓                               (API v0)                         consumidor)
   GitHub Actions: 1 job, 1 commit
```

Os componentes estão detalhados em [COMPONENTS.md](COMPONENTS.md); o caminho
de um dado de ponta a ponta, em [DATA-FLOW.md](DATA-FLOW.md).

## 3. Fontes de verdade

| Camada | É a verdade sobre | Não é a verdade sobre |
|:---|:---|:---|
| **GitHub** | existência, nome, descrição, visibilidade, linguagens, homepage, branch, commits, datas | classificação, curadoria, estado de site, histórico |
| **Banco** | histórico (sites, commits, métricas, execuções) e a cópia efetiva do editorial | fatos do repositório (é espelho mínimo, ressincronizado) |
| **Manifestos em Git** | decisões editoriais, durante a transição (D-002) | nada derivado |
| **README** | nada — é representação pública derivada | — |

Regra prática: se uma informação pode ser buscada no GitHub, o banco guarda
só o recorte que o catálogo usa (`repositories` não tem estrelas, forks,
issues…). Se ela só existe porque o sistema observou ou alguém decidiu
(checagem de site, curadoria, contador do monitor), o banco é o único lugar.

## 4. Modos de operação durante a migração

A migração não tem um "dia da virada". Ela passa por quatro modos, e cada
componente pode estar num modo diferente:

| Modo | Quem gera o artefato público | Quem grava no banco | Verificação |
|:---|:---|:---|:---|
| **A · atual** | Python | ninguém | testes Python |
| **B · sombra** | Python | Rust (em paralelo) | Rust renderiza para arquivo temporário; `diff` contra a saída do Python no CI |
| **C · virada** | Rust | Rust | validadores Python checam a saída do Rust (D-015) |
| **D · consolidado** | Rust | Rust | Python correspondente removido |

Um componente só avança de modo quando o `diff` do modo anterior ficou vazio
por um período combinado (proposta: 14 execuções seguidas). Ver
[PYTHON-TO-RUST.md](../migration/PYTHON-TO-RUST.md).

## 5. Execução e implantação

| Peça | Onde roda | Observação |
|:---|:---|:---|
| Coleta e geração | GitHub Actions (agendado) | um binário `profile-core`, cacheado entre execuções |
| Banco | PostgreSQL gerenciado (hospedagem a decidir — [DATABASE-ARCHITECTURE.md §3](../database/DATABASE-ARCHITECTURE.md)) | acesso por `DATABASE_URL` em secret |
| Monitor contínuo de sites | opcional: o mesmo binário em modo serviço | só se a frequência horária do Actions não bastar |
| API | opcional, fase posterior | ver [API-ARCHITECTURE.md](../api/API-ARCHITECTURE.md) |

## 6. Segurança e privacidade

- **Segredos só em variável de ambiente/secret**: `PROFILE_README_TOKEN`
  (GitHub, *fine-grained*, só leitura de metadados), `DATABASE_URL` (papel com
  o mínimo de privilégio para a etapa). Nada disso entra em arquivo versionado.
- **Nenhum conteúdo de repositório privado é lido.** O núcleo consulta
  metadados e mapas de linguagem; a leitura da árvore de arquivos
  (`lang_stats.py`) fica restrita a repositórios públicos.
- **A fronteira de publicação é uma view** (`ecosystem.public_projects`,
  D-010). Papéis de leitura pública não enxergam tabelas.
- **Menos dado é mais seguro:** repositório excluído não é armazenado (D-019);
  `repositories` não copia campos do GitHub que o catálogo não usa.

## 7. O que esta arquitetura não é

- Não é um espelho do GitHub. Commits, issues e PRs continuam lá.
- Não substitui o README. O README continua sendo a apresentação pública,
  com a mesma identidade visual e os mesmos marcadores; o banco o alimenta.
- Não é um serviço obrigatório no ar. Até a Fase 4, tudo roda em jobs
  agendados e o README não depende do banco estar acessível (D-009).
