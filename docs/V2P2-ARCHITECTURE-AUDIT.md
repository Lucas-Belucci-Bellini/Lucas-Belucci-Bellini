# Parte 2 — Auditoria de arquitetura V2

**Branch:** `feat/v2p2-architecture`  
**Base auditada:** `main` após merge do PR #48  
**Escopo:** catálogo editorial, builder estático, frontend, testes e GitHub Actions.

## Estado observado

- O PR #48 está mesclado na `main`; o merge registrado foi `8990c178797a4bbef0a2061e88d882401bd6dc05`.
- Os workflows `V2 Validation` e `Portfolio website — build and deploy` associados ao commit `7640acf0a9f615de619a31058643f4e3712b457d` concluíram com sucesso.
- O catálogo editorial tem 99 entradas: 21 explicitamente privadas e 78 marcadas como públicas/visíveis. O builder precisa continuar tratando `private` como sinal prioritário de exclusão.
- O workflow valida o build em pull requests; as condições do job de deploy exigem `refs/heads/main` e evento `push` ou `workflow_dispatch`.

## Achados e correções desta branch

### P1 — Destino arbitrário de build podia apagar diretório externo

**Antes:** o builder rejeitava destinos dentro do repositório (exceto `dist/`), mas aceitava destinos externos. Se o destino já existisse como diretório, o código removia-o recursivamente antes de renomear a saída temporária. Uma chamada equivocada como `--out /tmp` poderia apagar conteúdo fora do projeto.

**Correção:** a saída agora precisa resolver exatamente para `<raiz-do-repositório>/dist`. Foi adicionado teste de regressão que confirma que um diretório externo e seu arquivo sentinela permanecem intactos.

### P2 — Validação de links GitHub permissiva demais

**Antes:** o builder validava HTTPS e o hostname GitHub, mas não restringia o caminho a `/<owner>/<repo>` nem rejeitava query/fragment. Isso permitia tratar como link de repositório URLs com caminhos adicionais.

**Correção:** `safe_github_url` exige HTTPS, host `github.com`/ `www.github.com`, exatamente dois segmentos de caminho, sem query/fragment e sem credenciais. Foram adicionados testes para URLs válidas e casos adversariais.

### P3 — Contrato de status do frontend

O catálogo público permite status arquivado, mas o conjunto de status renderizáveis no frontend não o incluía. O frontend agora aceita `⚪ Archived`, mantendo a mesma lógica de filtros.

## Contratos de arquitetura

1. **Fonte editorial:** `docs/project-catalog.json`, gerada pelo fluxo editorial existente. Não criar uma segunda lista manual.
2. **Privacidade:** projetos com `private: true` são excluídos antes da projeção pública; entradas sem booleano explícito `private` falham fechadas.
3. **Visibilidade:** só entram projetos explicitamente `github_visible: true`.
4. **Identidade pública:** cada projeto deve ter slug único, campos descritivos não vazios, categoria declarada e status permitido.
5. **URL principal:** apenas URLs HTTPS que apontem para exatamente um repositório em GitHub são aceitas.
6. **Website secundário:** só é publicado quando `website_status` é `verified` e o URL é HTTPS válido.
7. **Projeção pública:** somente `name`, `slug`, `description`, `category`, `category_label`, `status`, `github` e opcionalmente `website` podem ser serializados.
8. **Build:** o artefato é determinístico e limitado a seis arquivos: `index.html`, `assets/site.css`, `assets/site.js`, `assets/favicon.svg`, `data/projects.json` e `.nojekyll`.
9. **Frontend:** carrega `data/projects.json` e usa APIs DOM/textContent; não carrega o catálogo editorial bruto.
10. **CI/CD:** PRs validam e geram artefato inspecionável; só `main` com `push` ou execução manual pode preparar e executar deploy de produção.

## Dependências entre as próximas conversas

- **Conversa 2 (design system):** pode alterar CSS/HTML, mas deve preservar IDs e classes consumidos por `assets/site.js`.
- **Conversa 3 (catálogo):** deve respeitar o schema editorial e não duplicar a fonte de dados.
- **Conversa 4 (build):** deve preservar as allowlists e os testes de privacidade desta branch.
- **Conversa 5 (testes):** deve ampliar testes de contrato sem duplicar ou remover as regressões existentes.
- **Conversa 6 (acessibilidade/performance):** deve manter links seguros, anúncios acessíveis e os estados do catálogo.
- **Conversa 7 (deploy):** não pode ampliar os gatilhos de produção para branches ou pull requests.
- **Conversa 8 (documentação):** deve manter os comandos alinhados ao builder real.
- **Conversa 9 (integração):** precisa executar todos os testes na base combinada, não inferir sucesso a partir de branches isoladas.
- **Conversa 10 (release):** deve conferir os checks atuais e validar a URL de produção separadamente do build.

## Validação necessária antes de merge

- [ ] Workflow da branch desta auditoria concluído com sucesso.
- [ ] Testes de regressão do builder passam no GitHub Actions.
- [ ] Revisão visual no navegador e navegação por teclado executadas.
- [ ] Confirmar que a URL do GitHub Pages só seja anunciada após deploy real validado.

Este documento registra o estado conhecido no momento da auditoria; resultados futuros de CI ou da publicação devem ser atualizados com evidência observada.
