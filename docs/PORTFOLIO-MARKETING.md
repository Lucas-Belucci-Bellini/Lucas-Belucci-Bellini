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

| Situação | Célula publicada |
|:---|:---|
| com site | `**[▸ Abrir site](url)** · [código](github)` |
| sem site | `[código](github)` |

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
| `FEATURED-PROJECTS` | coluna **Acesso** com o site na frente |
| `CURATED-FEATURED` | idem |
| `PUBLIC-PROJECTS` | coluna `GitHub` virou **Acesso**, com site quando há |
| `PROJECT-MAP` | colunas `GitHub` e `Site` fundidas em **Acesso** |
| `LIVE-PROJECTS` | colunas reordenadas para `Projeto \| Website \| Código \| Verificação` |
| `PRIVATE-PROJECTS` | inalterado — repositório privado não anuncia site |

Os marcadores `START`/`END` e a identidade visual (paleta, ASCII art, badges,
ícones, seções estáticas) seguem exatamente como estavam. A mudança é de ordem e
de peso, não de estética.

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
