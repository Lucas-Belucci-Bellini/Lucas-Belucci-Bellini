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
| Outras contribuições | Residual do total após commits, pull requests e issues; inclui reviews, repositórios e categorias adicionais quando existirem. |
| Total | Total oficial retornado pelo calendário de contribuições. |

A fonte é a API GraphQL do GitHub, usando `contributionsCollection`. Os dados brutos estão em [`assets/contributions-timeline-data.json`](assets/contributions-timeline-data.json), com as datas de início e fim de cada período e a hora de coleta.

> O gráfico não mede qualidade, produtividade ou impacto. Ele descreve a composição e a evolução temporal das atividades que o GitHub contabiliza como contribuições.

> O HTML carrega a biblioteca Plotly pelo CDN oficial. Os dados ficam embutidos no documento e não são enviados para outro serviço pela página, mas a visualização requer acesso ao CDN quando o arquivo é aberto.

## Publicação automática

O workflow [`contributions-timeline.yml`](../.github/workflows/contributions-timeline.yml) executa manualmente, diariamente por cron e quando o gerador ou este guia muda. Ele usa o Secret `PROFILE_README_TOKEN` quando configurado, com fallback para o token temporário do próprio workflow, regenera os dois artefatos e cria commit somente se houver alteração real.

Para habilitar a rotina, confirme que o arquivo está em `.github/workflows/`, crie `PROFILE_README_TOKEN` em **Settings → Secrets and variables → Actions** com o menor escopo compatível e execute o workflow uma vez em **Actions → Publish contribution timeline → Run workflow**. Não imprima o token nos logs e não inclua repositórios privados no snapshot.

A proteção recomendada para `main` exige tanto o check `V2 Validation / validate` quanto o check `Publish contribution timeline / regenerate-and-publish` apenas quando a alteração tocar a timeline. Se a política de branch exigir o workflow em todo PR, use somente o primeiro como requisito universal e deixe a publicação agendada/manual fora do bloqueio de merge.
