# Monitor V2 — State Engine

## Objetivo

Refatorar o modelo de estado do monitor sem reescrever o coletor GitHub. A V2 preserva descoberta de repositórios, `default_branch`, comparação de SHAs, retry/backoff e o agendamento horário, mas torna a contagem e a publicação do snapshot auditáveis.

## Invariantes

1. `project_commits_total` representa somente commits detectados nos projetos acompanhados desde o baseline legado.
2. `monitor_commits_total` representa somente snapshots do próprio monitor publicados em commits bem-sucedidos.
3. `tracked_commits_total = legacy_baseline + project_commits_total + monitor_commits_total`.
4. Uma execução que falha antes do `git commit`/`git push` não publica o novo estado e não produz um incremento observável do monitor.
5. `monitor_commit_this_run` é o incremento reservado para o snapshot que a própria execução está preparando. Ele só passa a existir na branch se o commit/push dessa execução for bem-sucedido.
6. Cada execução possui um `run_id` determinístico baseado no timestamp UTC da execução.
7. O baseline legado de 1538 fica identificado separadamente e nunca volta a ser tratado silenciosamente como `project_commits_total`.
8. O snapshot não registra o próprio SHA: o SHA de um commit só existe depois que o commit é criado. A confirmação de publicação acontece no workflow após `git commit` e `git push`.

## Estado V2

```json
{
  "schema": 6,
  "run": {
    "run_id": "RUN-20260817-141700",
    "started_at": "2026-08-17T14:17:00Z",
    "completed_at": "2026-08-17T14:17:12Z",
    "status": "candidate",
    "publication_rule": "The snapshot is published only if git commit and push succeed."
  },
  "baseline": {
    "legacy_tracked_commits": 1538,
    "source": "legacy monitor state"
  },
  "metrics": {
    "project_commits_total": 12,
    "monitor_commits_total": 4,
    "tracked_commits_total": 1554,
    "project_commits_this_run": 3,
    "monitor_commit_this_run": 1,
    "legacy_baseline": 1538
  }
}
```

## Migração do estado antigo

O schema 5 guardava o valor legado `1538` dentro de `project_commits`. A V2 converte esse formato sem duplicar o baseline:

```text
schema 5 project_commits = 1542
legacy baseline           = 1538
                         ─────
project delta             =    4
```

A partir daí, a V2 mantém apenas os deltas novos em `project_commits_total` e mantém os snapshots do monitor em `monitor_commits_total`.

## Fluxo de publicação

```text
RUN START
   ↓
scan repositories
   ↓
calculate project delta
   ↓
calculate next counters
   ↓
write snapshot candidate
   ↓
git add
   ↓
git commit
   ↓
git push
   ↓
workflow verifies command success
   ↓
snapshot is published
```

Não existe uma segunda gravação para colocar o SHA do próprio commit no mesmo snapshot. Isso criaria necessariamente outro commit e quebraria a regra de um snapshot por hora. O SHA publicado pode ser verificado pelo próprio workflow (`git rev-parse HEAD`) e pelo histórico Git.

## Falhas

- Falha de API em um repositório: execução pode ficar `degraded`, sem inventar commits para esse repositório.
- Falha total do scan: execução falha; nenhum snapshot é publicado.
- Falha no commit: execução falha; nenhum novo estado chega à branch.
- Falha no push: execução falha; o próximo ciclo parte do último estado publicado.
- Falha na verificação do HEAD: execução falha; o próximo ciclo reavalia a partir do estado que realmente está na branch.

## Testes obrigatórios

O arquivo `.github/scripts/test_ecosystem_state.py` valida as invariantes sem chamar a API do GitHub:

- migração do schema 5 sem duplicar o baseline;
- nenhuma mudança nos projetos ainda produz um snapshot do monitor;
- mudanças em projetos ficam separadas do snapshot do monitor;
- baseline legado não vira `project_commits_total`;
- identidade matemática do contador rastreado.

## GitHub Contributions

A métrica de Contributions do GitHub é externa ao contador do monitor. O monitor não deve tentar reproduzi-la nem usar seu valor como baseline de commits dos projetos.

## Escala

O estado por repositório continua contendo o último SHA conhecido. Isso permite que o monitor acompanhe novos repositórios automaticamente e evita copiar históricos inteiros para o perfil.

## Critério para promoção para `main`

Antes do merge, validar pelo menos:

- execução sem mudanças nos projetos;
- execução com mudanças em um projeto;
- execução com mudanças em vários projetos;
- falha transitória da API;
- falha de publicação;
- recuperação após falha;
- identidade entre `tracked_commits_total` e a soma do baseline + categorias;
- nenhum incremento fantasma do contador do monitor;
- testes de estado passando.
