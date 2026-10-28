# Matriz de aceitação — Portfólio V2

**Regra de status:** “Aprovado” exige evidência verificável; “Reprovado” exige falha observada; “Bloqueado” significa que a evidência/ambiente necessário ainda não foi validado. A existência de um PR não prova que a mudança foi integrada nem que passou nos testes.

| Critério | Evidência exigida | Status inicial |
|---|---|---|
| README renderiza corretamente no GitHub | Revisão da renderização da branch integrada | Bloqueado |
| README e site usam dados coerentes | Contrato da fonte canônica + diff de geração | Bloqueado |
| Catálogo não expõe projetos privados | Inspeção dos dados de origem e de todos os artefatos públicos | Bloqueado |
| Allowlist de campos e validação de URLs | Testes automatizados e inspeção do gerador | Bloqueado |
| Build reproduzível a partir de árvore limpa | Comando real, log e artefato identificados por commit | Bloqueado |
| Testes automatizados passam | Execução de CI ou local com logs e código de saída | Bloqueado |
| HTML/JS/JSON/Python validados conforme stack real | Relatório dos validadores existentes | Bloqueado |
| Busca/filtros e estados loading/vazio/erro funcionam | Testes de comportamento + revisão manual | Bloqueado |
| Teclado, foco, skip link, contraste e reduced motion | Auditoria manual e/ou ferramenta de acessibilidade | Bloqueado |
| Responsividade móvel e desktop | Evidência de viewport/teste manual | Bloqueado |
| Segurança básica de DOM e links externos | Revisão do código e testes relevantes | Bloqueado |
| Links e assets funcionam | Verificação automática/manual com resultados | Bloqueado |
| SEO/metadados essenciais | Inspeção do HTML gerado | Bloqueado |
| Workflows sem duplicação/conflito e com permissões mínimas | Revisão de todos os workflows + logs de CI | Bloqueado |
| GitHub Pages/deploy configurado | Settings/execução bem-sucedida/URL oficial | Bloqueado |
| URL publicada verificada | Abrir URL configurada e testar fluxo principal | Bloqueado |
| Documentação de operação e troubleshooting | Revisão destes documentos na branch integrada | Bloqueado |
| PRs das nove frentes revisados e conflitos resolvidos | Lista de PRs, status e diff de integração | Bloqueado |
| Aprovação humana final antes de release | Aprovação explícita registrada | Bloqueado |

## Decisão de release

**Não liberar a V2 enquanto qualquer critério obrigatório estiver bloqueado ou reprovado.** Atualize cada status com a evidência, commit/execução e responsável. Não marque como aprovado apenas porque o código está presente ou o PR foi aberto.
