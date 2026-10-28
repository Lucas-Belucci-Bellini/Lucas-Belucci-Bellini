# Contrato funcional do catálogo de projetos

**Status:** especificação para a Parte 3 — Conversa 2/20  
**Branch de trabalho:** `feat/v3p3-product-contract`  
**Base inspecionada:** `feat/profile-v2-consolidated`  
**Fonte canônica atual:** `docs/project-catalog.json`  
**Consumidor atual:** `assets/site.js`, que carrega `data/projects.json`

Este documento define comportamentos verificáveis para a central de projetos. Ele documenta o contrato atual e separa explicitamente os requisitos que ainda precisam de implementação. Não declara que todos os requisitos já estejam implementados.

## 1. Decisões arquiteturais

1. **Uma única fonte de dados:** `docs/project-catalog.json` é o catálogo canônico gerado por `scripts/update_profile.py`. Não manter uma segunda lista de projetos no JavaScript, HTML ou em páginas de detalhe.
2. **Artefato publicado:** o site consome `data/projects.json`, gerado/publicado a partir do catálogo canônico pelo pipeline existente. Não editar o artefato publicado manualmente.
3. **Manifestos editoriais continuam separados:** usar os manifestos existentes, como `docs/README_SITES.json` e `docs/README_FEATURED.json`, para as decisões editoriais que lhes pertencem; não copiá-los para outra lista de projetos.
4. **Compatibilidade de schema:** manter `lucas-belucci-bellini/project-catalog@1` e os campos atuais sempre que possível. Mudanças incompatíveis exigem uma nova versão explícita do schema e atualização coordenada do gerador, pipeline, site e testes.
5. **Falha fechada:** dado inválido, projeto privado ou link inválido nunca deve ser promovido para conteúdo público por inferência ou fallback.
6. **Sem promessas não verificadas:** declarar um endereço de site não significa que ele está operacional. A interface só pode chamar um site de “verificado” e oferecer CTA de site quando os critérios da seção 3 forem atendidos.
7. **README versus site:** o README especial do perfil permanece uma entrada curta e profissional; o catálogo, os filtros e as informações ampliadas pertencem ao site.

## 2. Modelo de dados e campos

O registro atual contém, entre outros, `repository`, `name`, `slug`, `description`, `category`, `category_label`, `status`, `github`, `github_visible`, `github_role`, `primary_cta`, `secondary_cta`, `featured`, `marketing_priority`, `private` e os campos de verificação do site (`website_declared`, `website_status`, `website_http_status`, `website_source`, `website`). Os campos opcionais não devem ser tratados como obrigatórios sem validar os dados reais e o gerador.

- `repository`: identificador `owner/repo`; deve ser único no catálogo.
- `slug`: identificador estável para navegação/detalhes; deve ser único. Não gerar conteúdo do projeto a partir do slug.
- `name`: nome de exibição.
- `description`: resumo editorial; se ausente/vazio, exibir texto neutro, sem inventar capacidades.
- `category`: valor canônico usado para filtros.
- `category_label`: rótulo de apresentação opcional; não usar como substituto silencioso do valor de filtro.
- `status`: um dos valores reconhecidos atualmente: `🟢 Active`, `🟡 In Development`, `🔵 Experimental`, `🟣 Academic`. Valores novos devem ser adicionados ao contrato, gerador e testes antes de serem expostos como filtros.
- `private` e `github_visible`: controles de elegibilidade pública; valores ausentes não autorizam inferir que um projeto privado é público.
- `featured` e `marketing_priority`: metadados editoriais de destaque/ordenação, não métricas de qualidade ou uso.
- `website_status`, `website_http_status`, `website_declared`, `website_source`, `website`: declaração e resultado da auditoria; não são intercambiáveis.

O campo `counts` e qualquer contagem derivada são metadados gerados, nunca devem ser usados como fonte para reconstruir a lista de projetos. As categorias são derivadas dos registros elegíveis, salvo se o gerador já definir uma taxonomia canônica explícita.

## 3. Regras de publicação e links

### GitHub

Um link de repositório só pode ser exibido quando todas as condições forem verdadeiras:

- registro é objeto válido;
- `private !== true` e `github_visible !== false`;
- `github` é HTTPS, hostname exatamente `github.com` e aponta para um caminho de repositório `/owner/repo`;
- a entrada não falha na validação estrutural do catálogo.

