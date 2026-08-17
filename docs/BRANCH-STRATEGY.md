# Branch Strategy — Ecosystem Monitor

## Objetivo

Manter `main` como linha estável do monitor horário e fazer mudanças estruturais em branches dedicadas antes de promovê-las.

## Regras

- `main` = produção do monitor horário.
- `monitor/*` = mudanças no mecanismo de observação, contadores, snapshots e tolerância a falhas.
- `backup/*` = cópias históricas; não são usadas pelo monitor como fonte de dados.
- `claude/*` = branches de trabalho de automações/agentes; não são tratadas como produção automaticamente.
- O monitor acompanha a `default_branch` de cada projeto, em vez de assumir `main` para todos.
- Não deletar branches antigas automaticamente: primeiro verificar se contêm trabalho ainda não integrado.

## Fluxo

```text
branch de trabalho
      ↓
validação
      ↓
PR / revisão
      ↓
main
      ↓
workflow horário
      ↓
snapshot do ecossistema
```

## Escala

A estratégia permite manter dezenas ou centenas de projetos sem transformar o monitor em uma cópia de cada repositório. O estado guarda apenas o último SHA observado, a branch acompanhada e os contadores agregados.

## Regra do contador

O contador próprio do ecossistema é separado de GitHub Contributions. Cada execução horária bem-sucedida acrescenta exatamente uma unidade de `monitor_commits`, correspondente ao snapshot publicado pela própria execução.
