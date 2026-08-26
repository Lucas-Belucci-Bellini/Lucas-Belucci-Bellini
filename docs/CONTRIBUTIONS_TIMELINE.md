# Contributions Timeline

O arquivo [`assets/contributions-timeline.html`](assets/contributions-timeline.html) é um gráfico interativo da atividade do perfil na janela móvel de 365 dias. Ele contém duas leituras: volume mensal empilhado e participação percentual por tipo.

## Como usar

Passe o cursor sobre um mês para ver o período, a quantidade da série e o total. Use o range slider para aproximar uma janela menor ou o botão **Últimos 6 meses** para focar na atividade recente. O botão de download do próprio Plotly permite salvar uma imagem da visão atual.

## O que está sendo medido

| Série | Significado |
|:---|:---|
| Commits | Commits atribuídos ao perfil na janela consultada. |
| Pull requests | Pull requests atribuídos ao perfil. |
| Issues | Issues atribuídas ao perfil. |
| Reviews | Reviews de pull request atribuídas ao perfil. |
| Repositórios | Contribuições relacionadas à criação de repositórios. |
| Total | Total oficial retornado pelo calendário de contribuições. |

A fonte é a API GraphQL do GitHub, usando `contributionsCollection`. Os dados brutos estão em [`assets/contributions-timeline-data.json`](assets/contributions-timeline-data.json), com as datas de início e fim de cada período e a hora de coleta.

> O gráfico não mede qualidade, produtividade ou impacto. Ele descreve a composição e a evolução temporal das atividades que o GitHub contabiliza como contribuições.

> O arquivo HTML é autocontido e inclui o Plotly localmente para permitir consulta offline depois do download. Ele não envia os dados para outro serviço durante a visualização.
