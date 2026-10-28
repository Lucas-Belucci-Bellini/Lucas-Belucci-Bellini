# V2-01 — Auditoria de arquitetura e plano de integração

**Data da auditoria:** 2026-10-27  
**Base inspecionada:** `feat/profile-readme-website-v2`  
**Branch de trabalho:** `feat/profile-v2-architecture-audit`  
**Escopo:** inspeção remota dos arquivos versionados acessíveis; não representa execução local dos testes nem verificação do estado de deploy.

## 1. Resumo executivo

A branch-base já contém uma solução híbrida de perfil e portfólio: README gerado por Python, catálogo JSON compartilhado, site estático em HTML/CSS/JavaScript, artefato de publicação filtrado e um workspace Rust com crates separados por responsabilidade. Também existem workflows de validação, geração de perfil, monitoramento e deploy.

A integração da V2 deve **conectar e validar esses componentes existentes**, não introduzir um segundo catálogo ou substituir o gerador sem decisão explícita. O catálogo `docs/project-catalog.json` declara o schema `lucas-belucci-bellini/project-catalog@1`; o site produz uma projeção pública distinta (`lucas-belucci-bellini/public-portfolio@1`). Essa distinção deve continuar explícita.

### Limites da evidência

- Arquivos e metadados foram lidos na branch-base por meio da API do GitHub.
- Não foi feita clonagem local, execução de Python/Rust, build de site, execução de testes, inspeção dos logs de Actions ou requisição HTTP aos sites de projeto.
- Os números no catálogo e os estados de website são valores existentes no snapshot do catálogo, não uma nova verificação realizada nesta auditoria.
- O estado real do deploy e a configuração das GitHub Pages não foram confirmados.
- A auditoria limita-se a metadados públicos e arquivos versionados acessíveis; não consulta nem reproduz conteúdo privado.

## 2. Estrutura existente observada

### Perfil e catálogo

- `README.md`: perfil bilíngue/visual com seções de apresentação, categorias, produtos, sites, ecossistema e métricas. Contém marcadores de conteúdo gerado, incluindo `WHAT-I-BUILD:START/END` e `PRODUCT-CARDS:START/END`; os demais marcadores devem ser preservados pelo trabalho que editar o README.
- `scripts/update_profile.py`: gerador do README e do catálogo. Importa funções de `scripts/project_catalog.py`, usa `docs/README_SITES.json`, `docs/README_FEATURED.json`, `docs/README_STACK.json`, `docs/README_EXCLUDED.json` e grava `docs/project-catalog.json`. O docstring declara que o gerador usa metadados e mapas de bytes de linguagens, sem ler arquivos dos repositórios privados.
- `scripts/project_catalog.py`: núcleo Python de construção/normalização do catálogo, descoberta e verificação de websites, CTAs e apresentação.
- `docs/project-catalog.json`: catálogo produzido pelo gerador; não deve ser editado manualmente. O snapshot lido declara schema `lucas-belucci-bellini/project-catalog@1` e inclui `counts`, `categories` e `projects`. Cada projeto pode conter campos como `repository`, `name`, `slug`, `description`, `category`, `category_label`, `status`, `github`, `github_visible`, `github_role`, `primary_cta`, `secondary_cta`, `featured`, `marketing_priority`, `private`, `website_declared`, `website_status`, `website_http_status`, `website_source` e `website`. Nem todo campo é necessariamente obrigatório em todos os registros.
- Documentação relacionada existente inclui `docs/README_DATA.md`, `docs/PROJECT-CATALOG.md`, `docs/WEBSITE-DISCOVERY.md`, `docs/WEBSITE-VERIFICATION.md`, `docs/PORTFOLIO_WEBSITE.md`, `docs/PORTFOLIO-MARKETING.md` e `docs/V2-ROADMAP.md`.

### Site e artefato público

- `index.html`: entrada do site.
- `assets/site.css`, `assets/site.js`, `assets/favicon.svg`: assets referenciados pelo build do portfólio.
- `scripts/build_portfolio.py`: lê o catálogo, descarta entradas marcadas como privadas ou não visíveis e seleciona uma lista explícita de campos públicos. Gera `dist/index.html`, `dist/assets/*`, `dist/data/projects.json` e `dist/.nojekyll`. O script remove o diretório `dist/` antes de recriá-lo.
- O manifesto público tem schema `lucas-belucci-bellini/public-portfolio@1` e contém `project_count` e `projects`. A lista de campos projetados inclui `name`, `description`, `category`, `category_label`, `status`, `github`, `github_visible`, `website_status` e `website`; não inclui o campo interno `repository`.
- `.github/workflows/deploy-portfolio.yml`: valida o JSON, executa o build e verifica arquivos e fronteira de privacidade. O deploy descrito no workflow é condicionado a push em `main`; PRs executam build/validação, não publicação.
- `docs/V2-PRODUCTION-DEPLOYMENT.md` e `docs/PORTFOLIO_WEBSITE.md` documentam o modelo de publicação. Isso não confirma que a publicação esteja atualmente ativa.

