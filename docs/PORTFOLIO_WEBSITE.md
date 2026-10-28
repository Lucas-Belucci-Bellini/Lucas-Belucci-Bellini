# Portfolio website — V2

O site é uma apresentação web do mesmo portfólio que o README do perfil. A identidade visual mantém a base escura e dourada do Field Manual, enquanto o site oferece navegação, busca e filtro que não existem no Markdown do GitHub.

## Arquitetura

- `index.html`: estrutura semântica e conteúdo editorial.
- `assets/site.css`: identidade visual responsiva, foco visível e suporte a redução de movimento.
- `assets/site.js`: busca, filtro por categoria e renderização segura dos cartões.
- `docs/project-catalog.json`: catálogo canônico gerado pelo processo atual do perfil.
- `scripts/build_portfolio.py`: cria `dist/` com apenas os arquivos públicos necessários e uma projeção filtrada dos projetos públicos.
- `.github/workflows/deploy-portfolio.yml`: valida e constrói o site em pull requests; publica no GitHub Pages em push para `main`.

## Privacidade

O deploy **não publica a raiz inteira do repositório**. O build copia somente o HTML, os três assets do site e `dist/data/projects.json`. O catálogo de saída exclui projetos marcados como privados e inclui apenas os campos necessários para exibição pública. Não coloque tokens, segredos, endpoints internos ou conteúdo de arquivos privados nos campos públicos do catálogo.

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

O workflow publica o artefato pelo GitHub Pages quando as alterações chegam à branch `main`. A configuração Pages do repositório deve permitir a origem **GitHub Actions**. A primeira publicação depende de essa configuração estar habilitada nas configurações do repositório.

## Relação com o README

O README continua sendo a landing page nativa do GitHub e deve direcionar ao site completo. As seções dinâmicas atuais permanecem sob controle de `scripts/update_profile.py` e seus marcadores; não edite manualmente os blocos gerados. O ajuste editorial do README será feito como parte da integração da V2, respeitando os contratos documentados em `docs/README_DATA.md`.
