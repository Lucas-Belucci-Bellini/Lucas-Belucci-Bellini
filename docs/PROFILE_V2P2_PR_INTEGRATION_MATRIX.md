# Matriz de integração — Pull Requests V2P2

**Data da consulta:** 2026-10-28.  
**Base de comparação principal:** `feat/profile-v2-consolidated`.  
**Objetivo:** separar alterações de implementação, testes, documentação e deploy antes de montar uma candidata integrada.  
**Estado:** auditoria remota de diffs e status; nenhum teste foi executado localmente por esta matriz.

## 1. Decisão de integração

Não fazer merge em lote. Os PRs compartilham arquivos centrais e alguns status verdes disponíveis são apenas o contexto `Vercel`, não um relatório completo da suíte Python/Node ou da integração final. A integração deve ser montada numa branch temporária e cada grupo deve ser validado após a composição.

## 2. Comparação de PRs

| PR | Branch | Commits à frente da base declarada | Arquivos/diferenças observadas | Status consultado | Tratamento recomendado |
|---|---|---:|---|---|---|
| #50 | `feat/v2p2-architecture` → `main` | 5 | `scripts/build_portfolio.py`, `tests/test_build_portfolio.py`, `assets/site.js`, novo `docs/V2P2-ARCHITECTURE-AUDIT.md` | Nenhum status retornado para o SHA consultado | Revisão especial: base é `main`, portanto comparar o patch com o consolidado antes de aproveitar as alterações. |
| #51 | `feat/v2p2-build-security` | 6 | builder, testes e `docs/portfolio-build.md` | `Vercel: success` | Prioridade alta; revisar allowlist, validação de URL e proteção de destino de build. O status Vercel não prova que toda a suíte passou. |
| #52 | `feat/v2p2-test-suite` | 15 | workflow deploy, JS, documentação, testes de build, novos testes de contrato e frontend | `Vercel: success` | Prioridade alta após #51; preservar testes novos e validar se são realmente executados pelo workflow. |
| #53 | `feat/v2p2-a11y-performance` | 3 | CSS, HTML e novo relatório de acessibilidade/performance | Nenhum status retornado | Integrar após contrato funcional estável; revisar foco, reduced motion e responsividade manualmente. |
| #54 | `feat/v2p2-project-catalog` | 8 | JS, HTML, builder, testes e contrato do catálogo | Nenhum status retornado | Integrar após #51/#52; atenção à sobreposição em builder, JS e testes. |
| #55 | `feat/v2p2-design-system` | 6 | CSS, testes e documento do design system | Nenhum status retornado | Comparar tokens e estilos com #53; executar teste visual manual. |
| #56 | `feat/v2p2-documentation` | 5 | README e novos documentos de arquitetura/operações | Nenhum status retornado | Integrar depois de confirmar que os comandos e caminhos documentados existem. |
| #57 | `feat/v2p2-integration-audit` | 3 | dois documentos de auditoria/checklist | `Vercel: success` | Usar como evidência documental, não como prova automática de integração completa. |
| #58 | `feat/v2p2-deployment` | 5 | workflow GitHub Pages e documento de deploy | Nenhum status retornado | Revisar permissões, gatilhos e gates; não disparar deploy durante a integração. |
| #59 | `feat/v2p2-release` | 1 | novo gate de release | Nenhum status retornado | Manter como checklist; não marcar release aprovada antes dos testes combinados. |

**Importante:** contagem de commits e lista de arquivos são o resultado da comparação remota no momento da consulta. Não são uma ordem de merge nem prova de que as alterações são independentes.

## 3. Áreas de conflito previsíveis

1. **Builder e contratos de segurança:** #50, #51 e #54 alteram `scripts/build_portfolio.py`; #51 e #54 também alteram `tests/test_build_portfolio.py`. Fazer uma única revisão final do builder e dos testes, em vez de aceitar versões sucessivas sem comparação.
2. **Testes e workflow:** #52 adiciona testes de contrato/frontend e altera `.github/workflows/deploy-portfolio.yml`; #58 também altera o workflow. Confirmar que todos os testes pretendidos são executados antes de qualquer gate de deploy.
3. **Frontend:** #50/#52 mexem em `assets/site.js`; #53/#54/#55 afetam estilos, HTML ou comportamento do catálogo. Validar teclado, links externos, busca/filtros, estados vazios e mobile depois da composição.
4. **Documentação e gate:** #56/#57/#59 adicionam documentação. Consolidar sem duplicar instruções contraditórias e manter explícita a diferença entre testes por PR e testes da composição integrada.
5. **Base do #50:** este PR parte de `main`, enquanto os demais PRs V2P2 aqui listados partem de `feat/profile-v2-consolidated`. Não trocar sua base automaticamente nem mesclar diretamente.

## 4. Ordem recomendada para uma candidata isolada

- [ ] Confirmar novamente os SHAs de `main`, `feat/profile-v2-consolidated` e dos dez PRs antes de usar a matriz.
- [ ] Extrair o patch útil de #50 e compará-lo com o builder/testes já existentes; não aplicar o commit de consolidação inteiro.
- [ ] Integrar/revisar #51 (segurança) e #52 (testes) em uma branch de integração temporária.
- [ ] Adicionar #54 (catálogo), reconciliando builder, JS e testes.
- [ ] Adicionar #53 e #55 (acessibilidade/performance e design), com revisão conjunta de CSS/HTML/JS.
- [ ] Adicionar #56 (documentação), #57 (auditoria), #58 (workflow) e #59 (gate) em etapas separadas, sem executar publicação.
- [ ] Executar build limpo, suíte Python, testes Node/frontend, validação do JSON e revisão do conteúdo de `dist/`.
- [ ] Verificar que nenhum dado privado ou campo interno foi exportado e que sites não verificados são omitidos.
- [ ] Fazer revisão manual de teclado, foco, movimento reduzido e larguras móveis.
- [ ] Atualizar o relatório de auditoria com os resultados concretos e links dos runs.
- [ ] Pedir aprovação explícita antes de qualquer merge ou deploy.

## 5. Critérios mínimos de aceite

- O builder rejeita schema/campos inválidos, slugs duplicados e URLs inseguras.
- Projetos privados ou não visíveis não aparecem no payload público.
- Somente websites verificados e HTTPS são publicados como CTA de site.
- Build determinístico, destino de saída seguro e artefatos antigos removidos.
- Busca/filtros, detalhes de projeto e links têm estados acessíveis e comportamento consistente.
- CI executa realmente os testes definidos; um status de preview Vercel não substitui os checks.
- Deploy continua protegido e não foi acionado pela auditoria.

## 6. Limitações

A matriz foi produzida por comparação de refs e leitura de status remotos. Os checks consultados retornaram somente `Vercel: success` para #51, #52 e #57; para os demais SHAs a API de status não retornou checks. Isso não permite concluir que esses PRs falharam, mas também não permite afirmar que passaram. Nenhum teste combinado foi executado aqui.
