#!/usr/bin/env bash
# Empurra o commit local do bot para o branch do workflow, tolerando que outro
# workflow tenha empurrado no meio tempo (auditoria, A8).
#
# Os quatro workflows que escrevem no main commitam arquivos disjuntos, então o
# rebase nunca deveria conflitar. Se conflitar, é defeito de posse de arquivo:
# o script falha alto em vez de resolver sozinho.
#
# Grupo de concorrência comum foi descartado de propósito: no GitHub, um run
# novo cancela o run PENDENTE do mesmo grupo, e o monitor horário poderia
# cancelar o refresh diário na fila (docs/DECISION-LOG.md, D-022).
#
#   uso: bash .github/scripts/push_with_rebase.sh [branch]
set -euo pipefail

branch="${1:-${GITHUB_REF_NAME:?defina GITHUB_REF_NAME ou passe o branch}}"
attempts=4

for attempt in $(seq 1 "$attempts"); do
  if git push origin "HEAD:${branch}"; then
    exit 0
  fi
  if [ "$attempt" -eq "$attempts" ]; then
    break
  fi
  echo "push recusado (tentativa ${attempt}/${attempts}); rebase sobre origin/${branch}"
  sleep $((attempt * 3))
  git pull --rebase --quiet origin "${branch}"
done

echo "::error::push para ${branch} falhou após ${attempts} tentativas"
exit 1
