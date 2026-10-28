# Checklist de integração — Portfólio V2 / Parte 2

Esta lista é o gate para integrar as mudanças em uma única base. Marque itens somente após executar a verificação no checkout integrado atualizado.

## 1. Preparação da base

- [ ] Confirmar que `feat/profile-v2-consolidated` é a base aprovada da rodada.
- [ ] Atualizar a branch candidata de integração sem alterar `main`.
- [ ] Comparar cada PR/branch com a base candidata e registrar o SHA testado.
- [ ] Resolver a divergência de `feat/v2p2-architecture` antes de aproveitar seus arquivos.
- [ ] Garantir que cada arquivo compartilhado tenha uma única versão final revisada.

## 2. Contrato do catálogo

- [ ] Fonte editorial e gerador estão documentados sem contradição.
- [ ] `docs/project-catalog.json` valida o schema `lucas-belucci-bellini/project-catalog@1`.
- [ ] Projetos privados e não visíveis são excluídos.
- [ ] O JSON servido usa `lucas-belucci-bellini/public-portfolio@1`.
- [ ] O payload contém somente `schema`, `project_count`, `projects`; cada projeto contém somente os campos públicos permitidos.
- [ ] Website é publicado apenas se verificado e seguro.
- [ ] A interface consome `data/projects.json` relativo à raiz publicada, não o catálogo editorial.

## 3. Testes e build

Executar na raiz do checkout integrado:

```bash
python -m py_compile scripts/build_portfolio.py
python -m json.tool docs/project-catalog.json > /dev/null
python -m unittest discover -s tests -v
python scripts/build_portfolio.py
```

- [ ] Registrar saída e código de saída dos quatro comandos.
- [ ] Confirmar que a suíte inclui `test_portfolio_contract.py` além dos testes do builder.
- [ ] Confirmar que build repetido é determinístico.
- [ ] Confirmar que saída antiga não deixa arquivos extras em `dist/`.
- [ ] Confirmar falha fechada para catálogo inválido/vazio e campos de visibilidade ausentes.
- [ ] Inspecionar o conteúdo do JSON gerado, sem imprimir nem anexar segredos.

## 4. Allowlist e privacidade de `dist/`

A saída deve conter exatamente:

```text
.nojekyll
index.html
assets/favicon.svg
assets/site.css
assets/site.js
data/projects.json
```

- [ ] Comparar a lista real de arquivos com a lista exata acima.
- [ ] Confirmar ausência de `docs/project-catalog.json`, manifests, arquivos de ambiente, tokens, credenciais, backups e metadados internos.
- [ ] Confirmar ausência dos campos `private`, `github_visible`, `repository`, `marketing_priority`, `website_status` e outros campos internos no JSON público.
- [ ] Confirmar URLs HTTPS seguras e sem credenciais.
- [ ] Servir `dist/` localmente e testar busca, filtros, ordenação, estados de erro/vazio e links externos.

## 5. Workflows e publicação

- [ ] Enumerar todos os arquivos em `.github/workflows/` no commit integrado.
- [ ] Confirmar que PRs executam validação/testes e não fazem deploy.
- [ ] Confirmar que apenas os eventos/ref de produção autorizados podem publicar Pages.
- [ ] Confirmar permissões mínimas e ausência de secrets nos logs/artefatos.
- [ ] Confirmar que a suíte completa roda no workflow canônico.
- [ ] Verificar os checks do SHA exato da branch integrada; não reutilizar checks de branches antigas.
- [ ] Verificar execução de deploy e URL real antes de declarar publicação concluída.

## 6. Aprovação

- [ ] Todos os itens críticos e altos concluídos.
- [ ] Nenhum conflito de merge pendente.
- [ ] Relatório final atualizado com SHA, comandos, resultados, links dos checks e riscos residuais.
- [ ] PR pronta para revisão humana.
- [ ] Nenhum merge em `main` realizado por esta tarefa.

## Estado inicial

Em 2026-10-28, a comparação remota identificou sobreposição entre builder, testes, frontend e workflow. Os testes integrados, o build de checkout combinado e a inspeção de `dist/` ainda não foram executados nesta sessão. Todos os itens acima permanecem desmarcados até haver evidência verificável.
