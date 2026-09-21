# O perfil como vitrine

Este repositório deixou de ser uma lista de repositórios. A mudança cabe em uma
frase:

> **O site do projeto é a experiência. O GitHub é a prova técnica.**

Quem chega no perfil não quer, em primeiro lugar, ler código — quer ver a coisa
funcionando. Quem quiser o código continua a um clique de distância, sempre.

## A regra central

```
projeto COM site verificado  →  CTA primário = site,   secundário = código
projeto SEM site             →  CTA primário = código, sem CTA secundário
```

Implementada em um lugar só, `resolve_presentation()` em
[`scripts/project_catalog.py`](../scripts/project_catalog.py), e renderizada por
`cta_cell()`:

| Situação | Célula publicada | Botões do card |
|:---|:---|:---|
| com site | `**[▸ Abrir site](url)** · [código](github)` | `▸ ABRIR SITE` em ouro + `CÓDIGO` apagado |
| sem site | `[código](github)` | `CÓDIGO` em ouro, sozinho |

Nos cards os CTAs são badges, não texto: a diferença de cor faz a hierarquia
aparecer antes de o visitante ler o rótulo. O ouro `#d4a24e` é o mesmo do resto
do perfil, e o botão secundário usa o roxo apagado `#4b3a5c`.

O destaque do site não é decorativo: ele vem primeiro na linha e em negrito, e o
código vem depois em peso normal. Um leitor que só bate o olho na tabela vê o
produto, não o repositório.

### O que o GitHub nunca deixa de ser

`github_visible` é `true` para todo projeto do catálogo. Nenhum projeto perde o
link do código — ele muda de posição, não de existência. O que muda é `github_role`:
`primary` quando é o único acesso, `source` quando existe um site na frente.

## Prioridade de exibição

`marketing_priority` é um inteiro de 0 a 100 que **só** ordena a vitrine. Não é
nota de qualidade técnica e não aparece para o visitante. Ele existe explícito
para que a ordem possa ser discutida em vez de parecer arbitrária.

| Fator | Peso | Por quê |
|:---|---:|:---|
| site verificado no ar | **+45** | é o que o visitante veio ver |
| site declarado, mas fora do ar | +5 | existe intenção de produto, sem entrega |
| curadoria manual (`README_FEATURED.json`) | +25 na ordem 1, −3 por posição | curadoria vence heurística |
| descrição pública com ≥ 25 caracteres | +10 | projeto que se explica |
| repositório público | +10 | o que o visitante consegue abrir |
| não é fork | +5 | trabalho próprio |
| não está arquivado | +5 | ainda de pé |

Empate é desfeito por nome, nunca por data — datas mudariam a ordem sem nada ter
mudado, e produziriam commit a cada execução.

## Onde a regra aparece no README

| Bloco | O que mudou |
|:---|:---|
| `WHAT-I-BUILD` | **novo** — os seis domínios de trabalho, com contagem real |
| `PRODUCT-CARDS` | **novo** — a vitrine em cards, 6 produtos, site com botão em destaque |
| `WEBSITE-DIRECTORY` | **novo** — diretório enxuto de todos os sites no ar |
| `ECOSYSTEM-MAP` | **novo** — árvore do ecossistema por categoria, com quantos têm site |
| `FEATURED-PROJECTS` | tabela **única**: absorveu `CURATED-FEATURED`, com coluna **Acesso** |
| `PUBLIC-PROJECTS` | coluna `GitHub` virou **Acesso**, com site quando há |
| `PROJECT-MAP` | colunas `GitHub` e `Site` fundidas em **Acesso** |
| `LIVE-PROJECTS` | colunas reordenadas para `Projeto \| Website \| Código \| Verificação` |
| `PRIVATE-PROJECTS` | inalterado — repositório privado não anuncia site |

Os marcadores `START`/`END` e a identidade visual (paleta, ASCII art, badges,
ícones, seções estáticas) seguem exatamente como estavam. A mudança é de ordem e
de peso, não de estética.

## A ordem de leitura do README

O README passou a ser lido como uma homepage compacta. Nenhuma seção foi
reescrita — foram movidas inteiras, e um teste de permutação confere que nenhuma
linha se perdeu no caminho.

