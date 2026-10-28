# Relatório de auditoria integrada — Parte 2

Data da inspeção: 2026-10-28  
Branch de referência: `feat/profile-v2-consolidated`  
Branch de auditoria: `feat/v2p2-integration-audit`

## Escopo e limites

Inspeção remota de branches, comparações de commits, arquivos do builder, contrato do catálogo, testes e workflow. Esta auditoria não executou o código em um checkout integrado local. Portanto, não declara testes de runtime aprovados nem substitui os checks do CI sobre um commit de integração atualizado.

## Achados por severidade

### Crítico — branches não formam uma base de integração única

A comparação com `feat/profile-v2-consolidated` mostrou:
- `feat/v2p2-architecture`: divergiu, 7 commits à frente e 22 atrás; contém uma cópia inicial de vários arquivos-base e do workflow.
- `feat/v2p2-deployment` e `feat/v2p2-documentation`: idênticas à referência na comparação consultada (nenhuma alteração exclusiva detectada).
- As demais branches têm alterações sobrepostas em `scripts/build_portfolio.py`, `tests/test_build_portfolio.py`, `assets/site.css`, `assets/site.js`, `index.html` e/ou `.github/workflows/deploy-portfolio.yml`.

**Ação recomendada:** não integrar a branch de arquitetura diretamente sem rebase/merge planejado e comparação do resultado; reconstruir a suíte integrada em uma branch candidata única e atualizar os checks após cada integração.

### Alto — contrato de catálogo precisa ser verificado ponta a ponta

O contrato documentado define o payload público como `lucas-belucci-bellini/public-portfolio@1`, em `dist/data/projects.json`, com `schema`, `project_count` e `projects`. A lista de campos permitidos por projeto é `name`, `slug`, `description`, `category`, `category_label`, `status`, `github` e `website` opcional.

O builder de `feat/v2p2-project-catalog` usa allowlist de saída e omite website não verificado. A variante em `feat/v2p2-build-security` é mais estrita na validação da raiz, categorias e URLs. Como ambas alteram o mesmo arquivo, escolher uma versão por revisão de diff é obrigatório; não se deve presumir que a última versão contenha automaticamente todas as garantias desejadas.

**Ação recomendada:** usar uma única implementação do builder e executar os testes de contrato sobre o catálogo real após integrar frontend + builder + testes.

### Alto — workflow de CI e deploy difere entre branches

O workflow na branch `feat/v2p2-test-suite` executa descoberta ampla dos testes, enquanto a versão de `feat/v2p2-architecture` executa somente `test_build_portfolio.py`. Isso pode deixar o teste de contrato de frontend/catálogo de fora caso seja integrada a variante antiga.

Na versão inspecionada de `feat/v2p2-test-suite`, a publicação Pages é condicionada a `refs/heads/main` e aos eventos `push` ou `workflow_dispatch`; o evento `pull_request` não satisfaz a condição de deploy. Isso é uma proteção presente nesse arquivo, não uma prova de que nenhum outro workflow publique produção.

**Ação recomendada:** manter a suíte completa no workflow canônico, auditar todos os workflows após integração e confirmar o status do job de deploy e do ambiente Pages na PR resultante.

### Médio — fonte editorial e documentação podem divergir

O contrato do catálogo descreve fontes editoriais como `docs/README_SITES.json` e `docs/README_FEATURED.json`, com `docs/project-catalog.json` como catálogo gerado. A documentação de build descreve `docs/project-catalog.json` como fonte gerada por `scripts/update_profile.py`. Isso pode ser compatível, mas o fluxo precisa ser confirmado no código real e descrito de forma única, incluindo qual comando regenera cada arquivo.

### Médio — verificações de URL no workflow são mais fracas que as do builder

O workflow valida URLs de site como HTTPS com `netloc`, enquanto o builder de segurança implementa verificações adicionais para hosts locais, credenciais, IPs não globais e portas. A validação do workflow é uma segunda barreira de formato, não substitui a regra de segurança do builder.

## Sobreposição de arquivos observada

| Arquivo | Branches com alterações relevantes | Risco |
| --- | --- | --- |
| `scripts/build_portfolio.py` | `feat/v2p2-project-catalog`, `feat/v2p2-build-security`, `feat/v2p2-architecture` | Alto: implementação duplicada/contratos diferentes |
| `tests/test_build_portfolio.py` | `feat/v2p2-design-system`, `feat/v2p2-build-security`, `feat/v2p2-project-catalog`, `feat/v2p2-architecture` | Alto: regressões ou testes perdidos em resolução de conflito |
| `.github/workflows/deploy-portfolio.yml` | `feat/v2p2-test-suite`, `feat/v2p2-architecture` | Alto: cobertura de testes e política de deploy diferentes |
| `assets/site.css` | `feat/v2p2-design-system`, `feat/v2p2-a11y-performance`, `feat/v2p2-architecture` | Médio: conflitos visuais e de acessibilidade |
| `assets/site.js` | `feat/v2p2-project-catalog`, `feat/v2p2-architecture` | Alto: contrato frontend/backend |
| `index.html` | `feat/v2p2-a11y-performance`, `feat/v2p2-project-catalog`, `feat/v2p2-architecture` | Médio: IDs/semântica e inicialização |

## Testes reais e checks

- Comparações de refs/arquivos remotos: executadas via integração GitHub.
- Leitura estática de builder, contrato, testes e workflow: executada.
- Execução local de `python -m unittest discover -s tests -v`: **não executada nesta sessão**; não há checkout/terminal do repositório conectado disponível nesta operação.
- Build integrado em `dist/`, inspeção de artefato e teste no navegador: **pendentes**.
- Checks atuais por commit das branches e resultado de deploy: **não confirmados neste relatório**.

## Ordem recomendada de integração

1. Escolher e atualizar a base de integração a partir de `feat/profile-v2-consolidated`; resolver a divergência da arquitetura separadamente.
2. Integrar uma única implementação de builder/contrato do catálogo.
3. Integrar o frontend contra o schema público final.
4. Integrar os testes unitários e de contrato; manter a descoberta completa no workflow.
5. Integrar design system e acessibilidade, resolvendo as alterações sobrepostas de CSS/HTML/JS.
6. Integrar documentação operacional com os comandos e caminhos finais.
7. Executar build + suíte completa + allowlist de `dist/` no commit integrado atualizado.
8. Verificar todos os workflows para garantir que PRs só validem e produzam artefatos de inspeção, sem publicar em produção.
9. Conferir os checks do commit final e abrir/atualizar PR para revisão humana.

## Estado desta entrega

Este documento registra achados reproduzíveis na comparação remota. Não foram mescladas branches, fechadas PRs, alterada `main` ou acionado deploy. Correções de código e validação integrada de runtime continuam bloqueadas até haver uma execução de testes sobre o checkout combinado; não seria correto declarar isso como concluído sem evidência.
