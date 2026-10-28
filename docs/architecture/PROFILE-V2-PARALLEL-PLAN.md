# Plano de arquitetura e integração — Profile V2

**Status:** proposta inicial de coordenação; contratos sujeitos à validação do código e dos responsáveis pelas outras frentes.  
**Branch:** `feat/profile-v2-architecture`  
**Data:** 2026-10-27

## 1. Objetivo e limites

Evoluir a experiência pública do perfil/site com mudanças pequenas, rastreáveis e compatíveis com o sistema de publicação existente. Este documento coordena as dez frentes; não autoriza reescrita do workspace Rust, alteração do pipeline de publicação, escolha de framework sem evidência ou merge automático.

O README do perfil e o site são superfícies relacionadas, mas não devem se tornar duas fontes de verdade independentes. Dados públicos devem vir de fontes documentadas e verificáveis; nenhuma métrica, biografia, projeto, status ou URL pode ser inventado.

## 2. Arquitetura em camadas (conceitual)

1. **Fontes de dados** — catálogos/manifests e dados públicos de repositórios, respeitando os contratos existentes.
2. **Domínio e validação** — regras de visibilidade, classificação, links e apresentação; evitar duplicar a mesma regra em múltiplos consumidores.
3. **Entrega de dados** — artefatos gerados e/ou endpoints existentes, apenas conforme implementados e documentados; não presumir que uma API existe.
4. **Apresentação** — site público e README. Componentes de interface consomem dados por contratos explícitos e não consultam credenciais diretamente.
5. **Operação** — workflows, build, deploy, observabilidade e validações; publicar somente após gates verificáveis.

Diagrama de fluxo:

`fontes/manifests → domínio/catalogação → artefatos/API existente → apresentação (site/README) → validação e publicação`

O fluxo é um modelo de integração, não uma afirmação de que todos os endpoints já existam.

## 3. Organização de frontend — condicional à descoberta

Não criar ainda `src/`, `app/`, `pages/` nem adicionar dependências. Se for confirmado um frontend dedicado, a organização preferida por responsabilidade é:

- `pages/` ou equivalente existente: composição de páginas e rotas;
- `components/`: componentes reutilizáveis e de apresentação;
- `features/`: comportamento agrupado por domínio de interface;
- `data/`: adaptadores e tipos de leitura, sem segredos;
- `styles/` ou sistema de tokens existente: design tokens e estilos globais;
- `utils/`: funções puras e genéricas;
- `tests/`: testes unitários, integração e acessibilidade.

Usar os diretórios e convenções reais já existentes quando encontrados; esta lista não autoriza criar uma segunda estrutura concorrente.

## 4. Mapa de componentes e responsabilidades proposto

| Unidade lógica | Responsabilidade | Não deve fazer |
|---|---|---|
| Layout global | Estrutura, navegação, landmarks e responsividade | Buscar dados de GitHub diretamente |
| Hero/identidade | Apresentar identidade e CTAs confirmados | Inventar bio ou estatísticas |
| Navegação | Links internos e estado de navegação | Duplicar a lista de páginas |
| Catálogo de projetos | Mostrar projetos e links recebidos do contrato de dados | Manter catálogo paralelo manual sem sincronização |
| Project card | Apresentar um projeto tipado | Consultar rede ou inferir status |
| Skills/tecnologias | Exibir apenas informações documentadas | Transformar linguagens detectadas em expertise não comprovada |
| Activity/metrics | Renderizar métricas com origem e data conhecidas | Simular dados indisponíveis |
| Contato/social | Exibir destinos verificados | Construir URLs a partir de suposições |
| Estado de carregamento/erro/vazio | Comunicar falhas e ausência de dados | Ocultar falha como se fosse sucesso |
| SEO/metadados | Título, descrição, canonical e cards sociais | Publicar dados privados |
| Analytics, se aprovado | Medição mínima com política documentada | Coletar dados pessoais sem aprovação |