### Rust, testes e automação

- `Cargo.toml` define um workspace Rust com crates `ecosystem-domain`, `store`, `site-monitor`, `github-client`, `catalog`, `profile-render` e `profile-core`. A configuração declara edition 2024 e rust-version 1.94.
- `tests/` contém, entre outros, testes para timeline de contribuições e monitoramento do ecossistema, além de fixtures, um GitHub falso e uma pasta `e2e`. A listagem consultada não foi suficiente para confirmar todos os casos ou resultados.
- `.github/workflows/v2-validation.yml` compila fontes Python, executa `python -m unittest discover -s tests -v` e chama validadores de badges/CTAs/identidade visual. O trigger observado está configurado para `main`, PRs direcionados a `main` e execução manual.
- Há workflows adicionais para `profile-core`, `core-shadow`, `rust`, `db-validation`, `ecosystem-watch`, `site-monitor-shadow`, `lang-stats`, `snake` e timeline de contribuições. A auditoria não executou esses workflows.

## 3. Arquitetura atual (observada)

```text
Metadados públicos do GitHub + manifestos editoriais
                     |
                     v
       scripts/project_catalog.py
                     |
          scripts/update_profile.py
              /             \
             v               v
      README.md        docs/project-catalog.json
                              |
                  +-----------+------------+
                  |                        |
                  v                        v
         Site estático               Consumidores/
         index.html + assets         validadores e docs
                  |
                  v
       scripts/build_portfolio.py
                  |
                  v
       dist/ (projeção pública filtrada)
                  |
                  v
       GitHub Pages (workflow condicionado a main)
```

Em paralelo, o workspace Rust organiza domínio, armazenamento, cliente GitHub, catálogo, monitor de sites e renderização/núcleo de perfil. A existência desses crates é verificada pela declaração do workspace; equivalência funcional completa com o gerador Python não foi provada nesta auditoria.

## 4. Arquitetura-alvo recomendada para a integração V2

1. **Uma única fonte canônica de catálogo:** manter `docs/project-catalog.json` como artefato gerado e contrato compartilhado, sem criar outro inventário concorrente.
2. **Projeções por consumidor:** README e site podem apresentar subconjuntos diferentes, mas derivados do mesmo catálogo e respeitando seus schemas/limites de privacidade.
3. **Geração controlada:** `scripts/update_profile.py` continua dono dos blocos gerados do README e do catálogo. Alterações editoriais devem evitar regiões geradas e preservar marcadores.
4. **Fronteira pública explícita:** o build do site deve continuar incluindo somente arquivos necessários e dados públicos allowlisted. Nunca copiar a raiz do repositório para o artefato.
5. **Paridade Rust/Python incremental:** onde o núcleo Rust for usado como sombra ou substituto, comparar saídas determinísticas e schemas por testes antes de alterar o caminho de produção.
6. **Validação na branch de integração:** os workflows relevantes devem executar para PRs direcionados a `feat/profile-readme-website-v2`, ou o checklist de integração deve executar equivalentes verificáveis manualmente. O estado atual observado dos workflows de validação/deploy está centrado em `main`; não foi confirmado um workflow dedicado a validar esta branch de integração.
7. **Deploy separado de validação:** sucesso de build/validação não equivale a publicação bem-sucedida. Só declarar deploy depois de verificar execução e URL resultante.

## 5. Contratos de dados compartilhados

