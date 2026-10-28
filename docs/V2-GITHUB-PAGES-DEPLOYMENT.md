# V2 — GitHub Actions, Pages e deploy

## Estado desta implementação

- O site estático usa `index.html`, `assets/site.css`, `assets/site.js` e `assets/favicon.svg`.
- A fonte de metadados é `docs/project-catalog.json`; `scripts/build_portfolio.py` produz o artefato em `dist/`.
- O workflow único `.github/workflows/deploy-portfolio.yml` valida as fontes, compila o script Python, valida o JSON, executa o build e confere uma allowlist exata de arquivos e campos públicos.
- Pull requests para `main` e `feat/profile-readme-website-v2`, e pushes na branch de integração, fazem build e armazenam o artefato inspecionável por sete dias.
- Somente push ou execução manual na branch `main` prepara o artefato de Pages e inicia o deploy. Pull requests nunca publicam produção.
- O build/deploy não precisa de secrets nem publica o repositório inteiro. O job de build tem `contents: read`; somente o job de deploy recebe `pages: write` e `id-token: write`.
- GitHub Pages e a URL de produção ainda precisam ser confirmados na configuração/execução real. Nenhuma URL de produção é presumida.

## Conteúdo autorizado

O artefato final contém exatamente:

- `index.html`
- `assets/site.css`
- `assets/site.js`
- `assets/favicon.svg`
- `data/projects.json`
- `.nojekyll`

O validador compara o conjunto completo de arquivos e falha se houver qualquer arquivo ausente ou extra. O catálogo também é validado contra uma allowlist de campos públicos: nome, descrição, categoria, rótulo da categoria, status, URL do GitHub, visibilidade e metadados de verificação do site. A URL de um site só pode ser incluída quando `website_status` é `verified`; links publicados precisam usar HTTPS. Campos de controle interno, como `repository` e `private`, não são aceitos no artefato.

O workflow valida `project_count`, exige um catálogo público não vazio e usa apenas o diretório `dist/` para upload. Nenhum secret ou token é necessário para gerar o site estático.

## Execução

O workflow é executado automaticamente quando arquivos relevantes mudam em PRs para `main` ou `feat/profile-readme-website-v2), e em pushes nessas branches. Também aceita `workflow_dispatch`. Uma execução manual na branch de integração gera e armazena o artefato para inspeção, mas não publica produção. Para produção, a referência precisa ser `main`.

Para inspecionar o build, abra a execução do workflow e baixe o artefato `portfolio-static-site`. O diretório `dist/` é a única entrada autorizada para Pages.

## Configuração manual pendente do GitHub Pages

Antes do primeiro deploy:

1. Abra **Settings → Pages** no repositório e confirme que GitHub Pages está disponível e autorizado.
2. Configure a fonte de publicação como **GitHub Actions**.
3. Confirme a URL/domínio desejado e preserve qualquer domínio personalizado ou configuração DNS existente.
4. Revise as regras de proteção de `main` e escolha conscientemente quem pode publicar.
5. Execute o workflow na `main` após a revisão. O job de deploy deve publicar o artefato validado pelo build, usando o ambiente `github-pages`.
6. Confira o resumo da execução, o status do job e a URL fornecida pelo próprio GitHub Pages; abra essa URL e valide a resposta HTTP e o conteúdo antes de anunciá-la.

Não adicione tokens ou URLs de deploy ao código. Se Pages não estiver configurado ou a publicação falhar, registre o bloqueio e não contorne a falha com um deploy alternativo.

## Verificação de produção

Até uma execução real bem-sucedida, o estado é **build automatizado; publicação não confirmada**. Após o deploy, registrar a URL que o GitHub Pages fornecer, o SHA publicado, o resultado do job e o teste HTTP. Não declarar o site como publicado apenas porque o workflow existe ou o artefato foi gerado.
