# Parte 3 — Plano de execução e integração (3/20)

**Branch:** `feat/v3p3-implementation-plan`  
**Base solicitada:** `feat/profile-v2-consolidated`  
**Escopo:** planejar dependências e integração da evolução do portfólio; não implementar novamente funcionalidades do V2.

## 1. Estado observado e limites da auditoria

- A branch `feat/v3p3-implementation-plan` foi criada a partir de `feat/profile-v2-consolidated`.
- Na busca remota de branches, não havia branch com o prefixo `feat/v3p3` antes desta criação. Foram encontradas branches V2 separadas para arquitetura, landing/hero/layout, catálogo/projetos, README, testes, acessibilidade, build e deployment.
- PRs V2 relevantes encontrados abertos na consulta: #52 (testes), #53 (acessibilidade), #54 (catálogo/filtros), #55 (design system), #56 (documentação operacional), #57 (auditoria de integração), #58 (gates de deploy) e #59 (release gate). A consulta foi uma fotografia do momento; conferir estados/checks novamente antes de integrar.
- O arquivo `docs/portfolio-build.md` declara `docs/project-catalog.json` como artefato editorial gerado por `scripts/update_profile.py`, e `scripts/build_portfolio.py` como responsável por gerar `dist/`. O frontend consome `dist/data/projects.json` (publicado como `data/projects.json`), não o catálogo editorial bruto.
- O catálogo consultado declara schema `lucas-belucci-bellini/project-catalog@1`, 99 entradas, 78 públicas, 21 privadas e 12 com website verificado. São valores do snapshot consultado, não métricas garantidas para commits futuros.
- A leitura de vários documentos esperados em `feat/profile-v2-consolidated` retornou 404, enquanto `docs/portfolio-build.md`, `scripts/build_portfolio.py` e `docs/project-catalog.json` estavam disponíveis. Portanto, não assumir que todos os documentos descritos em PRs já estejam na base escolhida ou integrados.
- Não foi possível inspecionar o checkout local nem executar comandos de teste local via esta integração. A validação real do código combinado e dos checks atuais permanece pendente.

## 2. Princípios não negociáveis

1. **Uma fonte editorial:** atualizar os manifestos e o gerador existentes; nunca manter uma segunda lista manual de projetos.
2. **Uma fronteira pública:** o builder filtra dados privados/não visíveis e emite somente a allowlist do schema público. Nunca publicar o catálogo editorial bruto.
3. **Sites verificados apenas:** só anunciar CTA de website se o registro tiver estado de verificação explícito e URL HTTPS válida. Uma verificação HTTP é pontual, não garantia de disponibilidade futura.
4. **README de perfil curto:** o README especial aponta para o site e recursos principais; catálogo extenso, busca, filtros e detalhes pertencem ao site.
5. **Reutilização:** antes de cada mudança, comparar o branch de trabalho com sua base e com PRs existentes; adaptar ou reusar a implementação válida em vez de duplicar.
6. **Sem promoção automática:** nenhuma etapa pode fazer merge em `main`, aprovar PR ou publicar produção sem autorização explícita.
7. **Sem alegações sem evidência:** builds, testes, checks, websites e deployments só podem ser declarados como verificados com evidência atual associada ao SHA exato.

## 3. Mapa de dependências entre as 20 conversas

A numeração abaixo é uma sequência de trabalho recomendada para a Parte 3. Se os prompts já definidos para alguma conversa tiverem outro título, manter o escopo funcional e aplicar a mesma dependência; não criar mudanças duplicadas só para seguir a numeração.