Os nomes são contratos conceituais, não nomes de arquivos obrigatórios.

## 5. Contratos de dados

Os componentes devem receber dados serializáveis e validados por adaptadores. O contrato final precisa ser ajustado ao schema real do catálogo/API existente.

### Projeto público (conceitual)

- `id`: identificador estável.
- `name`: nome público.
- `description`: descrição publicada e revisada.
- `repositoryUrl`: URL pública validada.
- `websiteUrl`: URL opcional, apenas se declarada e verificada conforme a regra atual.
- `status`: enumeração documentada; nunca inferida de uma falha transitória.
- `topics`: lista opcional de tags verificadas.
- `featuredOrder`: opcional, editorial e estável.

### Estado de carregamento

Modelar estados discriminados, por exemplo: `loading`, `success(data)`, `empty`, `error(retryable)`. Não misturar erro com lista vazia. A interface deve manter navegação útil mesmo quando métricas ou dados dinâmicos falham.

### Política de links

Validar esquema HTTPS quando aplicável, rejeitar URLs malformadas, usar rótulos acessíveis e não construir URLs arbitrárias a partir de entrada não confiável. Links externos devem ter comportamento de segurança consistente com a stack existente.

## 6. Convenções, qualidade e acessibilidade

- Seguir o formatter/linter da stack efetivamente encontrada; não introduzir convenções de TypeScript se não houver frontend TypeScript.
- Componentes e tipos devem usar nomes semânticos consistentes com o código existente.
- Funções de transformação de dados devem ser puras quando possível e testáveis sem rede.
- Tratar timeouts, respostas inválidas, rate limits e ausência de dados sem expor tokens ou payloads sensíveis.
- Testar dados válidos, campos ausentes, arrays vazios, links inválidos e erros de upstream.
- Usar HTML semântico, landmarks, hierarquia de títulos, foco visível, navegação por teclado, nomes acessíveis e texto alternativo útil.
- Respeitar preferência por movimento reduzido e contraste legível; não comunicar status apenas por cor.
- Verificar layout em mobile e desktop e evitar dependência de hover para ações essenciais.
- Não adicionar tracking ou analytics sem decisão explícita.

## 7. Arquivos compartilhados e propriedade

**Bloqueados para mudanças paralelas sem coordenação explícita:**

