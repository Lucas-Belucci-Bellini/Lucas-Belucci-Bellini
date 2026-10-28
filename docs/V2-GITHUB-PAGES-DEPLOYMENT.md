# V2 — GitHub Actions, Pages e deploy

## Objetivo e workflow único de publicação

O único workflow que publica o site é `.github/workflows/deploy-portfolio.yml`. Os demais workflows do repositório podem validar outros componentes, mas não devem chamar `actions/deploy-pages` nem publicar o mesmo site.

O workflow do portfólio executa, em sequência:

1. Compila o builder Python e valida o JSON do catálogo e os arquivos-fonte.
2. Executa a suíte completa de testes em `tests/`.
3. Gera o site público em `dist/`.
4. Verifica a allowlist exata, rejeita links simbólicos e valida os campos/URLs do catálogo público.
5. Anexa um artefato de inspeção por sete dias.
6. Somente para `push` em `main` ou `workflow_dispatch` executado com `main` como ref, prepara o artefato do Pages e executa o job de deploy.

## Pull requests e produção

- Pull requests executam validação, testes e build; não têm etapa de upload de artefato do Pages nem job de deploy.
- Pushes em branches diferentes de `main` não publicam.
- A execução manual (`workflow_dispatch`) só publica se a ref selecionada for `main`; em outras refs ela pode validar, mas o job de deploy é ignorado.
- O job de build tem apenas `contents: read`. O job de deploy recebe somente `pages: write` e `id-token: write`, e depende do build concluído com sucesso.
- O pipeline não precisa de secrets personalizados para compilar ou publicar com o mecanismo OIDC oficial do GitHub Pages.

## Allowlist do artefato

O diretório `dist/` deve conter exatamente estes arquivos:

- `index.html`
- `assets/site.css`
- `assets/site.js`
- `assets/favicon.svg`
- `data/projects.json`
- `.nojekyll`

Arquivos extras, arquivos ausentes, links simbólicos, campos não públicos ou URLs inválidas fazem a validação falhar antes da publicação. O builder também deve substituir o diretório de saída por uma geração limpa e determinística.

## Configuração e permissões do GitHub Pages

1. Abra **Settings → Pages** e confira a fonte de publicação configurada como **GitHub Actions**. Não altere essa configuração automaticamente.
2. Preserve o domínio personalizado, DNS, regras de proteção e exigências de aprovação existentes. Este procedimento não autoriza removê-los nem enfraquecê-los.
3. Em **Settings → Actions → General**, confira se Actions está habilitado e se as políticas de ações permitidas aceitam as ações usadas pelo workflow.
4. O token padrão do workflow precisa das permissões declaradas no YAML. O job de deploy também precisa que o ambiente `github-pages` e o mecanismo OIDC/Pages estejam disponíveis. Não crie PAT ou secret extra como primeira tentativa.
5. Se o repositório usar aprovação obrigatória para o ambiente `github-pages`, mantenha-a e conclua a aprovação pelo fluxo normal.

## Como validar sem confundir build com publicação

- Em **Actions**, abra a execução do workflow `Portfolio website — build and deploy` e confira o evento, a branch, o SHA e cada job.
- Em um PR, o resultado esperado é build/testes concluídos e job de deploy ignorado/inexistente para produção.
- Um build verde **não confirma publicação**. Considere publicado somente quando a execução de produção na ref `main` mostrar o job `Deploy GitHub Pages production` como sucesso e o output da URL estiver presente.
- Após o deploy confirmado, acesse a URL reportada pelo próprio job e verifique a página. Não presuma domínio nem altere o README antes da confirmação.

## Recuperação de falhas

1. Identifique o primeiro step que falhou e leia os logs; não repita deploy manual nem contorne o teste.
2. Se falhar build/teste/allowlist, corrija a fonte ou teste na branch e abra/atualize o PR. Não publique um artefato incompleto.
3. Se o build passar e o deploy falhar, confira **Settings → Pages**, as permissões Actions, a configuração do ambiente `github-pages`, as aprovações pendentes e os logs do job. Preserve domínio e proteção.
4. Depois de corrigir, valide novamente pelos checks normais. Não faça merge nem deploy manual como parte desta auditoria.
5. Se uma publicação anterior foi afetada, prefira corrigir e publicar pelo fluxo autorizado de `main`; não apague artefatos/configuração ou mude DNS sem autorização.

## Estado auditado em 2026-10-28

Foram observados checks reais do GitHub Actions no repositório. A execução do workflow de portfólio em `main` concluiu o build e a validação do artefato, mas o job de deploy falhou com HTTP 404 ao criar a implantação; o log do GitHub recomenda conferir se Pages está habilitado. A API do repositório também retornou `has_pages: false`. Portanto, a publicação não está confirmada. Confira **Settings → Pages** e habilite/configure Pages somente após a decisão explícita do responsável; esta PR não muda essa configuração. Outra execução recente em uma branch de trabalho falhou nos testes de contrato antes de gerar `dist/`. Investigue os logs e os SHA dessas execuções individualmente, pois um resultado verde de outro commit não valida automaticamente este commit.
