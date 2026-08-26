# README Data Contract

Este documento explica como o README do perfil é mantido. O objetivo é transformar o perfil em um catálogo atualizável sem publicar conteúdo privado.

## Fonte de verdade

A fonte primária é a API autenticada do GitHub, especialmente o inventário de repositórios com afiliação `owner`, `collaborator` e `organization_member`, o endpoint de linguagens de cada repositório e os metadados públicos de cada projeto. O workflow não usa a lista manual do README como fonte de dados.

A rotina também verifica as URLs declaradas como homepage no GitHub e as URLs públicas mantidas em `docs/README_SITES.json`. Uma implantação só entra na seção **Live Projects** depois de responder com código HTTP de sucesso ou redirecionamento válido durante a execução.

## O que é publicado

A seção pública pode conter nome, descrição editorial, categoria, status, link do GitHub, linguagens agregadas e deployments publicamente verificados. O catálogo mantém a distinção entre linguagens de programação/DSLs e tecnologias, frameworks, bibliotecas, ferramentas e plataformas.

Repositórios privados podem aparecer somente com nome, categoria, descrição pública disponível, status, marcador `Private repository` e link do GitHub. O processo não lê, copia ou publica código, nomes de arquivos, estrutura interna, imagens privadas, secrets, tokens, credenciais, endpoints ou arquivos `.env` privados.

## Cálculo das estatísticas

O endpoint de linguagens do GitHub retorna bytes por linguagem. O script soma os bytes de todos os repositórios públicos, ordena as linguagens por volume e calcula a participação de cada linguagem sobre o total público detectado. A contagem de repositórios por linguagem é o número de mapas de linguagem públicos que contêm aquela chave.

Os números do dashboard são calculados a partir do inventário retornado no momento da execução. `PROJECTS` conta todos os repositórios acessíveis ao token do workflow; `PUBLIC PROJECTS` e `PRIVATE PROJECTS` usam a visibilidade retornada pelo GitHub; `DEPLOYED PROJECTS` conta somente deployments públicos verificados; `ACADEMIC PROJECTS` usa uma classificação editorial determinística baseada no nome, descrição e evidência pública do repositório.

O status é uma heurística editorial transparente: privado tem prioridade; repositórios acadêmicos recebem `Academic`; repositórios sem atividade recente podem ser classificados como experimentais; e atividade recente é marcada como `Active` ou `In Development` conforme a data de push. O status não é inferido apenas pelo nome.

## Workflow

O workflow [`.github/workflows/update-profile.yml`](../.github/workflows/update-profile.yml) roda diariamente e também pode ser acionado manualmente. Ele configura Python, consulta o GitHub, executa o gerador, valida o Markdown e cria commit somente quando `README.md` ou os assets gerados realmente mudam.

O workflow não roda a cada minuto ou hora. A frequência diária reduz ruído e mantém a atualização adequada para um catálogo de perfil. O job usa `PROFILE_README_TOKEN` quando configurado; esse secret deve possuir apenas as permissões mínimas necessárias. O token é usado para métricas agregadas e nunca para publicar conteúdo de arquivos privados.

## Atualização manual

Para atualizar localmente com o inventário produzido durante uma auditoria autenticada:

```bash
python3 scripts/update_profile.py \
  --input-repos /caminho/para/repos.json \
  --languages-dir /caminho/para/languages \
  --write
```

Para uma execução direta contra a API do GitHub, forneça `PROFILE_GITHUB_TOKEN` ou `GITHUB_TOKEN` no ambiente e execute:

```bash
python3 scripts/update_profile.py --write
```

Depois, revise `README.md`, `assets/lang-stats.svg`, os links públicos e a política de privacidade antes de abrir um pull request.

## Contrato de mudança

As seções geradas são delimitadas por marcadores `START`/`END`. Não edite o conteúdo entre esses marcadores manualmente: altere a lógica em `scripts/update_profile.py`, o mapa de URLs em `docs/README_SITES.json` ou a fonte de dados. A atualização deve ser determinística, legível e idempotente.
