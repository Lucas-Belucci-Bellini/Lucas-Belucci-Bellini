# Verificação de sites

Descobrir uma URL não autoriza publicá-la. Esta etapa decide se o link entra na
vitrine — e é a razão de o perfil não anunciar deployments caídos.

Implementação: `check_website()` / `check_websites()` em
[`scripts/project_catalog.py`](../scripts/project_catalog.py).

## O que conta como "no ar"

| Resultado | `status` | Vira link? |
|:---|:---|:---|
| `200`, ou redirecionamento cujo destino respondeu | `verified` | **sim** |
| `404`, `500`, timeout, DNS que não resolve | `unreachable` | não |
| URL malformada, esquema que não é `http`/`https` | `invalid` | não |

Redirecionamento conta porque é o caso normal de apex → `www` e de `http` →
`https`: o que importa é se alguém que clicar chega a uma página. O destino final
é registrado em `website_final_url` **só quando difere da URL declarada**; o link
publicado é sempre a URL declarada, que é a estável.

A verificação usa `GET`, não `HEAD`: parte dos deployments responde `405` a `HEAD`
e `200` a `GET`, e tratar isso como "fora do ar" produziria falso negativo.

## Concorrência, timeout e retentativa

- Concorrência limitada (padrão 6). O limite existe porque vários desses sites
  ficam no mesmo provedor; disparar dezenas de requisições simultâneas contra ele
  é ruído que não acelera nada.
- Timeout padrão de 15 s por requisição.
- Uma retentativa extra, com espera curta e linear. Serve para tolerar um soluço
  de rede, **não** para insistir num site que está fora.
- URLs repetidas são verificadas uma vez só.

## Falha fecha, não abre

Se a verificação falha, o projeto **volta a apresentar só o código**. Não existe
caminho em que uma URL não verificada vire link. É o inverso do padrão comum
("na dúvida, mostra"): aqui, na dúvida, esconde.

Uma URL declarada que não respondeu não é esquecida — fica gravada em
`website_declared` no catálogo, com o `website_status` e o código HTTP. É o que
permite cobrar um deployment caído em vez de ele sumir do radar.

## Frescura: por que não há carimbo de tempo no catálogo

`docs/project-catalog.json` é versionado. Um campo `checked_at` mudaria a cada
execução e geraria um commit a cada execução, quebrando a regra do repositório:
**dado igual → README igual → nenhum commit**.

O `checked_at` existe no objeto em memória e aparece no relatório de
`scripts/check_websites.py`, que não grava nada.

## Ferramenta de plantão

```bash
python3 scripts/check_websites.py              # relatório legível
python3 scripts/check_websites.py --json       # inclui checked_at
python3 scripts/check_websites.py --fail-on-down   # sai 1 se algo caiu
```

Lê as URLs do catálogo gerado (inclusive as que estavam fora do ar) e completa
com o manifesto. Não altera README nem manifestos.

## Estado na auditoria que acompanha esta mudança

16 URLs declaradas, **8 no ar e 8 respondendo HTTP 404**:

| Fora do ar | URL declarada |
|:---|:---|
| Academic-Portfolio | `https://academic-portfolio-blush.vercel.app` |
| Ark-Initiative | `https://ark-initiative.vercel.app` |
| DailyPlanner | `https://daily-planner-phi-smoky.vercel.app` |
| DriveTax-Motors | `https://drive-tax-motors.vercel.app` |
| FLUX | `https://flux-ten-red.vercel.app` |
| NEXORA | `https://web-2qfw.vercel.app` |
| Projeto-Baluarte | `https://projeto-baluarte.vercel.app` |
| Recycle-game | `https://recycle-game-opal.vercel.app` |

Todas vêm do campo `homepage` do GitHub. `Projeto-Baluarte` é a única que o
README anterior já publicava como `HTTP 200` — a verificação corrigiu a
afirmação. As outras sete nunca chegaram a ser verificadas porque o gerador
anterior só conferia o que estava no manifesto.

Consertar esses deployments é trabalho de cada projeto, não deste repositório.
Enquanto não voltarem, os projetos aparecem com o link de código.
