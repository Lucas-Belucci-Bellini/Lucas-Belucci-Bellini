# Arquitetura da aplicação do portfólio

Status: decisão de arquitetura e plano incremental. Este documento não declara
uma migração concluída nem altera o comportamento de produção.

## Objetivo

Manter o portfólio público rápido, estático e seguro, com:

- **Frontend em TypeScript:** interface, busca, filtros, acessibilidade e
  apresentação dos dados públicos.
- **Núcleo/backend em Rust:** regras de domínio, coleta autorizada, validação,
  geração dos feeds e integração com PostgreSQL quando configurado.
- **JSON como contrato:** o frontend consome somente artefatos públicos
  produzidos e filtrados pelo núcleo Rust.

A primeira entrega não exige uma API HTTP permanente. Um site de portfólio
predominantemente de leitura pode ser servido como arquivos estáticos; o Rust
continua sendo o backend de processamento e geração. Uma API HTTP só será
introduzida se surgir uma funcionalidade que realmente precise de operações
em tempo de execução.

## Estado atual verificado

| Camada | Estado atual | Próxima evolução |
|:---|:---|:---|
| Estrutura visual | `index.html` | Preservar semântica e acessibilidade |
| Estilos | `assets/site.css` | Preservar identidade visual e responsividade |
| Interações | `frontend/main.ts` + `frontend/contracts.ts`; módulos ES2022 compilados para `assets/main.js` e `assets/contracts.js` | Continuar modularizando sem alterar o contrato público |
| Feeds | `data/projects.json` e `data/stack-inventory.json`, gerados por `profile-core site build` | Manter schemas versionados e comparação com o builder Python |
| Build público | `tsconfig.json` + `profile-core site build`; `scripts/build_portfolio.py` permanece como oráculo de paridade | Manter a comparação byte a byte e a allowlist do artefato |
| Núcleo Rust | workspace com sete crates e CLI `profile-core`, incluindo o subcomando `site build` | Evoluir regras sem quebrar a paridade do catálogo |
| Persistência | crate `store` e migrations PostgreSQL | Usar apenas em rotinas que exigem histórico; site estático não depende do banco |
| Hospedagem | Vercel, com build command versionado em `vercel.json` | Validar a primeira implantação com o novo pipeline |

O runtime TypeScript e seus testes de privacidade foram integrados na PR #86.
A etapa seguinte compila os módulos ES2022 e troca o publicador de feeds para
Rust. O builder Python continua como referência independente para comparação.

**Limite importante:** o backend Rust do portfólio é um núcleo CLI de domínio,
coleta, validação e geração estática. Não existe uma API HTTP/Axum publicada;
a página é servida como arquivos estáticos e não depende de PostgreSQL.

## Contratos públicos

O frontend só pode receber os campos explicitamente permitidos:

- Projetos: `name`, `slug`, `description`, `category`,
  `category_label`, `status`, `github` e, quando verificado, `website`.
- Stack: resumo agregado, matriz de linguagens, ferramentas curadas,
  evidências públicas e referências a projetos públicos.

Regras obrigatórias:

1. Projetos privados nunca entram nos feeds públicos.
2. Não expor nomes de projetos privados, caminhos, código, variáveis,
   credenciais, tokens, endpoints internos ou metadados não permitidos.
3. URLs são validadas antes de renderizar links; o frontend usa criação de
   elementos e `textContent`, não HTML arbitrário vindo do JSON.
4. O snapshot de linguagens deve exibir sua data de atualização. Não chamar
   dados históricos de tempo real.
5. Mudanças de schema devem incrementar a versão do contrato e atualizar
   fixtures, validadores e consumidores na mesma PR.

## Plano de migração

### Etapa 1 — contratos e testes

- Formalizar tipos TypeScript para os dois feeds.
- Validar versão do schema, campos obrigatórios, arrays e URLs.
- Adicionar fixtures públicas e casos negativos para dados malformados.
- Manter o frontend atual funcionando enquanto a nova implementação é
  comparada.

Critério de saída: testes cobrem contratos válidos, inválidos e a exclusão de
projetos privados.

### Etapa 2 — frontend TypeScript

- Introduzir toolchain TypeScript com versões fixadas e lockfile.
- Migrar busca, filtros, cartões, inventário de linguagens e ferramentas.
- Ativar `strict`, `noUncheckedIndexedAccess` e `noEmitOnError`.
- Preservar IDs semânticos, navegação por teclado, foco visível e
  `prefers-reduced-motion`.
- Gerar assets estáticos versionados para o Vercel.

Critério de saída: build TypeScript reproduz a interface atual, testes passam
e nenhum código de runtime depende de executar TypeScript diretamente no
browser.

### Etapa 3 — Rust como gerador oficial

- Usar os crates existentes (`ecosystem-domain`, `github-client`,
  `catalog`, `profile-render`, `store` e `profile-core`).
- Migrar a geração dos feeds públicos para Rust, sem duplicar regra de
  privacidade no frontend.
- Executar Python e Rust em modo sombra e comparar saídas byte a byte.
- Preservar o Python como oráculo até cumprir os critérios de
  `docs/migration/PYTHON-TO-RUST.md`.

Critério de saída: paridade comprovada, fixtures atualizadas e workflow
validando o artefato Rust antes de qualquer troca do publicador.

### Etapa 4 — publicação e operação

- O Vercel publica somente o diretório de saída estático aprovado.
- GitHub Actions compila, testa e inspeciona a allowlist do artefato.
- Banco de dados não é requisito para servir a página pública.
- Rotinas que gravam no PostgreSQL precisam falhar claramente quando a
  configuração necessária estiver ausente; nunca simular sincronização bem
  sucedida.

## Política de integração

- Trabalhar em branches e PRs; nunca editar `main` diretamente.
- Não fazer merge se os checks obrigatórios estiverem pendentes ou falhando.
- Não mudar domínios, variáveis do Vercel, segredos ou configurações de
  produção como parte de uma migração de frontend.
- Não remover os geradores Python nem os testes de paridade antes do critério
  de saída documentado.
- Revisar o diff do feed público para garantir que nenhum dado privado entrou.

## Comandos de validação existentes

```bash
python -m py_compile scripts/build_portfolio.py
python -m json.tool docs/project-catalog.json >/dev/null
python -m json.tool docs/README_STACK.json >/dev/null
python scripts/validate_compact_readme.py
python -m unittest discover -s tests -p "test_build_portfolio.py" -v
python scripts/build_portfolio.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Os comandos Rust dependem da toolchain fixada em `rust-toolchain.toml`.
