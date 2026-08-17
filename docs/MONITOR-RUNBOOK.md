# Monitor do Ecossistema — Runbook de Auditoria

## Objetivo

O repositório de perfil funciona como um índice vivo dos projetos públicos do usuário. Ele não copia o histórico dos projetos: registra o último SHA conhecido de cada repositório e agrega a quantidade de commits detectada entre varreduras.

## Contadores oficiais do monitor

- `project_commits`: acumulado de commits detectados nos projetos acompanhados.
- `monitor_commits`: quantidade de snapshots horários publicados pelo próprio monitor.
- `tracked_commits`: `project_commits + monitor_commits`.
- `detected_project_commits_this_scan`: commits novos detectados nesta execução.
- `monitor_commit_this_scan`: deve ser `1` em toda execução que chega à etapa de publicação do snapshot.

Esses números **não são a métrica GitHub Contributions**.

## Regra do snapshot horário

A cada execução programada:

1. Descobrir os repositórios públicos do usuário.
2. Ignorar forks.
3. Usar o `default_branch` de cada repositório.
4. Ler o último SHA de cada branch.
5. Comparar com o SHA da varredura anterior.
6. Somar os commits novos que o GitHub conseguir determinar.
7. Incrementar `monitor_commits` em exatamente `1` para representar o snapshot que a própria execução publicará.
8. Gravar estado, health e relatório.
9. Criar o commit `chore(bot): snapshot horário do ecossistema [skip ci]`.

Uma execução do Actions **não é automaticamente um commit**. O contador só considera `monitor_commit_this_scan = 1` quando a execução chegou à geração do snapshot. O commit físico é produzido pela etapa seguinte do workflow.

## Auditoria de uma execução

O mínimo que deve ser possível reconstruir é:

```text
run do Actions
  -> timestamp
  -> repositórios escaneados
  -> mudanças por projeto
  -> commits de projetos detectados
  -> +1 snapshot do monitor
  -> novo total
  -> commit do snapshot
```

O histórico do Git é a fonte de verdade para verificar se o snapshot realmente foi publicado. O JSON do monitor é a fonte de verdade para a contabilidade que aquela execução calculou.

## Saúde

`health.status`:

- `healthy`: nenhuma consulta de repositório falhou.
- `degraded`: pelo menos uma consulta falhou, mas o scan continuou.

O monitor deve continuar coletando os demais repositórios quando uma consulta individual falhar. Retries com backoff protegem contra falhas transitórias da API.

## Capacidade anual

O agendamento é horário. Em um ano comum, 24 × 365 = **8.760 execuções programadas**. Isso é uma capacidade teórica: GitHub Actions pode atrasar ou limitar execuções, e uma execução cancelada não deve ser contabilizada como snapshot publicado.

## Diagnóstico de `Cannot retrieve latest commit`

Esse aviso de interface não deve ser tratado como prova de falha do monitor. Diagnosticar nesta ordem:

1. Verificar o último workflow run.
2. Verificar se terminou com `success`.
3. Verificar o SHA do commit na branch monitorada.
4. Verificar `docs/ECOSYSTEM-COMMIT-STATE.json`.
5. Só depois investigar a interface do GitHub.

## Regra de branches

- `main`: estado publicado/estável.
- `monitor/*`: desenvolvimento e endurecimento do monitor.
- O monitor deve acompanhar a branch padrão de cada projeto através de `default_branch`, sem assumir que todos usam `main`.

## Proteção contra loop

O workflow roda por `schedule` e `workflow_dispatch`. O `push` é limitado às mudanças do próprio script/workflow na `main`, enquanto o snapshot usa `[skip ci]`. A intenção é impedir um ciclo de `snapshot -> snapshot -> snapshot`.

## Regra de segurança da métrica

Nunca substituir `project_commits`, `monitor_commits` ou `tracked_commits` por uma leitura aproximada de GitHub Contributions. As métricas têm definições diferentes e devem permanecer separadas no README e nos relatórios.
