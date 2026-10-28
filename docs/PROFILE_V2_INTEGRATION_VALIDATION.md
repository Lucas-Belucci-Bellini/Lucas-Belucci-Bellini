# Portfolio V2 — Relatório de integração e validação

**Branch:** `feat/v3p3-integration-candidate`  
**Base:** `feat/profile-v2-consolidated`  
**PR:** #78 (Draft)  
**Última execução automatizada confirmada:** GitHub Actions run #54 (commit `b8eb5b0f88c307d635935627e9f31db2711ba2b8`)

## Escopo integrado

- Builder de publicação endurecido: saída restrita a `dist/`, rejeição de caminhos de entrada/saída inseguros e validação de URLs.
- Catálogo público gerado a partir da fonte canônica; a interface não mantém uma segunda lista de projetos.
- Busca textual combinável com categoria, status e ordenação.
- Dados editoriais inseridos via APIs DOM/texto, sem usar `innerHTML`.
- Links GitHub limitados a URLs de repositório válidas; o link de site é exibido apenas quando o builder o inclui após a verificação.
- Workflow executa a suíte Python, testes Node, build e verificação de allowlist/contrato antes de considerar o artefato.
- Melhorias de acessibilidade: rótulos associados, instrução da busca, contagem com live region, região de resultados, foco de teclado visível, suporte a forced-colors e anúncio dos estados vazio/erro.

## Evidência do run #54

- Python: 118 testes passaram.
- Node: 8 testes passaram, 0 falhas, incluindo busca combinada com categoria/status e ordenação A–Z, Z–A e status.
- Build do site estático: passou.
- Allowlist do artefato: passou.
- Contrato de privacidade: 78 projetos públicos e nenhum campo fora da allowlist.
- Artefato de inspeção: `portfolio-static-site`, 15.367 bytes (conforme o resumo do run #54).
- Job de deploy de produção: ignorado para a branch de feature.

## Cobertura adicional do catálogo

- A suíte frontend agora verifica filtros combinados (texto + categoria + status) e os modos de ordenação explícitos, além de estados de erro, links seguros e semântica acessível.
- GitHub Actions runs #53 e #54 confirmaram novamente os testes e o build nos commits atualizados.

## Limites conhecidos

- O status Vercel reporta `build-rate-limit`; é separado do workflow GitHub Actions e não foi resolvido neste trabalho.
- Os testes automatizados não substituem uma revisão manual com leitor de tela, navegação exclusivamente por teclado, zoom/reflow e temas de alto contraste.
- Não declarar conformidade WCAG total apenas com base nos testes atuais.
- A ordenação por status segue uma ordem explícita e estável (Active, In Development, Experimental, Academic, Archived), com desempate alfabético por nome. A ordenação padrão permanece alfabética porque o payload público atual não expõe `featured` nem `marketing_priority`. Se o requisito editorial de destaque/prioridade for aprovado, atualizar em conjunto a allowlist do builder, a verificação do artefato, o contrato do payload e os testes antes de ordenar por esses metadados.

## Próximas etapas seguras

1. Fazer uma revisão manual de teclado e leitor de tela em um preview de branch.
2. Decidir explicitamente se os metadados editoriais devem entrar no contrato público antes de alterar a ordenação inicial.
3. Confirmar que o preview/site de produção esperado está acessível quando a limitação da Vercel for resolvida.
4. Manter a PR em Draft até concluir a revisão funcional.

Nenhum merge em `main`, aprovação de PR ou deploy de produção foi realizado como parte desta integração.
