# Portfolio V2 — Relatório de integração e validação

**Branch:** `feat/v3p3-integration-candidate`  
**Base:** `feat/profile-v2-consolidated`  
**PR:** #78 (Draft)  
**Última execução automatizada verde confirmada:** GitHub Actions run #59 (commit `98f876821637839536611daa59e90ef56c393fa4`). A correção do teste passou. Mudanças posteriores para alinhar a ordem do filtro de status à prioridade semântica foram feitas nos commits `a101288b0e7ddf26648f144e3775146c038ce168` e `94bc95ac9bd005f3f318e08cef9a4056ca70f680`; essas mudanças aguardam CI.

## Escopo integrado

- Builder de publicação endurecido: saída restrita a `dist/`, rejeição de caminhos de entrada/saída inseguros e validação de URLs.
- Catálogo público gerado a partir da fonte canônica; a interface não mantém uma segunda lista de projetos.
- Busca textual combinável com categoria, status e ordenação.
- Dados editoriais inseridos via APIs DOM/texto, sem usar `innerHTML`.
- Links GitHub limitados a URLs de repositório válidas; o link de site é exibido apenas quando o builder o inclui após a verificação.
- Workflow executa a suíte Python, testes Node, build e verificação de allowlist/contrato antes de considerar o artefato.
- Melhorias de acessibilidade: rótulos associados, instrução da busca, contagem com live region, região de resultados, foco de teclado visível, suporte a forced-colors e anúncio dos estados vazio/erro.

## Evidência da última execução verde (#59)

- Python: 118 testes passaram.
- Node: 8 testes passaram, 0 falhas, incluindo busca combinada com categoria/status e ordenação A–Z, Z–A e status.
- Build do site estático: passou.
- Allowlist do artefato: passou.
- Contrato de privacidade: 78 projetos públicos e nenhum campo fora da allowlist.
- Artefato de inspeção: `portfolio-static-site`, 15.367 bytes (conforme o resumo do run #54).
- Job de deploy de produção: ignorado para a branch de feature.

## Cobertura adicional do catálogo

- A suíte frontend agora verifica filtros combinados (texto + categoria + status) e os modos de ordenação explícitos, além de estados de erro, links seguros e semântica acessível.
- A run #57 detectou uma expectativa de teste incompatível com a nova prioridade de status; a expectativa foi corrigida e a run #59 terminou com sucesso.
- A lista do filtro de status agora também deve seguir a prioridade semântica, evitando divergência entre as opções de filtro e a ordenação de resultados. O teste dessa consistência foi adicionado e aguarda CI.

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
