# Desempenho e experiência móvel — V3P3

## Escopo e base auditada

- Branch de trabalho: `feat/v3p3-performance`
- Base: `feat/v3p3-site-experience` (`d88d70d76026f7213e0347d1d2a017ff4c850a09` no início da etapa)
- Fontes consultadas: `index.html`, `assets/site.css`, `assets/site.js`, `scripts/build_portfolio.py`, `tests/test_build_portfolio.py` e `docs/portfolio-build.md`.
- O site é estático: o builder publica os recursos de frontend e `data/projects.json`; não foi adicionada dependência.

## Mudanças nesta etapa

- `assets/site.js`: normaliza o texto pesquisável uma vez após carregar o catálogo, em vez de concatenar e converter o texto de cada projeto em todas as teclas digitadas. Os filtros combinados continuam usando os mesmos campos e controles.
- `assets/site.js`: agrupa eventos de busca/filtro que ocorram no mesmo frame com `requestAnimationFrame`, sem temporizador de debounce ou atraso intencional de digitação.
- `assets/site.css`: limita a navegação horizontal móvel à largura do cabeçalho e melhora o gesto de rolagem em dispositivos touch. O menu continua horizontal e rolável, sem criar uma dependência ou um menu JavaScript.
- Não foi alterado o catálogo canônico, o builder, o schema público, o README ou os workflows.

## Comparação de tamanho dos recursos

Tamanhos UTF-8 dos arquivos de origem recuperados pela API do GitHub antes e depois da edição:

| Recurso | Antes | Depois | Diferença |
|---|---:|---:|---:|
| `assets/site.js` | 6.614 B | 7.108 B | +494 B |
| `assets/site.css` | 17.445 B | 17.541 B | +96 B |

A alteração **não reduz o tamanho transferido** desses dois arquivos. O JavaScript cresceu para armazenar o texto normalizado por projeto e coordenar os renders; a expectativa é reduzir trabalho repetido durante a busca, mas essa melhoria de tempo de CPU ainda precisa ser medida em navegador. Os números acima são tamanho bruto de origem, não tamanho comprimido (gzip/Brotli).

## Responsividade e acessibilidade já existentes

O CSS auditado já possui breakpoints em 850 px e 600 px, grade de projetos em três/duas/uma coluna, filtros empilhados em telas estreitas, foco visível e suporte a `prefers-reduced-motion`/forced colors. Esta etapa preserva essas regras e acrescenta apenas contenção de rolagem horizontal no cabeçalho móvel.

## Validação e limitações

- Confirmado via GitHub que as alterações foram gravadas em `feat/v3p3-performance` e que o arquivo JS contém o cache de busca e o agendamento via `requestAnimationFrame`; o CSS contém as regras móveis adicionadas.
- A suíte existente `tests/test_build_portfolio.py` cobre o builder/artefato público, não o comportamento de interação do frontend. A árvore consultada não mostrou `tests/test_portfolio_frontend.cjs`.
- Não foi possível executar um checkout local, `node --check`, a suíte Python, o build, Lighthouse ou testes visuais nesta sessão através da integração GitHub. Não há medições antes/depois de LCP, INP, CLS, tempo de render, FPS ou rede.
- Não foi possível confirmar o layout em viewport real de 320/375/768/1024 px. A revisão CSS estática não substitui a inspeção visual e os testes de teclado/toque.
- O CSS já contém uma camada de estilos legados e uma camada posterior de tokens/design system; consolidá-las sem um diff visual e testes de regressão seria arriscado e ficou fora do escopo.

## Próximos passos de validação

1. Executar `python -m unittest discover -s tests -v` e `python scripts/build_portfolio.py` num checkout desta branch.
2. Executar verificação sintática JavaScript e criar teste DOM para busca/filtros/estado vazio, se o harness existente suportar.
3. Comparar em navegador, com o mesmo catálogo, digitação rápida e filtro combinado antes/depois; registrar tempo de interação e tamanho comprimido dos recursos.
4. Testar 320, 375, 600, 768, 850 e 1024 px, navegação por toque/teclado e movimento reduzido.
5. Não integrar nem publicar até checks atuais e revisão visual estarem concluídos.
