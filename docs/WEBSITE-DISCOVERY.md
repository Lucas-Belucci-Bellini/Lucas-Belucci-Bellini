# Descoberta de sites

Como o perfil decide **qual é o site de um projeto** — e, principalmente, quando
decide que não existe site nenhum.

Implementação: `discover_project_website()` em
[`scripts/project_catalog.py`](../scripts/project_catalog.py).

## Ordem de precedência

| # | Fonte | Por que vem nessa posição | `website_source` |
|:--|:---|:---|:---|
| 1 | Campo `homepage` do repositório no GitHub | É o próprio dono do repositório dizendo qual é o site. Ganha de tudo. | `github_homepage` |
| 2 | [`docs/README_SITES.json`](README_SITES.json) | Manifesto editorial, para quando a `homepage` está vazia. | `manifest` |
| 3 | Nada | O projeto passa a apresentar só o código. | `none` |

A origem escolhida fica gravada em `docs/project-catalog.json`, por projeto, para
que qualquer link publicado possa ser auditado até a fonte que o declarou.

## A regra que não tem exceção

**Nenhuma URL é deduzida do nome do repositório.**

`meu-projeto` não vira `meu-projeto.vercel.app` por dedução, nem por convenção,
nem porque "os outros seguem esse padrão". Um endereço inventado que por acaso
existe pertence a outra pessoa; um que não existe é um link quebrado anunciado
como vitrine. Os dois casos são piores do que exibir só o código.

Quem quiser registrar um site que o GitHub não conhece declara em
`docs/README_SITES.json` — é o degrau 2 justamente para isso.

## Formatos aceitos no manifesto

O formato histórico continua válido e não precisa ser migrado:

```json
{ "Lucas-Belucci-Bellini/Veritas": "https://veritas-opal-seven.vercel.app" }
```

O formato estendido aceita um objeto, para quando houver mais o que declarar:

```json
{ "Lucas-Belucci-Bellini/Veritas": { "website": "https://veritas-opal-seven.vercel.app" } }
```

`normalize_site_overrides()` lê os dois e descarta entradas sem `website`.

## Repositórios privados

Um repositório privado **nunca** declara site público, mesmo que tenha `homepage`
preenchida. A descoberta nem chega a rodar para ele: o laço em
`scripts/update_profile.py` pula repositórios privados antes de consultar a
`homepage`, e `resolve_presentation()` zera qualquer site que chegue até lá.

## Descobrir não é publicar

Descoberta responde "qual URL este projeto declara". Publicar exige que a URL
tenha respondido — ver [`WEBSITE-VERIFICATION.md`](WEBSITE-VERIFICATION.md).
As duas etapas são separadas de propósito: na auditoria que gerou este documento,
**8 das 16 URLs declaradas respondiam HTTP 404**.