| Contrato | Fonte/consumidor | Regras de integração |
|---|---|---|
| Catálogo canônico | `docs/project-catalog.json` | Preservar schema `lucas-belucci-bellini/project-catalog@1`; gerado pelo script; não editar manualmente. Validar JSON e campos consumidos. |
| Registro de projeto | `projects[]` do catálogo | `name` identifica o rótulo; `github` é URL de navegação; `private` e `github_visible` são gates de publicação; `website_status` deve representar estado produzido pela verificação/configuração existente, não uma promessa editorial. |
| Conteúdo editorial de README | `README.md` + manifestos `docs/README_*.json` | Preservar marcadores gerados e a propriedade do gerador. Alterar fontes de dados ou lógica, não o JSON de saída à mão. |
| Projeção do site | `dist/data/projects.json` | Schema separado `lucas-belucci-bellini/public-portfolio@1`; conter apenas campos allowlisted e projetos públicos/visíveis. Não incluir `repository` ou metadados internos desnecessários. |
| Estado de website | campos `website_declared`, `website_status`, `website_http_status`, `website_source`, `website` quando presentes | Distinguir URL declarada de estado verificado. Não reutilizar um status antigo como prova de disponibilidade atual. |
| CTA e links | `primary_cta`, `secondary_cta`, `github`, `website` | Links devem vir do catálogo; respeitar regra existente de site primeiro e código como alternativa quando aplicável. Não criar links/URLs sem fonte. |
| Privacidade | `private`, `github_visible` e allowlist do build | Nunca publicar conteúdo de repositórios privados; não assumir que um campo de metadados autoriza expor arquivos privados. Falhar de forma segura em entrada inválida. |
| Métricas | manifestos de dados e saídas geradas | Não codificar valores variáveis manualmente. Informar fonte, janela temporal e método de cálculo; não misturar contagem de projetos com contagem de sites. |

### Compatibilidade

Mudanças de schema devem ser explícitas: atualizar consumidores, documentação e testes no mesmo PR; evitar renomear/remover campos sem plano de migração. O schema de catálogo e o schema de saída do site são contratos diferentes e não devem ser tratados como intercambiáveis.

## 6. Dependências entre as dez frentes

Mapa lógico sugerido para coordenação (papéis inferidos a partir das frentes solicitadas, não declaração de implementação concluída):

1. **Arquitetura/auditoria (esta frente):** estabelece inventário, contratos, riscos e critérios de aceitação.
2. **Layout global:** depende dos requisitos de navegação e da estrutura de conteúdo; deve fornecer pontos de encaixe sem duplicar dados do catálogo.
3. **Hero/apresentação pessoal:** depende do layout e de fatos editoriais aprovados; não deve alterar geradores ou métricas.
4. **Projetos/repositórios:** depende do contrato do catálogo e da regra de privacidade; não deve manter uma lista concorrente hardcoded.
5. **Estatísticas/atividade GitHub:** depende de fontes de métricas documentadas e do pipeline existente; não pode inventar contagens nem substituir o gerador sem coordenação.
6. **Ecossistema/tecnologias:** depende de categorias e fontes de dados confirmadas; tecnologias devem ser sustentadas pelo repositório/metadados verificáveis.
7. **Experiência visual/responsividade/acessibilidade:** depende dos componentes/layout; deve preservar navegação por teclado, foco visível e redução de movimento existentes.
8. **Integração README:** depende das frentes editoriais e do contrato do gerador; preservar todos os marcadores e delegar conteúdo dinâmico a `scripts/update_profile.py`.
9. **Testes/qualidade:** depende dos contratos acordados; cobre schemas, links, marcadores, build público e não regressão.
10. **Produção/deploy:** depende do build e dos testes; valida configuração de Pages, artefato e URL publicado sem confundir PR de validação com deploy.

**Ordem recomendada:** 1 → contratos mínimos → 2–7 em paralelo com limites de arquivo claros → 8 → 9 → 10. O trabalho paralelo deve evitar editar os mesmos arquivos; alterações concorrentes em `README.md`, `docs/project-catalog.json` e scripts geradores exigem coordenação e revisão integrada.

## 7. Riscos e pontos a resolver

- **R1 — Workflow não alinhado à branch de integração:** os workflows de validação V2 e deploy consultados declaram `main` como branch de PR/push. Confirmar como a branch de integração receberá os mesmos checks.
- **R2 — Dois motores de geração:** Python gera perfil/catálogo e o workspace Rust declara crates equivalentes por domínio. Sem um teste de paridade confiável, os outputs podem divergir.
- **R3 — Catálogo com metadados sensíveis por desenho:** o catálogo pode registrar nomes e visibilidade de projetos privados; o build do site aplica filtro e allowlist, mas README e outras saídas devem continuar revisados para não expor informação que não se destina ao público.
- **R4 — Métricas e status temporais:** contagens e status HTTP podem ficar desatualizados. Não apresentar valores do snapshot como atuais sem execução do processo de atualização/verificação.
- **R5 — Destruição de artefato local:** `scripts/build_portfolio.py` remove `dist/` antes de gerar. Confirmar que `dist/` é descartável/gerado e não contém dados manuais antes de executar localmente.
- **R6 — Estado de deploy desconhecido:** a configuração de GitHub Pages e os últimos runs não foram inspecionados; a documentação do workflow descreve comportamento esperado, não o estado operacional atual.
- **R7 — Testes não executados nesta auditoria:** o inventário dos workflows não é evidência de que passaram.
- **R8 — Integração concorrente:** dez frentes podem causar conflitos em arquivos centrais e alterações de schema incompatíveis.

