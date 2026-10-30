# Portfolio website — V2

O site é uma apresentação web do mesmo portfólio que o README do perfil. A identidade visual mantém a base escura e dourada do Field Manual, enquanto o site oferece navegação, busca e filtro que não existem no Markdown do GitHub.

## Arquitetura

- `index.html`: estrutura semântica e conteúdo editorial.
- `assets/site.css`: identidade visual responsiva, foco visível e suporte a redução de movimento.
- `assets/site.js`: busca, filtro por categoria e renderização segura dos cartões.
- `docs/project-catalog.json`: catálogo canônico gerado pelo processo atual do perfil.
- `docs/PROFILE-ARCHIVE.md` + `docs/README_STACK.json`: fontes públicas da matriz de linguagens e das ferramentas documentadas.
- `data/stack-inventory.json`: feed público gerado pelo builder, com participação por linguagem, categorias, evidências e links para projetos públicos correspondentes.
- `scripts/build_portfolio.py`: cria `dist/` com apenas os arquivos públicos necessários e uma projeção filtrada dos projetos públicos.
- `.github/workflows/deploy-portfolio.yml`: valida e constrói o artefato em pull requests e pushes; a publicação de produção é responsabilidade da integração Git do Vercel.

## Privacidade

O deploy **não publica a raiz inteira do repositório**. O build copia somente o HTML, os assets do site e os dois feeds públicos `dist/data/projects.json` e `dist/data/stack-inventory.json`. O catálogo de saída exclui projetos marcados como privados e inclui apenas os campos necessários para exibição pública. Não coloque tokens, segredos, endpoints internos ou conteúdo de arquivos privados nos campos públicos do catálogo.

## Executar localmente

Requer Python 3.10 ou superior:

```bash
python scripts/build_portfolio.py
python -m http.server 8000 --directory dist
```

Abra `http://localhost:8000`. Não abra o HTML diretamente via `file://`, pois o catálogo é carregado com `fetch`.

## Validação

```bash
python -m json.tool docs/project-catalog.json
python scripts/build_portfolio.py
```

O workflow também verifica se o artefato contém os arquivos necessários e se o catálogo publicado não contém registros privados.

## Publicação

A hospedagem de produção é o projeto Vercel `lucas-belucci-bellini`, conectado ao repositório GitHub. O workflow deste repositório faz build e validação e mantém um artefato inspecionável; ele não tenta publicar GitHub Pages. O domínio principal configurado no projeto é `lucas-belucci-bellini.vercel.app`.

## Relação com o README

O README continua sendo a landing page nativa do GitHub e deve direcionar ao site completo. As seções dinâmicas atuais permanecem sob controle de `scripts/update_profile.py` e seus marcadores; não edite manualmente os blocos gerados. O ajuste editorial do README será feito como parte da integração da V2, respeitando os contratos documentados em `docs/README_DATA.md`.