Links externos devem usar `rel="noopener noreferrer"` quando abertos em nova aba. Não exibir links GitHub construídos a partir de nomes ou slugs se o campo de origem estiver ausente/inválido.

### Sites de projetos

Um CTA de site só pode ser mostrado como **“Site verificado”** quando:

- `website_status === "verified"`;
- `website` é uma URL HTTPS válida;
- `website_http_status === 200`;
- o registro é elegível para publicação pública.

`website_declared` é apenas o endereço descoberto/declarado. `unreachable`, `invalid`, status ausente ou qualquer status HTTP diferente de 200 não pode ser apresentado como site verificado. Nessas situações, ocultar o CTA do site e manter o CTA GitHub quando válido. Não transformar uma URL declarada em URL verificada no cliente.

A verificação HTTP é um retrato do momento da auditoria, não uma garantia permanente de disponibilidade. A interface não deve sugerir monitoramento em tempo real.

## 4. Catálogo, busca e filtros

### Elegibilidade

O catálogo público deve conter somente projetos públicos elegíveis. A interface deve validar os registros antes de renderizá-los e ignorar registros malformados sem expor dados privados. Se nenhum registro elegível permanecer, apresentar o estado vazio/indisponível apropriado, sem inventar projetos.

### Busca

- Busca textual sem distinção entre maiúsculas e minúsculas e com comparação localizada em pt-BR.
- A busca deve ser aplicada em conjunto com os filtros ativos.
- Campos mínimos de busca: nome, descrição, categoria, rótulo de categoria e status.
- Se forem acrescentados aliases, stack ou tecnologias, eles devem vir de campos existentes e documentados no schema; não inferir tecnologias pelo nome do repositório.
- Espaços no início/fim da consulta não devem alterar o resultado.
- Consulta vazia remove apenas o critério textual; não limpa os demais filtros.

### Filtros

- Categoria e status são filtros combináveis com a busca e entre si (lógica AND entre dimensões).
- “Todas” significa não restringir aquela dimensão.
- As opções de categoria/status devem ser derivadas dos projetos elegíveis, evitando opções sem resultados.
- Categorias e status devem ser ordenados de forma estável e legível em pt-BR.
- Ao alterar um filtro, atualizar resultados e contagem sem recarregar a página.

### Ordenação

- A ordem inicial deve ser determinística e refletir os metadados editoriais existentes: destaque primeiro, prioridade editorial decrescente quando disponível, e nome em ordem alfabética como desempate.
- Registros sem metadados editoriais válidos não podem causar erro; devem permanecer em posição determinística.
- Não apresentar `marketing_priority` como avaliação, popularidade ou métrica de desempenho.
- Se controles de ordenação forem adicionados à interface, cada opção deve ter regra explícita e teste. Até lá, não anunciar opções que não existam visualmente.

## 5. Cartões e navegação para detalhes

### Cartão do catálogo

Cada cartão deve apresentar, quando disponível: nome, categoria, status reconhecido e descrição. O cartão deve ter um CTA de site somente quando verificado e um CTA GitHub somente quando válido. Não renderizar botões/links vazios nem um CTA que não funcione.

Texto ausente deve receber um fallback neutro e consistente. Status desconhecido não deve ser mapeado silenciosamente para outro status; ocultar o selo até o valor ser reconhecido.

### Página de detalhes

O catálogo atual já fornece `slug`, mas esta auditoria do `assets/site.js` confirma apenas o grid, busca, filtros e cartões; não confirma uma página de detalhes implementada. Portanto, detalhes são requisito futuro, não funcionalidade declarada como existente.

Quando implementada, a página deve resolver o projeto pelo `slug` no catálogo canônico carregado, sem duplicar conteúdo em arquivos de detalhe. Deve exibir apenas dados disponíveis e links sujeitos às mesmas regras de elegibilidade/verificação da listagem. Slug inexistente ou inválido deve mostrar “Projeto não encontrado” e uma forma clara de voltar ao catálogo. A navegação deve ser compartilhável e acessível por teclado; não depender apenas de estado transitório em memória.

## 6. Estados de interface e falhas

