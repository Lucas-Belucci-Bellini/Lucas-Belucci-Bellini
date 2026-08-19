# V2 do perfil Lucas-Belucci-Bellini

## Objetivo

Transformar o repositório de perfil em um índice automatizado, estável e de baixo ruído. O perfil deve acompanhar mudanças do ecossistema sem replicar cada commit dos projetos nem produzir commits próprios quando não houver alteração semântica.

## Onda 1 — concluída nesta V2

A primeira vertical slice estabiliza o monitor do ecossistema. O script `ecosystem_watch.py` agora exclui o próprio repositório de perfil, compara somente o estado semântico dos repositórios e sai sem escrever arquivos quando os SHAs e erros não mudaram. O workflow continua verificando o ecossistema a cada hora, mas o histórico só recebe snapshot quando existe informação nova.

A onda também adiciona testes unitários para exclusão do perfil, no-op, alteração de SHA e mudança de estado de erro. O workflow `v2-validation.yml` executa esses testes em mudanças de código, testes ou workflows, sem disparar para os snapshots de documentação gerados pelo monitor.

## Onda 2 — próxima

Adicionar paginação observável e limites por lote no scanner, registrar duração por fase e taxa de erro no snapshot e reduzir chamadas repetidas por meio de um cache de descoberta de repositórios. A saída pública deve continuar compatível com o schema atual.

## Onda 3 — próxima

Adicionar um job de reconciliação diário para detectar mudanças que possam ter sido perdidas durante uma falha horária, sem converter a reconciliação em commits extras quando o estado final não mudou.

## Contratos e invariantes

| Área | Invariante |
| --- | --- |
| Publicação | Estado sem mudança semântica não gera commit. |
| Self-monitor | O repositório `Lucas-Belucci-Bellini` nunca é contado como projeto. |
| Falha parcial | Erro de um projeto não derruba os demais. |
| Histórico | Um snapshot publicado incrementa o contador do próprio monitor uma vez. |
| Compatibilidade | `ECOSYSTEM-COMMIT-STATE.json` e `ECOSYSTEM-COMMIT-MONITOR.md` permanecem públicos e legíveis. |
| Segurança | Token só entra por variável de ambiente/segredo; nenhum segredo é gravado no Git. |

## Critérios de saída da V2

A V2 só avança quando os testes unitários passam, os workflows de validação ficam verdes, o histórico deixa de crescer em ciclos sem mudança e o snapshot registra erros parciais sem perder os repositórios saudáveis.
