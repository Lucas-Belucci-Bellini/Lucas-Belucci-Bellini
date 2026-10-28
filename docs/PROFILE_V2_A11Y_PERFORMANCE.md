# Acessibilidade, segurança do frontend e desempenho — V2P2

## Alterações nesta branch
- Reforço de foco visível com `:focus-visible` e espaçamento do foco para controles interativos.
- Respeito a `prefers-reduced-motion`, incluindo rolagem suave e transições/animações.
- Navegação horizontal utilizável em telas estreitas, sem forçar quebra dos links.
- Ajustes de layout em larguras de 720px e 420px para reduzir overflow e manter ações utilizáveis.
- Remoção do anúncio de região viva no contador de resultados: o número muda a cada digitação e filtros, então anunciá-lo em cada alteração seria repetitivo. O estado do catálogo continua sendo comunicado por `#catalog-message[role=status]`.
- Mantém criação de conteúdo por `textContent`/DOM APIs, URLs externas limitadas a HTTPS, links externos com `noopener noreferrer` e filtros/busca existentes.

## Validação automatizada
- Revisão estática das alterações e dos pontos de entrada existentes.
- Não foi possível executar build/testes no ambiente desta alteração via integração GitHub; não declarar como aprovados até o workflow da PR executar.
- Nenhuma nota Lighthouse, teste real com leitor de tela ou auditoria visual automatizada é reivindicada.

## Checklist manual antes do merge
- [ ] Abrir a página e navegar apenas com Tab, Shift+Tab, Enter, Espaço e setas nos selects.
- [ ] Confirmar foco visível em links, busca, filtros e ações dos cartões.
- [ ] Conferir ordem de headings e landmarks com árvore de acessibilidade do navegador.
- [ ] Testar leitor de tela: carregamento, erro, estado vazio e mudanças de busca/filtros; confirmar ausência de anúncios repetitivos.
- [ ] Testar zoom de 200% e 400%, orientação retrato/paisagem e larguras 320, 375, 720, 768 e 1440 px.
- [ ] Verificar contraste de texto, placeholders, status e foco usando ferramenta de contraste.
- [ ] Ativar redução de movimento no sistema e confirmar que não há animações/rolagem suave persistentes.
- [ ] Inspecionar links com URLs inválidas, HTTP, credenciais embutidas e conteúdo de catálogo com HTML/script literal.
- [ ] Executar workflow de build e testes da PR e revisar seus resultados reais.

## Riscos ainda não verificados
- Contraste quantitativo WCAG de todos os pares de cores; os tokens atuais não foram integralmente medidos.
- Compatibilidade com combinações reais de leitores de tela/navegadores.
- Visualização real em dispositivos físicos e larguras intermediárias.
- Auditoria Lighthouse/Core Web Vitals, tamanho de recursos e desempenho em rede lenta.
- O CSS existente pode conter regras de media query conflitantes; precisa de inspeção visual no navegador.
- Segurança de links externos depende também da validade e confiabilidade dos dados de origem; a validação atual não impede todos os destinos de phishing em HTTPS.
