# Métricas avançadas para o README

Este documento descreve métricas que podem enriquecer o portfólio sem transformar o README em um painel opaco. Cada número deve informar **fonte, janela temporal, data de coleta e escopo**.

## O que vale a pena incorporar

| Métrica | Fonte | Valor narrativo | Limitação |
|:---|:---|:---|:---|
| Contribuições totais por tipo | GitHub GraphQL `contributionsCollection` | Mostra a composição entre commits, PRs, issues, reviews e repositórios. | É uma janela móvel; não é um total histórico imutável. |
| Commits semanais | REST `stats/commit_activity` | Revela cadência de implementação no último ano. | Mede commits, não impacto ou qualidade. |
| Adições e deleções | REST `stats/code_frequency` | Mostra volume de mudança de código. | Pode retornar 422 em repositórios com 10.000 commits ou mais. |
| Participação do proprietário | REST `stats/participation` | Compara commits do proprietário com a atividade total do repositório. | Cobre as últimas 52 semanas. |
| Horário dos commits | REST `stats/punch_card` | Ajuda a descrever padrões de atividade. | Não deve ser interpretado como produtividade ou disponibilidade. |
| Stars e forks | REST/GraphQL de repositórios | Indica alcance e reutilização observáveis. | Não é medida automática de qualidade. |
| Issues e pull requests abertas | REST/GraphQL | Mostra superfície de manutenção e colaboração. | Precisa de contexto por projeto. |
| Releases e última publicação | REST `releases` | Demonstra entrega versionada e manutenção. | Nem todo projeto precisa de releases. |
| Última atividade do repositório | GraphQL `lastContributionDate` | Sinaliza projetos ativos ou abandonados. | Está documentada como métrica beta pública. |
| Commits da branch padrão | GraphQL `commitCount` | Permite acompanhar evolução de um repositório. | Está documentada como métrica beta pública. |
| Views, visitantes e clones | REST `traffic/views` e `traffic/clones` | Mede descoberta recente de repositórios. | Requer acesso de escrita e cobre somente os últimos 14 dias. |
| Paths e referrers populares | REST Traffic | Mostra quais páginas e origens geram interesse. | Também cobre somente os últimos 14 dias. |

## Política de apresentação

Métricas oficiais do GitHub devem aparecer com o nome da fonte e a janela de coleta. Métricas de terceiros devem ser rotuladas como externas. O contador de perfil usado no README é um indicador de carregamentos do badge; ele não é a mesma coisa que visitas oficiais ao perfil.

Para evitar comparações enganosas, não coloque `contribuições totais` e `commits` como se fossem a mesma grandeza. A primeira é composta por tipos distintos de atividade; a segunda é apenas a contagem de commits atribuídos ao perfil na janela selecionada.

## Visitantes: duas arquiteturas possíveis

| Opção | Como funciona | Custo/complexidade | Quando usar |
|:---|:---|:---|:---|
| Badge externo | Um serviço externo incrementa o contador quando o badge é carregado. | Baixa; sem workflow. | Quando o objetivo é um indicador visual simples. |
| Workflow com tráfego de repositórios | Uma Action consulta `traffic/views` e `traffic/clones`, grava um resumo versionado e atualiza o README. | Média; requer Secret e acesso de escrita. | Quando você quer métricas oficiais de projetos específicos. |

O GitHub não documenta um endpoint público e estável para visualizações do perfil pessoal equivalente ao tráfego de um repositório. Se usar o badge externo, mantenha o rótulo `External profile views` e a nota explicativa no README.

## Referências

[1] [REST API — repository statistics](https://docs.github.com/en/rest/metrics/statistics)

[2] [REST API — repository traffic](https://docs.github.com/en/rest/metrics/traffic)

[3] [Viewing traffic to a repository](https://docs.github.com/en/repositories/viewing-activity-and-data-for-your-repository/viewing-traffic-to-a-repository)

[4] [Additional repository metrics via GraphQL](https://github.blog/changelog/2023-10-24-additional-repository-metrics-available-via-graphql-api/)
