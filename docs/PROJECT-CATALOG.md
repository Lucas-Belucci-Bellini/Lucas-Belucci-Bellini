# `docs/project-catalog.json`

O catálogo legível por máquina de tudo que o perfil apresenta. Gerado por
[`scripts/update_profile.py`](../scripts/update_profile.py) na mesma execução que
reescreve o README, para que os dois nunca discordem.

> **Não editar à mão.** Ajuste `docs/README_SITES.json`,
> `docs/README_FEATURED.json`, `docs/README_EXCLUDED.json` ou a lógica do gerador.

## Formato

```jsonc
{
  "schema": "lucas-belucci-bellini/project-catalog@1",
  "note": "…",
  "counts": { "projects": 89, "public": 70, "private": 19, "with_live_website": 8 },
  "categories": ["AI & Intelligence", "Web & SaaS", "…"],
  "projects": [ /* ordenado por prioridade de exibição, depois por nome */ ]
}
```

### Campos de um projeto

| Campo | Significado |
|:---|:---|
| `repository` | `owner/repo` |
| `name` | nome curto, como aparece no README |
| `description` | descrição **pública** do repositório |
| `category` | taxonomia canônica fechada (ver abaixo) |
| `category_label` | o rótulo em português exibido no README |
| `status` | `🟢 Active`, `🟡 In Development`, `🔵 Experimental`, `🟣 Academic`, `⚪ Archived`, `🔒 Private` |
| `github` | URL do repositório |
| `github_visible` | sempre `true` — o código nunca some da apresentação |
| `github_role` | `primary` quando não há site; `source` quando há |
| `primary_cta` / `secondary_cta` | `website` \| `github` \| `null` — a regra central |
| `featured` | aparece em `docs/README_FEATURED.json` |
| `marketing_priority` | 0–100, só ordenação (ver [`PORTFOLIO-MARKETING.md`](PORTFOLIO-MARKETING.md)) |
| `private` | visibilidade no GitHub |
| `website` | **só existe quando a URL foi verificada.** É o link que o README publica |
| `website_declared` | a URL descoberta, no ar ou não |
| `website_status` | `verified` \| `unreachable` \| `invalid` |
| `website_http_status` | código HTTP da última verificação |
| `website_source` | `github_homepage` \| `manifest` |
| `website_final_url` | destino do redirecionamento, **só quando difere** da declarada |

Um projeto privado não tem `website` nem `website_declared`: a descoberta não roda
para ele.

## Taxonomia

Lista fechada, em `CATEGORIES`:
`AI & Intelligence`, `Web & SaaS`, `Games`, `Infrastructure`, `Automation`,
`Education`, `Productivity`, `Research`, `Security`, `Hardware & Simulation`,
`Software & Tools`, `Experimental`.

O README continua exibindo os rótulos em português que o perfil já usava;
`CATEGORY_ALIASES` é o único lugar onde as duas listas se encontram. Um rótulo
desconhecido cai em `Experimental` em vez de criar categoria nova — a taxonomia
só cresce por edição deliberada de `CATEGORIES`.

## Ordenação e idempotência

Ordenado por `marketing_priority` decrescente e, em empate, por nome — **nunca por
data**. Duas execuções sobre os mesmos dados produzem exatamente os mesmos bytes,
e `write_catalog_if_changed()` só grava quando o conteúdo muda. Não há carimbo de
tempo no arquivo; o motivo está em
[`WEBSITE-VERIFICATION.md`](WEBSITE-VERIFICATION.md).

## Privacidade

O catálogo publica **metadados**: nome, descrição pública, visibilidade, status,
categoria e links. Nunca conteúdo de arquivo, caminho interno, estrutura de
diretórios, token, secret ou `.env` — nem de repositório público, nem de privado.

## Validação

```bash
python3 scripts/validate_project_links.py
```

Falha se um site aparecer sem estar verificado, se um repositório privado
declarar site, se a ordem dos CTAs estiver invertida no README, ou se `website` e
`primary_cta` discordarem.
