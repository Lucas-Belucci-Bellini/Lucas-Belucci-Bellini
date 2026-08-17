# Monitor V2 — State Engine

## Objetivo

Refatorar o modelo de estado do monitor sem reescrever o coletor GitHub. A V2 preserva descoberta de repositórios, `default_branch`, comparação de SHAs, retry/backoff e o agendamento horário, mas torna a contagem e a publicação do snapshot auditáveis.

## Invariantes

1. `project_commits_total` representa somente commits detectados nos projetos acompanhados.
2. `monitor_commits_total` representa somente snapshots do próprio monitor que foram efetivamente publicados.
3. `tracked_commits_total = project_commits_total + monitor_commits_total`.
4. Uma execução que falha antes da confirmação do commit não incrementa `monitor_commits_total`.
5. `monitor_commit_this_run` só pode ser `1` depois que o commit do snapshot foi criado com sucesso.
6. Cada execução possui um `run_id` determinístico baseado no timestamp UTC da execução.
7. O baseline legado de 1538 não deve voltar a ser tratado silenciosamente como `project_commits_total`; ele deve ficar identificado como baseline importado.

## Estado proposto

```json
{
  "schema": 6,
  "run": {
    "run_id": "RUN-20260817-1417",
    "started_at": "2026-08-17T14:17:00Z",
    "completed_at": "2026-08-17T14:17:12Z",
    "status": "published",
    "snapshot_sha": "..."
  },
  "baseline": {
    "legacy_tracked_commits": 1538,
    "source": "legacy monitor state"
  },
  "metrics": {
    "project_commits_total": 0,
    "monitor_commits_total": 0,
    "tracked_commits_total": 0,
    "project_commits_this_run": 0,
    "monitor_commit_this_run": 1
  }
}
```

## Fluxo de publicação

```text
RUN START
   ↓
scan repositories
   ↓
calculate project delta
   ↓
prepare candidate state
   ↓
write snapshot candidate
   ↓
git commit
   ↓
verify commit / HEAD
   ↓
mark run as published
   ↓
only then persist monitor +1
```

O ponto crítico é que o estado não pode declarar um `+1` do monitor antes de existir um commit correspondente.

## Falhas

- Falha de API em um repositório: execução pode ficar `degraded`, sem inventar commits.
- Falha total do scan: execução `failed`; não incrementa o monitor.
- Falha no commit: execução `failed`; não incrementa o monitor.
- Falha na verificação do SHA publicado: execução `failed`; o próximo ciclo deve recuperar a partir do último estado confirmado.

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
- identidade entre `tracked_commits_total` e a soma das duas categorias;
- nenhum incremento fantasma do contador do monitor.