| Conversa | Entrega proprietária | Depende de | Regra de integração |
|---|---|---|---|
| 1 | Diagnóstico de baseline | Nenhuma | Registrar SHA, branches, PRs, arquivos e gaps atuais; não editar produto. |
| 2 | Contrato funcional do portfólio | C1 | Fixar fonte canônica, schema público, comportamento de links e responsabilidades README/site. |
| 3 | Plano de execução e integração | C1–C2 | Este documento; define ordem, propriedade e gates. |
| 4 | Arquitetura e limites entre componentes | C1–C3 | Documentar arquitetura real; sem reescrever código já válido. |
| 5 | Catálogo canônico e proveniência | C2–C4 | Manter gerador/manifestos atuais; nenhuma lista paralela. |
| 6 | Builder, schema e allowlist pública | C2, C5 | Preservar schema compatível; testar privacidade e URLs antes de ampliar campos. |
| 7 | Frontend: busca, filtros, ordenação e estados | C2, C6 | Consumir somente JSON público gerado; reutilizar IDs e controles já implementados. |
| 8 | Página de detalhes / informação ampliada | C2, C5–C7 | Reutilizar modelo canônico; não duplicar metadados editoriais no HTML/JS. |
| 9 | Verificação e política de websites | C2, C5–C6 | Só publicar links com estado verificado; registrar data/método se o modelo existente suportar. |
| 10 | README de perfil compacto | C2, C5, C9 | README serve de entrada para o site; não replica catálogo. |
| 11 | Design system e responsividade | C4, C7–C8 | Reutilizar tokens/classes existentes; mudanças CSS isoladas sempre que possível. |
| 12 | Acessibilidade e navegação por teclado | C7–C8, C11 | Preservar semântica, foco, movimento reduzido e anúncios de status; validar controles reais. |
| 13 | Segurança e privacidade de conteúdo | C5–C9 | Auditar allowlist, escaping/textContent, links externos, campos internos e erros. |
| 14 | Testes de contrato e regressão | C5–C13 | Consolidar testes nos harnesses existentes; sem criar uma segunda suíte concorrente. |
| 15 | Build reproduzível e artefato | C6, C14 | Executar builder e validar conjunto exato de arquivos em `dist/`. |
| 16 | CI e gates de pull request | C14–C15 | Reusar workflow existente; PR valida sem publicar produção. |
| 17 | Documentação operacional | C4–C16 | Documentar comandos realmente existentes, riscos e recuperação. |
| 18 | Integração das branches e resolução de conflitos | C4–C17 | Integrar em ordem de dependência; rever cada diff após resolver conflitos. |
| 19 | Verificação de release e smoke test | C18 | Confirmar checks no SHA candidato, artefato, navegação e links; sem deploy não autorizado. |
| 20 | Revisão final e handoff | C19 | Relatar evidências, pendências e decisão go/no-go; merge/publicação continuam dependentes de autorização. |

## 4. Propriedade dos arquivos e prevenção de conflitos

| Área / arquivos | Proprietário primário recomendado | Concorrência permitida |
|---|---|---|
| `docs/README_SITES.json`, `docs/README_FEATURED.json`, `scripts/update_profile.py`, `docs/project-catalog.json` | C5 / dados editoriais | Evitar edição simultânea. O catálogo JSON gerado só muda junto com a atualização controlada dos manifestos/gerador. |
| `scripts/build_portfolio.py`, `docs/portfolio-build.md` | C6 / C15 | Não alterar builder em paralelo com schema sem combinar contrato e testes. |
| `index.html`, `assets/site.js` | C7 / C8 | Um proprietário por vez; C8 amplia comportamento somente após confirmar o contrato do catálogo. |
| `assets/site.css`, tokens e documentação de design | C11 | Pode avançar em paralelo com builder/catálogo; não alterar IDs ou contratos DOM sem coordenação. |
| `tests/**` | C14 | Novos testes podem ser preparados em paralelo, mas devem ser executados contra a integração completa. Não apagar testes existentes para “fazer passar”. |
| `.github/workflows/**` | C16 | Um único proprietário de workflow. Reutilizar o workflow de deploy existente; não adicionar workflow concorrente de Pages. |
| `README.md` e gerador de README | C10 | Não editar ao mesmo tempo que o gerador de perfil; preservar marcadores gerados e conteúdo fora do escopo. |
| `docs/ARCHITECTURE.md`, documentos de operação/contrato/auditoria | C4 / C17 / C18 | Documentação pode ser paralela se os arquivos forem distintos; reconciliar afirmações com o código efetivamente integrado. |

A propriedade é de escrita, não exclusividade de leitura. Qualquer etapa pode ler arquivos de outras áreas. Mudança transversal exige atualizar primeiro o contrato e informar os consumidores afetados.

## 5. Estratégia de branches e integração

