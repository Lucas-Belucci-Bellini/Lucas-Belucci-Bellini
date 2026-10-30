# V2 — GitHub Actions, Pages e deploy

## Pipeline

O workflow `.github/workflows/deploy-portfolio.yml` valida as fontes, compila o builder Python, executa os testes do contrato, gera `dist/` e compara os arquivos e campos publicados com allowlists exatas. O artefato é limitado a `index.html`, `assets/site.css`, `assets/site.js`, `assets/favicon.svg`, `data/projects.json`, `data/stack-inventory.json` e `.nojekyll`.

Pull requests geram e validam o build, mas nunca publicam produção. Apenas push ou execução manual na branch `main` pode preparar o artefato do GitHub Pages e executar o deploy. O build usa permissões mínimas e não requer secrets para filtrar e gerar o site.

## Antes do primeiro deploy

1. Abra Settings → Pages e confirme que a fonte está configurada como GitHub Actions.
2. Preserve qualquer domínio personalizado e as regras de proteção existentes.
3. Revise os checks do PR consolidado antes de mesclar.
4. Após merge em `main`, confira o resultado real do job de deploy e use a URL fornecida pelo GitHub Pages.
5. Abra a URL publicada e verifique resposta HTTP e conteúdo antes de anunciá-la no README.

Até que essa execução real seja bem-sucedida, o estado correto é **build automatizado; publicação não confirmada**. Não contorne falhas com deploy alternativo nem adicione tokens/URLs secretos ao repositório.
