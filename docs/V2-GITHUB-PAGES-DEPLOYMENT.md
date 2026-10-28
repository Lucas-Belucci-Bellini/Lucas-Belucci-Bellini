# V2 — GitHub Actions, Pages e deploy

## Estado desta implementação

- O site estático usa `index.html`, `assets/site.css`, `assets/site.js` e `assets/favicon.svg`.
- O catálogo de origem é `docs/project-catalog.json`.
- `scripts/build_portfolio.py` gera o diretório `dist/` com uma lista explícita de arquivos públicos.
- O workflow `.github/workflows/portfolio-build.yml` valida os arquivos, compila o script Python, valida o JSON, executa o build, confere a lista permitida e armazena o artefato por sete dias.
- Este workflow **não publica** o site e não requer secrets.
- GitHub Pages ainda não é considerado configurado. Nenhuma URL de produção é presumida ou declarada como verificada.

## Execução e verificações

O workflow é acionado em pull requests para `feat/profile-readme-website-v2`, em pushes nessa branch quando arquivos relevantes mudam e manualmente por `workflow_dispatch`. A concorrência cancela uma execução anterior do mesmo grupo.

O job tem somente `contents: read`. O artefato contém exclusivamente:

- `index.html`
- `assets/site.css`
- `assets/site.js`
- `assets/favicon.svg`
- `data/projects.json`
- `.nojekyll`

O job falha explicitamente se algum arquivo esperado estiver ausente, se arquivos extras entrarem no artefato, se o JSON for inválido ou se o catálogo público estiver vazio.

## Ativar GitHub Pages — etapa manual pendente

Antes de habilitar publicação automática:

1. Abra **Settings → Pages** no repositório e confirme que GitHub Pages está disponível e autorizado para este repositório.
2. Defina a fonte de publicação como **GitHub Actions**.
3. Confirme o domínio/URL desejado e se há um domínio personalizado ou arquivo DNS a preservar.
4. Escolha explicitamente qual branch aprovada poderá publicar em produção. Não use a branch de trabalho para deploy de produção.
5. Só então adicione um workflow de deploy com permissões mínimas `pages: write` e `id-token: write`, ambiente `github-pages`, upload do artefato produzido pelo build e publicação do mesmo artefato — sem checkout/publicação do repositório inteiro.
6. Execute o deploy manualmente, confira o resultado no resumo da execução e abra a URL indicada pelo próprio GitHub Pages para validar HTTP e conteúdo.

Não adicione tokens ou URLs de deploy ao código. A publicação deve falhar se Pages não estiver configurado; não contorne esse erro nem publique conteúdo alternativo sem revisão.

## Verificação de produção

Ainda não foi realizado deploy por este workflow. Após a configuração manual, registrar aqui a URL fornecida por GitHub Pages, o SHA do commit publicado, o resultado do job de deploy e o teste HTTP da URL. Até então, o estado correto é **build automatizado; publicação pendente**.