- `README.md`: conteúdo gerado entre marcadores; alterar gerador/fonte, não editar blocos manualmente.
- `docs/project-catalog.json` e manifests de `docs/README_*.json`: não editar o mesmo arquivo por duas frentes nem tratar dados gerados como fonte editorial.
- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`: mudanças de stack/workspace exigem justificativa e revisão da coordenação.
- `.github/workflows/**`: workflows compartilhados de CI/publicação; uma frente por arquivo, revisão obrigatória.
- `db/migrations/**`: nunca editar migration aplicada.
- arquivos globais de estilos, layout, router, tokens, config de build e deploy (se existirem): um único responsável de integração.
- `AGENTS.md`, `docs/DECISION-LOG.md` e documentos canônicos de arquitetura: somente a coordenação altera durante esta fase, salvo acordo expresso.

Antes de editar qualquer caminho, cada frente deve declarar os arquivos que possui e confirmar que não há sobreposição. Arquivos gerados não são lugar para correção manual.

## 8. Plano de trabalho paralelo: dez frentes

Esta divisão é uma proposta operacional; ajustar aos prompts das conversas 02–10 antes de iniciar implementações.

| Frente | Escopo | Entrega |
|---|---|---|
| 01 — Arquitetura (esta) | Auditoria, contratos, coordenação, decisões e riscos | Este documento e auditoria inicial |
| 02 — Descoberta de stack/deploy | Identificar site real, build, host, variáveis públicas e pipeline | Relatório com evidências, sem segredos |
| 03 — Design system | Tokens, tipografia, cores, espaçamento e responsividade | Especificação compatível com os estilos existentes |
| 04 — Layout e navegação | Estrutura de página, header/footer e rotas existentes | Layout integrado sem criar router duplicado |
| 05 — Hero e perfil | Identidade, apresentação e CTAs | Conteúdo verificável e componente isolado |
| 06 — Projetos e catálogo | Cartões/listas consumindo fonte canônica | Tipos/adaptador e estados de erro/vazio |
| 07 — Skills e atividade | Apresentação de tecnologias e métricas existentes | Componentes sem números inventados |
| 08 — Acessibilidade e responsividade | Auditoria de teclado, semântica, contraste e viewport | Correções e checklist reprodutível |
| 09 — SEO e performance | Metadados, assets, carregamento e otimização | Medições e melhorias com evidência |
| 10 — Testes e integração | Testes, revisão de conflitos, gates e smoke tests | Relatório de integração; sem merge unilateral |

As frentes 02–10 devem trabalhar em branches próprias, baseadas na mesma base acordada, e abrir PRs sem merge automático. A coordenação define a ordem de integração depois de receber os resultados da descoberta. Nenhuma frente deve presumir que sua branch inclui mudanças de outra.

## 9. Protocolo de integração

1. Registrar objetivo, base SHA, arquivos sob responsabilidade e critérios de aceite no início de cada frente.
2. Evitar commits cruzados em arquivos compartilhados; pedir coordenação se houver conflito de ownership.
3. Abrir PRs pequenos, sem merge automático, com resumo, testes executados e riscos.
4. Integrar primeiro contratos/fundações; depois componentes consumidores; por fim validações transversais.
5. Revalidar cada PR contra a base mais recente, sem alterar a branch principal diretamente.
6. Executar checks correspondentes à stack descoberta. Para o workspace Rust, seguir `AGENTS.md`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` e `cargo test --workspace`, além de testes específicos se os arquivos afetados exigirem.
7. Para alterações nos geradores Python com paridade Rust, executar os testes/fixtures documentados em `AGENTS.md`.
8. Fazer smoke test do site e verificar links, acessibilidade, responsividade e comportamento de falha.
9. Só declarar deploy após resultado explícito do provedor e verificação da URL publicada.
10. Não alterar modo de publicação ou guards de workflow sem autorização explícita.

## 10. Riscos e decisões pendentes

| Risco/decisão | Estado | Como resolver |
|---|---|---|
| Frontend real e stack | Pendente | Inspecionar repositórios, workflows e configuração de deploy |
| Vercel é destino efetivo? | Não confirmado neste documento | Confirmar projeto e configuração antes de adaptar |
| Fonte canônica de dados para o site | Pendente de leitura integral | Comparar catálogo/manifests/API existente |
| Rotas e páginas da V2 | Proposta | Validar com responsável de produto/design |
| Biblioteca visual ou CSS existente | Pendente | Não adicionar dependências até inventário |
| Métricas públicas permitidas | Requer validação | Usar apenas dados públicos comprovados e respeitar exclusões |
| Analytics e cookies | Não decidido | Não implementar até aprovação e revisão de privacidade |
| Domínio/canonical/SEO | Pendente | Confirmar domínio oficial e deployment |
| Compatibilidade com README gerado | Obrigatória | Testes de paridade e validadores existentes |
| Integração simultânea | Alto risco | Ownership de arquivos, PRs isolados e gates comuns |

## 11. Critérios de aceite da arquitetura

- Stack existente e fronteira do frontend identificadas com evidência.
- Fonte de verdade para projetos e métricas definida.
- Componentes desacoplados de rede e credenciais.
- Estados de loading, erro e vazio documentados.
- Nenhuma alteração em `main`; cada frente trabalha em branch.
- Ownership dos arquivos compartilhados registrado.
- Testes e verificações são relatados sem exagerar o que foi executado.
- Nenhum merge ou deploy é considerado concluído sem confirmação real.
