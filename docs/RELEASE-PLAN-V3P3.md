# V3P3 — Plano de release e implantação

> Estado: **preparação documental; release não aprovada nem publicada**. Este documento não autoriza merge, promoção de deployment ou execução de workflow em produção.

## Referências verificadas

- Repositório: [Lucas-Belucci-Bellini/Lucas-Belucci-Bellini](https://github.com/Lucas-Belucci-Bellini/Lucas-Belucci-Bellini)
- Branch-base consultada: `main`.
- Branch desta preparação: `feat/v3p3-release-prep`.
- Workflow de build/deploy: [`.github/workflows/deploy-portfolio.yml`](../.github/workflows/deploy-portfolio.yml).
- Validação geral V2: [`.github/workflows/v2-validation.yml`](../.github/workflows/v2-validation.yml).
- Catálogo canônico usado pelo builder: [`docs/project-catalog.json`](./project-catalog.json).
- Builder do artefato: [`scripts/build_portfolio.py`](../scripts/build_portfolio.py).
- Instruções existentes de Pages: [`docs/V2-GITHUB-PAGES-DEPLOYMENT.md`](./V2-GITHUB-PAGES-DEPLOYMENT.md).

Os arquivos acima foram lidos na branch `main` antes desta alteração. O SHA do blob do workflow de deploy é `fe5a9d057e9c1c1b77d89031c19cec2fda977a72`; o SHA do blob do catálogo é `2e16f15bf83c2669b48452c72b0a56b92f8d7794`. **Esses são SHAs de blobs, não SHAs de commit.** O conector consultado não forneceu um SHA verificável do HEAD de `main`; por isso, não se declara aqui um commit de release.

## Escopo e critério de release

O site deve ser um catálogo público de projetos com busca, filtros e páginas/detalhes conforme a implementação consolidada. `docs/project-catalog.json` é a entrada canônica para o builder do site; não duplicar manualmente seu conteúdo no artefato. O builder produz uma lista pública com campos permitidos e exclui registros privados/não visíveis. Um site só pode ser anunciado como disponível depois de verificação real da URL e do conteúdo publicado; uma URL presente no catálogo não é, sozinha, prova de disponibilidade.

### Workflow e disparadores observados

O workflow `Portfolio website — build and deploy`:

- Em `pull_request` para `main`, `feat/profile-readme-website-v2` ou `feat/profile-v2-consolidated`, quando algum dos caminhos listados no YAML muda, valida o builder/catálogo, executa `tests/test_build_portfolio.py`, constrói o artefato público e verifica a allowlist.
- Em `push`, está configurado para `main` e `feat/profile-readme-website-v2` (com filtros de caminho). Um push em `feat/v3p3-release-prep` **não** corresponde a esse gatilho de push.
- Tem `workflow_dispatch`, mas o job de deploy e o upload do artefato de Pages têm condição explícita de `refs/heads/main` e evento `push` ou `workflow_dispatch`. **Executar manualmente na branch `main` pode publicar produção. Não fazer isso sem autorização explícita.**
- O workflow `V2 Validation` roda em PRs para `main`, pushes em `main` e execução manual, condicionado aos caminhos configurados.

O workflow de deploy gera um artefato estático com allowlist exata: `index.html`, `assets/site.css`, `assets/site.js`, `assets/favicon.svg`, `data/projects.json` e `.nojekyll`. O contrato rejeita campos adicionais no catálogo publicado.

## Sequência proposta (não executar em produção nesta etapa)

1. Abrir o PR da branch de implementação consolidada contra `main`; confirmar o diff e a lista de arquivos antes de integrar. Esta branch de documentação não substitui a auditoria funcional das outras etapas.
2. Confirmar que os checks exigidos passaram, incluindo `V2 Validation` e `Portfolio website — build and deploy` no contexto do PR, além de testes de acessibilidade, busca/filtros, navegação, segurança de links e responsividade já definidos pelas etapas anteriores.
3. Confirmar que a versão final do catálogo tem apenas dados públicos, slugs únicos, links HTTPS válidos e sites marcados como verificados somente com evidência recente.
4. Registrar o SHA completo do commit exato que foi aprovado para release, o resultado dos checks e a referência do artefato. Não usar o SHA desta branch documental como se fosse o commit da versão final do site.
5. Só após autorização explícita, integrar o PR. O push em `main` com os caminhos correspondentes dispara o workflow de build/deploy; alternativamente, o disparo manual em `main` também pode publicar.
6. Aguardar a conclusão de build e deploy, registrar a URL emitida pelo job e verificar a publicação conforme o checklist abaixo.

## Checklist de pós-deploy

- [ ] O workflow terminou com sucesso e o job de deploy concluiu (não basta o build ou upload do artefato).
- [ ] A URL de Pages veio do resultado real do deployment; não foi deduzida nem inventada.
- [ ] Página inicial carrega sem erro, layout quebrado ou recursos essenciais ausentes.
- [ ] Busca encontra projetos por nome/descrição e não quebra com texto sem correspondência.
- [ ] Filtros por categoria/status funcionam em conjunto com a busca e podem ser limpos.
- [ ] Cartões e detalhes mostram dados coerentes com `data/projects.json`, sem conteúdo privado ou campos internos.
- [ ] Links GitHub apontam para o repositório esperado; links de site abrem somente quando verificados e continuam acessíveis.
- [ ] Teste em viewport móvel e desktop; teclado/foco visível e rótulos acessíveis.
- [ ] Console e rede sem erros críticos; CSS, JavaScript, favicon e JSON retornam com sucesso.
- [ ] Nenhuma URL de preview é apresentada como produção. Atualizar README apenas após verificação real.
- [ ] Registrar SHA implantado, URL efetiva, horário, run do workflow e resultado de cada verificação.

## Rollback

O rollback é uma decisão operacional separada e exige autorização. Não há neste documento uma execução de rollback.

1. Identificar a última versão de produção comprovadamente saudável: SHA de commit e URL do deployment/run correspondente.
2. Se o defeito for apenas de conteúdo, corrigir a fonte canônica e validar novamente; não editar manualmente `dist/`, que é gerado.
3. Para regressão do código, preparar um PR de reversão do commit causador ou restaurar o commit saudável por um novo commit/revert. Não fazer force-push nem alterar `main` diretamente.
4. Após aprovação explícita e merge, acompanhar o workflow de produção e repetir o checklist pós-deploy.
5. Confirmar a recuperação na URL publicada e registrar causa, SHA revertido, SHA restaurado e evidências.

Não existe neste plano um SHA de rollback confirmado. A referência deve ser preenchida a partir do histórico e de um deployment saudável observado, nunca por suposição.

## Riscos e bloqueios conhecidos

- **Commit de release não fixado:** não foi obtido um SHA completo verificável do HEAD de `main` ou do candidato consolidado durante esta consulta.
- **Checks não confirmados:** a leitura do YAML confirma comandos configurados, não que tenham passado no último run.
- **Deployment não confirmado:** nenhum run de produção foi disparado ou verificado nesta tarefa.
- **Branch-base do site:** o workflow de PR inclui `main`, `feat/profile-readme-website-v2` e `feat/profile-v2-consolidated`, mas o gatilho de push não inclui `feat/profile-v2-consolidated`. Não confundir build de PR com publicação.
- **Dependência da configuração Pages:** `Settings → Pages` deve continuar apontando para GitHub Actions; isso não foi alterado nem validado por esta preparação.
- **Dados de sites envelhecem:** o status salvo no catálogo pode ficar desatualizado; verificar novamente antes de anunciar.
- **Mudanças paralelas:** existem várias branches `feat/v3p3-*`; a branch de release-prep não integra nem substitui essas implementações. Revisar a branch consolidada e seus conflitos antes de decidir qual commit publicar.

## Decisão pendente

**Não liberar ainda** até confirmar o SHA exato do candidato, os checks exigidos, o contrato público do artefato e a URL/resultado de um deployment autorizado. Esta documentação não promove, aprova ou dispara produção.