```
HERO                  ← identidade + 4 botões de entrada
     ↓
COMECE POR AQUI       ← menu de navegação com âncoras
     ↓
FICHA DE AGENTE       ← quem é, PT e EN
     ↓
O QUE EU CONSTRUO     ← seis domínios, com contagem real
     ↓
PRODUTOS              ← cards com botão de abrir
     ↓
MISSÕES EM DESTAQUE   ← a tabela única
     ↓
SITES NO AR           ← diretório + tabela de verificação
     ↓
BALUARTE              ← ecossistema-farol e seus módulos
     ↓
ECOSSISTEMA DIGITAL   ← o mapa, depois o catálogo completo
     ↓
ARSENAL               ← tecnologias, matriz de linguagens recolhida
     ↓
ENGENHARIA            ← atividade, streak, troféus
     ↓
MÉTRICAS              ← snapshot e inventário de repositórios
     ↓
ARQUIVO PESSOAL       ← gaming e fan fiction, recolhidos
     ↓
CONTATO               ← profissional · conteúdo · pessoal
```

A estatística saiu do topo. Ela continua inteira, mais abaixo, posicionada como
evidência de engenharia — não como capa.

## Uma seção de destaque, não duas

`MISSÕES EM DESTAQUE` e `MISSÕES ESCOLHIDAS` faziam quase a mesma coisa e
repetiam os mesmos projetos com rótulos diferentes. Viraram uma tabela só:
quem está em `README_FEATURED.json` entra primeiro, com o rótulo e o foco que o
operador escreveu; o resto do espaço (até 10 linhas) vai para os projetos
públicos de maior `featured_score` que ainda não apareceram. Um projeto retirado
pela regra `website_required` não volta pela porta dos fundos da heurística.

## O que ficou recolhido

Nada foi apagado. Saiu do corpo principal e foi para `<details>`:

| Conteúdo | Onde está agora |
|:---|:---|
| participação de cada linguagem | `<details>` dentro do Arsenal |
| catálogo de repositórios públicos e privados | `<details>`, como já estavam |
| gaming e fan fiction | `<details>` no Arquivo Pessoal |

## Domínios: a cobertura é verificada

`DOMAINS` agrupa os rótulos de `classify()` em seis domínios de leitura rápida.
`DOMAIN_LABELS` reúne todos os rótulos cobertos e um teste falha se uma
categoria nova ficar de fora — sem ele, uma categoria recém-criada sumiria da
vitrine em silêncio.

## Curadoria: `docs/README_FEATURED.json`

O manifesto ganhou três campos, todos opcionais; um arquivo sem eles continua
funcionando igual.

| Campo | Efeito |
|:---|:---|
| `priority` | 0–100. Substitui o peso derivado de `order` no `marketing_priority`. |
| `reason` | Por que a entrada está na curadoria. Documentação, não é renderizado. |
| `website_required` | `true` retira a entrada da tabela curada enquanto o site não estiver verificado. |

`website_required` é **opt-in**: hoje todas as entradas estão em `false`, então
nenhum projeto desaparece por causa de um deployment caído. Quem quiser a regra
mais dura liga por entrada.

## O que a vitrine não faz

- **Não inventa URL.** Ver [`WEBSITE-DISCOVERY.md`](WEBSITE-DISCOVERY.md).
- **Não publica link não verificado.** Ver [`WEBSITE-VERIFICATION.md`](WEBSITE-VERIFICATION.md).
- **Não fabrica métrica.** Nenhum número de visitas, downloads, estrelas, cliques
  ou usuários aparece em lugar nenhum. Os contadores do dashboard são contagens
  do inventário retornado pela API na hora da execução.
- **Não expõe repositório privado.** Nome, descrição pública, status e link —
  nada além disso, e nunca um site.
- **Não some com o código.** Marketing aqui é ordem de leitura, não omissão.

## Validação

```bash
python3 scripts/validate_project_links.py   # a regra de CTA, no catálogo e no README
python3 scripts/check_websites.py           # saúde dos deployments
python3 -m unittest discover -s tests       # inclui tests/test_project_catalog.py
```