## 8. Checklist de integração verificável

### Catálogo e privacidade
- [ ] `python -m json.tool docs/project-catalog.json` termina com código zero.
- [ ] O schema canônico permanece `lucas-belucci-bellini/project-catalog@1`, salvo migração aprovada e coordenada.
- [ ] O catálogo é regenerado pelo fluxo oficial; não foi editado manualmente.
- [ ] A saída pública contém apenas projetos permitidos e campos allowlisted.
- [ ] Nenhuma entrada privada ou conteúdo de repositório privado é copiado para `dist/`.
- [ ] `dist/data/projects.json` usa o schema `lucas-belucci-bellini/public-portfolio@1`, e não o schema canônico por engano.

### README e gerador
- [ ] `scripts/update_profile.py` e `scripts/project_catalog.py` permanecem compatíveis.
- [ ] Todos os marcadores de conteúdo gerado existentes continuam presentes, balanceados e na ordem esperada.
- [ ] Uma regeneração do README não apaga conteúdo editorial manual fora das regiões geradas.
- [ ] Nenhuma métrica, tecnologia, status ou URL foi adicionada sem fonte verificável.
- [ ] Os validadores `validate_language_badges.py`, `validate_project_links.py` e `validate_restored_style.py` passam.

### Site
- [ ] `python scripts/build_portfolio.py` conclui com sucesso em ambiente de teste.
- [ ] `dist/index.html`, `dist/assets/site.css`, `dist/assets/site.js`, `dist/assets/favicon.svg` e `dist/data/projects.json` existem e não estão vazios quando aplicável.
- [ ] Busca, filtros, cartões, links e estados vazios são testados com catálogo real e fixtures.
- [ ] Acessibilidade: teclado, foco visível, semântica, contraste e redução de movimento são verificados.
- [ ] Links de projetos usam dados do catálogo e não URLs inventadas.

### Testes, Rust e automação
- [ ] `python -m py_compile .github/scripts/*.py scripts/*.py tests/*.py` passa.
- [ ] `python -m unittest discover -s tests -v` passa.
- [ ] `cargo fmt --all -- --check` e `cargo test --workspace` passam, se suportados pelo ambiente e crates atuais.
- [ ] Se o caminho Rust produzir os mesmos artefatos do Python, teste de paridade confirma saída determinística e schema compatível.
- [ ] Os checks necessários executam em PR para `feat/profile-readme-website-v2`, não apenas em PRs para `main`.

### Produção
- [ ] Build de produção usa somente o artefato filtrado `dist/`.
- [ ] Configuração de GitHub Pages permite o provider/origem esperado pelo workflow.
- [ ] O run de deploy é concluído com sucesso na branch de publicação configurada.
- [ ] A URL publicada é obtida do resultado real do deploy e recebe uma requisição HTTP de verificação.
- [ ] Falhas de deploy não são descritas como sucesso e não são mascaradas por badges estáticos.

## 9. Verificações executadas nesta tarefa

- [x] Leitura remota de `README.md`, `docs/project-catalog.json`, `scripts/update_profile.py` e `docs/PORTFOLIO_WEBSITE.md`.
- [x] Inspeção de listagens de raiz, `docs/`, `scripts/`, `tests/` e `.github/workflows/` na branch-base.
- [x] Leitura de `Cargo.toml`, `scripts/build_portfolio.py`, `.github/workflows/deploy-portfolio.yml` e `.github/workflows/v2-validation.yml`.
- [x] Confirmação de que a issue #28 descreve esta auditoria e indica a branch-base.
- [x] Criação da branch `feat/profile-v2-architecture-audit` a partir de `feat/profile-readme-website-v2`.
- [ ] Execução local de testes/builds: não executada.
- [ ] Inspeção de resultados de CI/Actions: não executada.
- [ ] Verificação HTTP do site/deploy: não executada.

## 10. Critério de conclusão

Esta entrega é um relatório de auditoria estática e plano de integração. A conclusão operacional depende de executar o checklist, obter os resultados reais de CI, revisar o PR e verificar o deploy separadamente. Nenhum merge em `main` faz parte desta tarefa.
