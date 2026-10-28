# Portfolio V2 — Relatório de integração e validação

**Branch:** `feat/v3p3-integration-candidate`  
**Base:** `feat/profile-v2-consolidated`  
**PR:** #78 (Draft)  
**Última execução automatizada verde confirmada:** GitHub Actions run [#64](https://github.com/Lucas-Belucci-Bellini/Lucas-Belucci-Bellini/actions/runs/40679327711), concluída com sucesso em 28/10/2026 para o merge ref da PR #78 (`041259ab3d246a716e30d8554066e584740d89b4`), incluindo o commit da branch `5b0529329863e02874c15979cb5bfed773a7b8f2`. A run #62 encontrou um pressuposto incorreto no harness (ele não modela a opção estática `all`); a expectativa foi ajustada no commit `6a27c527977c75755e9a2d3ac437713829a84bc1` e validada pela run #64.

## Escopo integrado

- Builder de publicação endurecido: saída restrita a `dist/`, rejeição de caminhos de entrada/saída inseguros e validação de URLs.
- Catálogo público gerado a partir da fonte canônica; a interface não mantém uma segunda lista de projetos.
- Busca textual combinável com categoria, status e ordenação.
- Dados editoriais inseridos via APIs DOM/texto, sem usar `innerHTML`.
- Links GitHub limitados a URLs de repositório válidas; o link de site é exibido apenas quando o builder o inclui após a verificação.
- Workflow executa a suíte Python, testes Node, build e verificação de allowlist/contrato antes de considerar o artefato.
- Melhorias de acessibilidade: rótulos associados, instrução da busca, contagem com live region, região de resultados, foco de teclado visível, suporte a forced-colors e anúncio dos estados vazio/erro.

## Evidência da última execução verde (#64)

- Python: 118 testes passaram.
- Node: 8 testes passaram, 0 falhas, incluindo filtros combinados, ordenação e a prioridade semântica do filtro de status.
- Build do site estático: passou; 78 projetos públicos foram gerados.
- Allowlist do artefato: passou; o contrato público não contém campos fora da allowlist.
- Artefato de inspeção: `portfolio-static-site`, enviado pela run #64 (15.454 bytes no upload compactado).
- Job de deploy de produção: ignorado para a branch de feature, como esperado.

## Cobertura adicional do catálogo

- A suíte frontend agora verifica filtros combinados (texto + categoria + status) e os modos de ordenação explícitos, além de estados de erro, links seguros e semântica acessível.
- A run #57 detectou uma expectativa de teste incompatível com a nova prioridade de status; a expectativa foi corrigida e a run #59 terminou com sucesso.
- A lista do filtro de status agora também segue a prioridade semântica, evitando divergência entre as opções de filtro e a ordenação de resultados. A run #62 revelou que o harness não representa a opção estática `all` do HTML real; a expectativa foi ajustada para verificar somente as opções dinâmicas e a run #64 confirmou a correção.

## Limites conhecidos

- O status Vercel reporta `build-rate-limit`; é separado do workflow GitHub Actions e não foi resolvido neste trabalho.
- A run #64 exibiu avisos de depreciação de dependências de Actions/Node e `ResourceWarning` em testes existentes; não falharam o workflow, mas merecem revisão separada.
- Os testes automatizados não substituem uma revisão manual com leitor de tela, navegação exclusivamente por teclado, zoom/reflow e temas de alto contraste.
- Não declarar conformidade WCAG total apenas com base nos testes atuais.
- A ordenação por status segue uma ordem explícita e estável (Active, In Development, Experimental, Academic, Archived), com desempate alfabético por nome. A ordenação padrão permanece alfabética porque o payload público atual não expõe `featured` nem `marketing_priority`. Se o requisito editorial de destaque/prioridade for aprovado, atualizar em conjunto a allowlist do builder, a verificação do artefato, o contrato do payload e os testes antes de ordenar por esses metadados.

## Próximas etapas seguras

1. Fazer uma revisão manual de teclado e leitor de tela em um preview de branch.
2. Decidir explicitamente se os metadados editoriais devem entrar no contrato público antes de alterar a ordenação inicial.
3. Confirmar que o preview/site de produção esperado está acessível quando a limitação da Vercel for resolvida.
4. Manter a PR em Draft até concluir a revisão funcional.

Nenhum merge em `main`, aprovação de PR ou deploy de produção foi realizado como parte desta integração.
