# Portfolio website — V2

O site é a apresentação web do mesmo portfólio do README do perfil. A identidade visual mantém a base escura e dourada do Field Manual, enquanto o site oferece navegação, busca e filtros que não existem no Markdown do GitHub.

## Arquitetura

- `index.html`: estrutura semântica e conteúdo editorial; carrega `assets/main.js` como módulo ES.
- `assets/site.css`: identidade visual responsiva, foco visível e suporte a redução de movimento.
- `frontend/main.ts`: runtime TypeScript de busca, filtros, cartões, inventário de linguagens e ferramentas.
- `frontend/contracts.ts`: tipos e validadores de runtime dos feeds públicos.
- `tsconfig.json`: compilação estrita para módulos ES2022 em `frontend/build/`.
- `crates/profile-core/src/site_feed.rs`: geração Rust dos dois feeds públicos e cópia da allowlist de arquivos.
- `docs/project-catalog.json`: catálogo canônico gerado pelo processo de perfil.
- `docs/PROFILE-ARCHIVE.md` + `docs/README_STACK.json`: fontes públicas da matriz de linguagens e das ferramentas documentadas.
- `scripts/build_portfolio.py`: implementação Python mantida como referência independente para comparar os feeds byte a byte.
- `vercel.json`: executa o compilador TypeScript e `profile-core site build`, publicando somente `dist/`.
- `.github/workflows/deploy-portfolio.yml`: executa testes TypeScript, testes Rust, comparação Python/Rust e verificação da allowlist.

O comando Rust `profile-core site build` lê o snapshot já versionado. Ele não consulta o GitHub, não conecta ao PostgreSQL e não precisa de token. Isso mantém o build do site reproduzível e independente de credenciais.

## Privacidade

O deploy **não publica a raiz inteira do repositório**. O Rust monta `dist/` com uma lista explícita de arquivos: HTML, CSS, módulos JavaScript compilados, favicon e os dois feeds públicos. O catálogo de saída exclui projetos privados, remove campos internos e só publica websites HTTPS verificados. Nomes privados também são removidos das evidências de ferramentas.

O TypeScript valida o schema e rejeita campos inesperados antes de renderizar. A interface cria elementos DOM e usa `textContent`; não injeta HTML arbitrário vindo do JSON.

## Executar localmente

Requer Node.js 22+, Rust 1.94.1 e a toolchain TypeScript 5.9.3 via `npx`:

```bash
npx --yes --package typescript@5.9.3 tsc -p tsconfig.json
cargo run --locked --release -p profile-core -- site build --root . --out-dir dist --frontend-build frontend/build
python -m http.server 8000 --directory dist
```

Abra `http://localhost:8000`. Não abra o HTML diretamente via `file://`, pois o catálogo é carregado com `fetch`.

Para comparar o gerador Python de referência:

```bash
python scripts/build_portfolio.py --out /tmp/portfolio-python-dist
cmp /tmp/portfolio-python-dist/data/projects.json dist/data/projects.json
cmp /tmp/portfolio-python-dist/data/stack-inventory.json dist/data/stack-inventory.json
```

## Validação

```bash
npx --yes --package typescript@5.9.3 tsc --noEmit -p tsconfig.json
npx --yes --package tsx@4.20.6 tsx --test frontend/contracts.test.ts
cargo fmt --all -- --check
cargo test --locked -p profile-core site_feed
```

O workflow também compara os feeds Python/Rust byte a byte e verifica a allowlist do artefato para garantir que nenhum dado privado foi incluído.

## Publicação

A hospedagem de produção é o projeto Vercel `lucas-belucci-bellini`, conectado ao repositório GitHub. O `vercel.json` define o build e o diretório `dist/`. GitHub Actions valida o mesmo artefato, mas não publica GitHub Pages nem altera domínios ou segredos do projeto.

## Relação com o README

O README continua sendo a landing page nativa do GitHub e deve direcionar ao site completo. Os blocos dinâmicos do arquivo de perfil permanecem sob controle dos geradores e seus marcadores; não edite manualmente os blocos gerados. O catálogo público mantém os contratos documentados em `docs/README_DATA.md`.
