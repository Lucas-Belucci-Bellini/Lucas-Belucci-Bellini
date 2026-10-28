# V2 Design System — Field Manual

## Escopo

Esta revisão refina a camada visual existente sem trocar a identidade Baluarte / Field Manual: fundo escuro, dourado, Manrope para interface e DM Mono para metadados. Não adiciona bibliotecas, não altera o catálogo nem muda a lógica de busca/filtros.

## Tokens canônicos

Os tokens agora vivem em um único bloco `:root` em `assets/site.css`:

- **Cores semânticas:** canvas, superfícies, bordas, texto, dourado, sucesso, aviso, erro e foco.
- **Compatibilidade:** aliases antigos (`--bg`, `--panel`, `--line`, `--text`, `--muted`, `--gold`, `--gold-light`, `--green`) continuam disponíveis para os seletores existentes.
- **Tipografia:** `--font-body`, `--font-code`, `--text-xs/sm/md`.
- **Espaçamento:** escala de `--space-1` a `--space-12`.
- **Forma e interação:** raios, sombra, anel de foco e durações de movimento.

## Responsividade

- **Acima de 850px:** cabeçalho horizontal e grade de projetos em três colunas.
- **Até 850px:** cabeçalho passa a duas linhas, a navegação pode rolar horizontalmente sem quebrar links, o hero empilha e a grade usa duas colunas.
- **Até 600px:** conteúdo em coluna única, filtros empilhados, botões e cartões podem quebrar linhas sem alargar a página.
- A área de rolagem das âncoras considera o cabeçalho sticky em telas médias/pequenas.

## Acessibilidade e movimento

- Mantém `:focus-visible` e adiciona feedback consistente para campos via `:focus-within`.
- Alvos principais de ação têm altura mínima de 44px.
- `prefers-reduced-motion: reduce` minimiza animações, transições e deslocamentos.
- `forced-colors: active` define bordas e foco legíveis no modo de alto contraste.
- Os textos secundários usam o token de contraste compartilhado `--color-text-subtle`.

## Contratos preservados

IDs consumidos pelo JavaScript permanecem intactos: `project-search`, `category-filter`, `status-filter`, `project-count`, `project-grid`, `catalog-message` e `current-year`. Os seletores de cartão e estados de catálogo também foram mantidos. Não foram alterados `assets/site.js`, `docs/project-catalog.json`, `scripts/update_profile.py`, `scripts/build_portfolio.py`, README ou workflows.

## Verificação

Execute na raiz do repositório:

```bash
python -m unittest discover -s tests -v
python -m py_compile scripts/build_portfolio.py
python scripts/build_portfolio.py --out /tmp/portfolio-v2-dist
```

Os testes estáticos deste PR verificam tokens únicos, breakpoints, foco, redução de movimento, forced-colors e os IDs que ligam HTML ao JavaScript. Eles não substituem uma inspeção visual em navegador.

### Checklist manual pendente

- [ ] 375px: navegação, filtros, cartões, rodapé e ausência de rolagem horizontal da página.
- [ ] 768px: cabeçalho, hero, grade em duas colunas e controles de filtro.
- [ ] 1440px: hierarquia, largura máxima, hero e grade em três colunas.
- [ ] Navegação por teclado e visibilidade do foco.
- [ ] Catálogo carregado, busca sem resultados e erro de carregamento.
- [ ] Preferência de movimento reduzido e modo forced-colors.
- [ ] Confirmar os checks do GitHub Actions.

Nenhuma captura de tela nem aprovação visual é declarada nesta revisão.
