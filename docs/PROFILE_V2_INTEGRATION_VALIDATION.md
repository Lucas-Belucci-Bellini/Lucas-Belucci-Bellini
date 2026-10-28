# Portfolio V2 — Relatório de integração e validação

**Branch:** `feat/v3p3-integration-candidate`  
**Base:** `feat/profile-v2-consolidated`  
**PR:** #78 (Draft)  
**Última execução automatizada confirmada:** GitHub Actions run #51

## Escopo integrado

- Builder de publicação endurecido: saída restrita a `dist/`, rejeição de caminhos de entrada/saída inseguros e validação de URLs.
- Catálogo público gerado a partir da fonte canônica; a interface não mantém uma segunda lista de projetos.
- Busca textual combinável com categoria, status e ordenação.
- Dados editoriais inseridos via APIs DOM/texto, sem usar `innerHTML`.
- Links GitHub limitados a URLs de repositório válidas; o link de site é exibido apenas quando o builder o inclui após a verificação.
- Workflow executa a suíte Python, testes Node, build e verificação de allowlist/contrato antes de considerar o artefato.
- Melhorias de acessibilidade: rótulos associados, instrução da busca, contagem com live region, região de resultados, foco de teclado visível, suporte a forced-colors e anúncio dos estados vazio/erro.

## Evidência do run #51

- Python: 118 testes passaram.
- Node: 7 testes passaram, 0 falhas.
- Build do site estático: passou.
- Allowlist do artefato: passou.
- Contrato de privacidade: 78 projetos públicos e nenhum campo fora da allowlist.
- Artefato de inspeção: `portfolio-static-site`, 15.367 bytes.
- Job de deploy de produção: ignorado para a branch de feature.

## Limites conhecidos

- O status Vercel reporta `build-rate-limit`; é separado do workflow GitHub Actions e não foi resolvido neste trabalho.
- Os testes automatizados não substituem uma revisão manual com leitor de tela, navegação exclusivamente por teclado, zoom/reflow e temas de alto contraste.
- Não declarar conformidade WCAG total apenas com base nos testes atuais.
- A ordenação padrão permanece alfabética porque o payload público atual não expõe `featured` nem `marketing_priority`. Se o requisito editorial de destaque/prioridade for aprovado, atualizar em conjunto a allowlist do builder, a verificação do artefato, o contrato do payload e os testes antes de ordenar por esses metadados.

## Próximas etapas seguras

1. Fazer uma revisão manual de teclado e leitor de tela em um preview de branch.
2. Decidir explicitamente se os metadados editoriais devem entrar no contrato público antes de alterar a ordenação inicial.
3. Confirmar que o preview/site de produção esperado está acessível quando a limitação da Vercel for resolvida.
4. Manter a PR em Draft até concluir a revisão funcional.

Nenhum merge em `main`, aprovação de PR ou deploy de produção foi realizado como parte desta integração.