- **Carregamento:** indicar que os dados estão sendo carregados, por exemplo com `aria-busy="true"`; evitar apresentar contagem final antes de carregar.
- **Sucesso:** renderizar resultados e contagem de filtrados/total elegível.
- **Busca/filtros sem resultados:** informar que nenhum projeto corresponde aos critérios e orientar a limpar/alterar os filtros. Diferenciar isso de erro de carregamento.
- **Catálogo vazio:** se o JSON válido não tiver registros públicos elegíveis, mostrar estado vazio explícito; não simular sucesso com dados estáticos.
- **Erro de rede/HTTP/JSON/schema:** exibir mensagem compreensível, não renderizar dados parciais como se estivessem corretos e oferecer orientação de recuperação sem expor stack traces, tokens ou dados internos.
- **Link inválido ou site não verificado:** omitir o CTA correspondente; não quebrar o restante do cartão.
- **Acessibilidade:** mensagens dinâmicas devem ser anunciáveis por tecnologias assistivas; controles devem ter rótulos associados, foco visível e funcionamento por teclado.
- **Segurança:** construir nós e texto com APIs seguras (`textContent`/DOM); não inserir nomes, descrições ou URLs do catálogo como HTML arbitrário.

## 7. Critérios de aceite testáveis

1. [ ] O catálogo publicado deriva de `docs/project-catalog.json` via pipeline existente; não há uma segunda lista manual de projetos no frontend.
2. [ ] `repository` e `slug` não se repetem entre registros elegíveis; violações são detectadas por teste/validação.
3. [ ] Projeto privado, explicitamente oculto ou com URL GitHub inválida nunca aparece no grid.
4. [ ] Um CTA de site só aparece para `website_status=verified`, URL HTTPS válida e HTTP 200.
5. [ ] Site declarado como inacessível/inválido ou sem verificação não recebe rótulo “Site verificado”.
6. [ ] A busca encontra correspondências em nome, descrição, categoria/rótulo e status, sem diferenciar maiúsculas/minúsculas.
7. [ ] Busca, categoria e status funcionam simultaneamente; limpar a busca não remove filtros e vice-versa.
8. [ ] Opções de filtro vêm dos registros elegíveis e não oferecem categorias/status sem resultados.
9. [ ] A ordenação inicial é determinística e tem desempate por nome.
10. [ ] Dados opcionais ausentes ou status desconhecido não causam crash nem geram informação inventada.
11. [ ] O estado sem resultados por filtro é diferente do estado de falha do catálogo.
12. [ ] HTTP não-OK, erro de rede, JSON inválido e schema incompatível geram mensagem de erro; nenhum dado privado é exibido.
13. [ ] Cartões não contêm links vazios; links externos em nova aba têm proteção `noopener noreferrer`.
14. [ ] Página de detalhes, quando implementada, usa o catálogo como fonte única e trata slug desconhecido.
15. [ ] Busca, filtros, estados de erro/vazio e navegação essencial são utilizáveis por teclado e expostos semanticamente.
16. [ ] Os testes verificam o comportamento, não apenas a existência de seletores ou strings no código.

## 8. Lacunas observadas na implementação inspecionada

No `assets/site.js` da branch-base:

- já existem carregamento de `data/projects.json`, busca, filtros de categoria/status, contagem, cartões, validação estrutural básica de URLs e estado de erro/vazio;
- o cliente filtra registros privados/ocultos e valida links GitHub antes de renderizar;
- o CTA de site é decidido pela validade HTTPS de `project.website`, mas o cliente não confirma explicitamente `website_status === "verified"` e `website_http_status === 200`; a regra precisa ser reforçada no cliente e coberta por teste;
- a ordenação de resultados não é aplicada explicitamente no `render()`; ela depende da ordem recebida do JSON;
- a busca atual não inclui `slug`, `repository` ou stack/tecnologias;
- os detalhes de projeto não foram confirmados como implementados;
- não há `package.json` na raiz da branch-base consultada; identificar o mecanismo real de build/testes antes de escolher comandos de teste, sem presumir npm.

Essas lacunas são registros de auditoria, não autorização para expandir o escopo desta conversa para implementar todas as funcionalidades. A próxima etapa deve priorizar validação automatizada do contrato e correções diretamente necessárias ao comportamento de publicação dos links.

## 9. Fora de escopo

- Reescrever o design visual ou o README completo.
- Adicionar projetos, descrições, tecnologias, métricas ou URLs não presentes nas fontes verificadas.
- Alterar a taxonomia ou criar um novo schema sem necessidade comprovada.
- Implementar backend, autenticação, analytics ou monitoramento em tempo real.
- Fazer merge em `main`, aprovar PR ou publicar em produção.
