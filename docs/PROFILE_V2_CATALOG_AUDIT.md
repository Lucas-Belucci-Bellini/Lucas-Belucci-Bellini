# Auditoria do catálogo canônico de projetos

**Escopo:** inspeção estática de `docs/project-catalog.json`, do builder e dos testes na referência `feat/profile-v2-consolidated`.  
**Catálogo inspecionado:** blob `2e16f15bf83c2669b48452c72b0a56b92f8d7794`.  
**Objetivo:** identificar riscos antes de compor a integração; não editar manualmente os dados gerados.

## 1. Resumo observado

| Métrica declarada/contada | Valor |
|---|---:|
| Registros no JSON | 99 |
| Projetos marcados como públicos | 78 |
| Projetos marcados como privados | 21 |
| Sites com estado `verified` | 12 |
| URLs declaradas, mas não verificadas | 6 |
| Slugs duplicados no conjunto completo | 0 |
| Projetos públicos com URL GitHub fora do formato esperado | 0 |

Os números acima descrevem este blob específico, não uma contagem garantida do estado atual de GitHub ou do deploy. O próprio arquivo informa que é gerado por `scripts/update_profile.py`; mudanças devem ser feitas nas fontes do gerador, não diretamente neste JSON.

## 2. Contrato do builder confirmado por leitura estática

`scripts/build_portfolio.py` declara o schema `lucas-belucci-bellini/project-catalog@1` e usa allowlist explícita dos campos públicos: `name`, `slug`, `description`, `category`, `category_label`, `status`, `github` e `website`.

O builder:
- exclui registros com `private: true`;
- exige `private: false` e `github_visible: true` para considerar um projeto público;
- valida campos obrigatórios, slug, categoria, status e link GitHub;
- só inclui `website` quando `website_status == "verified"` e a URL passa a validação HTTPS;
- ordena os projetos de modo determinístico e gera `dist/data/projects.json`.

Essa inspeção confirma a lógica no código-fonte lido; não substitui a execução dos testes.

## 3. Sites declarados não devem ser promovidos automaticamente

Neste snapshot, seis sites declarados aparecem com estado `unreachable` e HTTP 404:

- Projeto-Baluarte — `https://projeto-baluarte.vercel.app`
- Academic-Portfolio — `https://academic-portfolio-blush.vercel.app`
- Ark-Initiative — `https://ark-initiative.vercel.app`
- DriveTax-Motors — `https://drive-tax-motors.vercel.app`
- FLUX — `https://flux-ten-red.vercel.app`
- Recycle-game — `https://recycle-game-opal.vercel.app`

Não apresentar esses endereços como links de site ativos enquanto não forem verificados novamente. O site do portfólio deve continuar oferecendo o link GitHub seguro quando disponível. Um 404 neste snapshot é um sinal de verificação pendente, não prova de que o projeto ou repositório deixou de existir.

## 4. Casos de teste que a composição final deve cobrir

- [ ] JSON inválido, schema inesperado e campos obrigatórios ausentes.
- [ ] Registro privado não aparece no payload público.
- [ ] Projeto não visível no GitHub não aparece como público.
- [ ] Slugs duplicados e slugs malformados falham explicitamente.
- [ ] URL GitHub inválida ou com host não permitido não vira link de projeto.
- [ ] `javascript:`, URL com credenciais e HTTP não são aceitos como site seguro.
- [ ] Site declarado mas não verificado é omitido do payload.
- [ ] Site verificado com URL inválida também é omitido.
- [ ] Build repetido produz JSON idêntico e remove artefatos obsoletos.
- [ ] O build não escreve fora do destino permitido `dist/`.

Os testes existentes precisam ser executados no checkout composto. A lista é um checklist de cobertura e não afirma que todos os casos já existem ou passaram.

## 5. Decisão

**Preservar o catálogo canônico e sua proveniência.** Corrigir URLs ou status na fonte que alimenta `scripts/update_profile.py` e regenerar o JSON. Não aumentar a contagem de sites ativos nem preencher endereços por suposição. Antes de release, regenerar o catálogo, revisar o diff e executar o builder e a suíte de testes na mesma composição de código.

## 6. Limitações

Esta auditoria é uma inspeção estática de uma referência remota e não uma execução local do build. Não verificou a disponibilidade atual das URLs por uma nova requisição HTTP, não executou os testes e não confirma o estado do site implantado.