1. Manter `feat/profile-v2-consolidated` como referência de integração da Parte 2 até uma nova auditoria confirmar outra base mais segura. Não assumir que seja igual a `main`.
2. Para cada etapa, verificar se a branch sugerida já existe. Se existir, comparar base/HEAD e diff antes de reutilizá-la; não recriar nem resetar uma branch existente. Se não existir, criá-la a partir da última base de integração validada, não automaticamente de `main`.
3. Cada PR deve ter escopo pequeno, lista de arquivos proprietários, dependências, comandos executados e SHA testado. Preferir branches focadas, não uma branch por microalteração sem valor.
4. Integrar em lotes: (A) contrato e documentação; (B) fonte canônica e builder; (C) frontend e detalhes; (D) README; (E) acessibilidade/segurança; (F) testes/build/CI; (G) auditoria final.
5. Antes de cada lote: atualizar referências remotas, verificar estado do PR e comparar a base com o HEAD. Depois de cada lote: executar testes pertinentes e rever o diff consolidado.
6. Conflitos nunca são resolvidos escolhendo automaticamente “ours” ou “theirs”. Determinar a intenção de cada mudança, preservar ambas quando compatíveis, rodar testes de regressão e registrar a resolução.
7. Não alterar arquivos fora do escopo de uma conversa para “aproveitar” o branch. Não fazer squash/merge/close de PRs automaticamente.
8. PRs V2 ainda abertos são candidatos a reuso, não prova de integração. Confirmar seu estado, base, checks e diff atual antes de depender deles. Em especial, reavaliar #52–#59 no momento da integração.
9. Não retargetar PRs nem atualizar refs de branches compartilhadas sem necessidade e sem revisão explícita.

## 6. Gates obrigatórios antes de integrar

### Gate A — contrato e privacidade
- Schema versionado compatível com consumidores.
- Projetos privados e não visíveis excluídos do artefato público.
- Campos do JSON público limitados à allowlist documentada.
- Websites omitidos quando não verificados ou quando a URL falha na política.
- Sem tokens, segredos, caminhos locais ou metadados internos no artefato.

### Gate B — funcionalidade e acessibilidade
- Busca, filtros de categoria/status e ordenação funcionam em combinação.
- Estados de carregamento, erro, catálogo vazio e zero resultados distinguíveis.
- Links externos seguros e navegação de teclado/foco visível.
- Layout verificado em mobile e desktop; respeita `prefers-reduced-motion`.
- Detalhes adicionais vêm da mesma fonte canônica.

### Gate C — automação
Executar, conforme ferramentas disponíveis e instruções atuais do repositório:

```bash
python -m py_compile scripts/update_profile.py scripts/build_portfolio.py
python -m json.tool docs/project-catalog.json >/dev/null
python -m unittest discover -s tests -v
node --test tests/test_portfolio_frontend.cjs
python scripts/build_portfolio.py
```

Se um script/teste não existir na branch integrada, não inventar um resultado: identificar o comando ausente, localizar o equivalente atual e atualizar este plano/documentação em revisão separada. Validar o JSON gerado e a allowlist exata de `dist/`; conferir que o catálogo bruto não está em `dist/`.

### Gate D — CI e release
- Checks concluídos no SHA exato do candidato à integração.
- Falha de workflow analisada por log/SHA; lista vazia de statuses não significa sucesso.
- Artefato de build inspecionado; testes estáticos não substituem navegador real.
- Smoke test de site, links, teclado e dimensões mobile/desktop documentado.
- Deploy só por processo autorizado e somente após aprovação explícita.

## 7. Critérios de go/no-go

**GO para próxima etapa:** contrato claro, diff limitado, testes pertinentes verdes no SHA atual, nenhuma exposição de dados e nenhuma regressão conhecida sem aceite explícito.

**NO-GO:** base divergente não entendida; PRs/checks não revalidados; website não verificado anunciado; catálogo bruto publicado; falha de teste sem explicação; conflitos resolvidos sem revisão; ou necessidade de merge/deploy sem autorização.

## 8. Pendências para a próxima conversa

1. Revalidar as branches/PRs e escolher a base efetiva após conferir o estado atual de #52–#59.
2. Ler o contrato funcional da Conversa 2 e reconciliar diferenças com os arquivos disponíveis na branch.
3. Auditar os prompts/títulos efetivos das conversas 4–20 e mapear cada um para a matriz acima sem criar sobreposição.
4. Executar a suíte e o build em checkout integrado; anexar resultados reais e SHA.
5. Confirmar que o site servido usa somente `dist/data/projects.json` e que o README especial é conciso.
6. Não marcar a Parte 3 como concluída até testes, validação visual e estado dos PRs serem comprovados.

---
**Status deste documento:** plano registrado na branch de trabalho. Não representa integração de PRs, execução local de testes, aprovação, merge ou publicação.
