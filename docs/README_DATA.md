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

O workflow [`.github/workflows/update-profile.yml`](../.github/workflows/update-profile.yml) roda diariamente e também pode ser acionado manualmente. Ele configura Python, consulta o GitHub, executa o gerador quando o secret `PROFILE_README_TOKEN` está configurado, valida o Markdown e cria commit somente quando `README.md` ou os assets gerados realmente mudam.

O workflow não roda a cada minuto ou hora. A frequência diária reduz ruído e mantém a atualização adequada para um catálogo de perfil. Sem `PROFILE_README_TOKEN`, o job preserva o README atual e não executa uma atualização pública que poderia apagar a seção de projetos privados. O token deve possuir apenas as permissões mínimas necessárias. Quando usado, ele serve para métricas agregadas e nunca para publicar conteúdo de arquivos privados.

O timestamp da tabela de linguagens é derivado da data mais recente de `pushed_at`/`updated_at` do inventário, e não do relógio da execução. Assim, uma execução idêntica não cria uma mudança artificial só porque o workflow rodou em outro dia.

## Atualização manual

Para atualizar localmente com o inventário produzido durante uma auditoria autenticada:

```bash
python3 scripts/update_profile.py \
  --input-repos /caminho/para/repos.json \
  --languages-dir /caminho/para/languages \
  --write
```

Para uma execução direta contra a API do GitHub, forneça `PROFILE_GITHUB_TOKEN` com acesso de leitura aos repositórios que deseja agregar e execute:

```bash
PROFILE_GITHUB_TOKEN=... python3 scripts/update_profile.py --write
```

O gerador recusa o modo `--write` sem um inventário privado-aware, justamente para evitar substituir o catálogo completo por uma visão pública parcial. Para uma prévia pública sem escrita, execute `python3 scripts/update_profile.py` sem `--write`.

Depois, revise `README.md`, `assets/lang-stats.svg`, os links públicos e a política de privacidade antes de abrir um pull request.

## Contrato de mudança

As seções geradas são delimitadas por marcadores `START`/`END`. Não edite o conteúdo entre esses marcadores manualmente: altere a lógica em `scripts/update_profile.py`, o mapa de URLs em `docs/README_SITES.json` ou a fonte de dados. A atualização deve ser determinística, legível e idempotente.


## Preservação da identidade visual

A versão atual do README preserva a estética original do perfil: banner `capsule-render`, faixas animadas `readme-typing-svg`, badges de status/universidade/localização, bloco ASCII `BALUARTE // FIELD MANUAL`, ícones `skillicons`, gráficos e assets de estatísticas, console visual do J.A.R.V.I.S., build log de CPU, mapa de atividade, canais de contato, contador de visitas e footer. Os blocos auditáveis são inseridos entre marcadores dedicados para que a automação atualize os dados sem remover esses elementos editoriais e visuais.

A rotina de refresh deve ser executada sobre esse template preservado. O script [`scripts/restore_original_style.py`](../scripts/restore_original_style.py) documenta a migração pontual a partir da branch de backup; a atualização diária usa apenas [`scripts/update_profile.py`](../scripts/update_profile.py) e não substitui o template inteiro.


## Seção personalizada e Arsenal expandido

A seção `CURATED-FEATURED` é controlada pelo manifesto [`README_FEATURED.json`](README_FEATURED.json). Ela aceita apenas projetos encontrados no inventário autenticado e ignora automaticamente entradas privadas ou inexistentes. O texto editorial, a ordem e o foco podem ser ajustados nesse manifesto sem alterar o template visual restaurado.

O bloco `ARSENAL-STACK` combina duas fontes. As linguagens, pesos e contagens vêm dos mapas públicos de linguagens do GitHub; as ferramentas, plataformas e ambientes vêm do manifesto [`README_STACK.json`](README_STACK.json), que registra a evidência pública usada para cada item. A atualização não publica arquivos privados.

As ferramentas são agrupadas em quatro categorias de leitura rápida: **Frameworks & Web**, **Infraestrutura & DevOps**, **IA & Conhecimento** e **Hardware & Simulação**. A categoria de linguagens permanece separada porque é a única calculada diretamente por bytes e contagem de repositórios. Uma ferramenta só entra no Arsenal quando existe evidência pública documentada no manifesto.

## Contribuições e commits

Os cards de atividade usam a janela móvel de 365 dias da API GraphQL do GitHub. `Contribuições totais` é o total do calendário e pode combinar commits, pull requests, issues, reviews e contribuições de repositório; `Commits diretos` é somente `totalCommitContributions`. Portanto, os dois números não precisam ser iguais. O monitor de ecossistema possui ainda uma métrica própria de commits rastreados, que não deve ser comparada diretamente ao calendário de contribuições.

## Como verificar que o workflow só muda seções dinâmicas

A rotina cria `/tmp/README.before.md` antes do refresh e executa [`scripts/validate_dynamic_sections.py`](../scripts/validate_dynamic_sections.py) depois dele. O validador substitui temporariamente o conteúdo entre os marcadores `START/END` dos blocos gerados e compara o restante do README antes e depois. Se qualquer banner, badge, bloco ASCII, texto editorial, link visual ou outra parte estática mudar, o job falha antes do commit.

Para uma verificação manual local, gere uma cópia do README antes da execução, rode o gerador com os mesmos dados e compare os arquivos pelo script:

```bash
cp README.md /tmp/README.before.md
python3 scripts/update_profile.py --input-repos /caminho/repos.json --languages-dir /caminho/languages --write
python3 scripts/validate_dynamic_sections.py --before /tmp/README.before.md --after README.md
```

O resultado esperado é `dynamic-only README validation: pass`. No GitHub, a confirmação adicional é verificar o log do job `Validate generated Markdown and configuration`, o resumo do commit e o diff do workflow. Um refresh sem mudanças reais deve terminar sem novo commit, porque a etapa final usa `git diff --quiet -- README.md assets/lang-stats.svg`.

O workflow [`v2-validation.yml`](../.github/workflows/v2-validation.yml) também executa [`scripts/validate_language_badges.py`](../scripts/validate_language_badges.py) em cada pull request e em pushes na `main`. Esse check confirma que os 17 labels são únicos e legíveis, que `C#` e `PL/pgSQL` não aparecem percent-encoded como texto, que cada URL usa o endpoint estático do Shields.io, que as quatro categorias estão presentes, que a matriz tem a mesma quantidade de linguagens e que todos os badges respondem com HTTP 2xx/3xx.


## Exclusões editoriais permanentes

Os repositórios listados em [`README_EXCLUDED.json`](README_EXCLUDED.json) são removidos antes de qualquer cálculo ou renderização. Isso significa que não aparecem no mapa, no catálogo público ou privado, nos destaques, nos sites, nas linguagens agregadas, no dashboard ou em links gerados. A decisão atual exclui três projetos privados por solicitação do proprietário do perfil.

O teste [`scripts/validate_exclusions.py`](../scripts/validate_exclusions.py) falha se qualquer nome excluído aparecer no README. O inventário autenticado continua podendo conter esses projetos para fins de controle interno, mas o README e suas métricas trabalham com o inventário filtrado.
