# V2 Parte 2 — Release gate e checklist final

> **Estado inicial:** NÃO LIBERADA. Este documento é um gate de revisão, não uma declaração de build aprovado, merge concluído ou publicação realizada.
>
> **Referência auditada:** `feat/profile-v2-consolidated`  
> **Branch desta documentação:** `feat/v2p2-release`

## 1. Contrato público do artefato

O gerador `scripts/build_portfolio.py` declara o schema de entrada `lucas-belucci-bellini/project-catalog@1` e produz `dist/data/projects.json` com schema `lucas-belucci-bellini/public-portfolio@1`, `project_count` e `projects`.

Campos públicos permitidos por projeto: `name`, `slug`, `description`, `category`, `category_label`, `status`, `github` e, somente quando verificado, `website`.

Regras de publicação observadas no gerador:
- exige `private: false` explícito e omite projetos com `private: true`;
- omite projetos sem `github_visible: true`;
- valida slug, categoria, status e URL HTTPS do repositório GitHub;
- só inclui `website` quando `website_status: "verified"` e a URL é HTTPS válida;
- exige ao menos um projeto público válido;
- copia apenas `index.html`, `assets/site.css`, `assets/site.js` e `assets/favicon.svg`, gera `data/projects.json` e `.nojekyll`.

**Compatibilidade obrigatória:** os testes e o frontend devem consumir exatamente esse contrato. Não renomear campos ou schemas isoladamente. Não incluir dados privados, tokens, arquivos de configuração ou conteúdo interno no artefato.

## 2. Evidência de integração disponível

Comparações remotas feitas contra `feat/profile-v2-consolidated`:
- `feat/v2p2-build-security`: 3 commits à frente, 0 atrás; modifica gerador, testes e documentação do build.
- `feat/v2p2-test-suite`: 6 commits à frente, 0 atrás; adiciona teste de contrato e altera workflow/documentação.
- `feat/v2p2-documentation`: 3 commits à frente, 0 atrás; altera README e adiciona documentação operacional/arquitetural.
- `feat/v2p2-deployment`: idêntica à referência, sem commits exclusivos.
- `feat/v2p2-architecture`: divergente, 7 commits exclusivos e 22 commits atrás da referência; contém alterações sobrepostas incluindo workflow, frontend, gerador, testes e documentos.

Esses dados indicam que as alterações de algumas branches continuam propostas, não integradas à referência. A branch de arquitetura não deve ser mesclada diretamente sem rebase/integração e revisão de conflitos. A igualdade da branch de deploy com a referência significa que ela não fornece mudanças exclusivas a integrar.

## 3. Checks e resultados

| Verificação | Resultado |
|---|---|
| Comparação de branches remotas | Executada; resultados acima |
| Execução local do gerador | Não executada nesta auditoria |
| Execução local de testes | Não executada nesta auditoria |
| Build de produção | Não confirmado |
| Checks CI no SHA candidato | Não confirmados nesta documentação |
| Deploy do GitHub Pages | Não confirmado |
| URL publicada e smoke test | Não confirmados |

Não converter ausência de evidência em aprovação. Antes do merge, registrar links/SHA e estado final dos checks reais.

## 4. Checklist de aceite

### Acessibilidade
- [ ] Fluxo completo apenas com teclado; foco visível e ordem lógica.
- [ ] Link para pular ao conteúdo, landmarks, títulos e nomes acessíveis.
- [ ] Contraste validado; zoom de 200% e reflow sem perda de conteúdo.
- [ ] Estados de carregamento, vazio e erro anunciados de forma acessível.
- [ ] `prefers-reduced-motion` respeitado; nenhum conteúdo depende apenas de cor.

### Privacidade e segurança
- [ ] Artefato `dist/` contém somente os arquivos públicos previstos.
- [ ] Projetos privados e entradas sem `github_visible: true` ausentes do JSON gerado.
- [ ] Campos extras do catálogo não vazam para a saída.
- [ ] Links são validados; conteúdo do catálogo é renderizado sem injeção HTML.
- [ ] Nenhum secret, token, `.env`, dado pessoal desnecessário ou log sensível no artefato/logs.
- [ ] Permissões do workflow limitadas ao mínimo necessário; sem secrets expostos.

### Navegação e conteúdo
- [ ] Links internos, âncoras e navegação móvel funcionam.
- [ ] Links externos têm comportamento seguro e indicação compreensível.
- [ ] URLs de sites só aparecem quando verificadas; não declarar sites ativos por suposição.
- [ ] README renderiza corretamente no GitHub e mantém os marcadores de conteúdo gerado.

### Responsividade e desempenho
- [ ] Validado em viewport móvel estreito, tablet e desktop.
- [ ] Sem overflow horizontal, controles cortados ou alvos de toque impraticáveis.
- [ ] Imagens/recursos externos falhando não impedem o conteúdo principal.
- [ ] Console sem erros críticos; métricas de desempenho registradas se Lighthouse for usado.

### GitHub Actions e Pages
- [ ] Workflow executa nos eventos esperados, inclusive no PR para a branch correta.
- [ ] Build falha explicitamente em catálogo inválido e limpa/recria `dist/` de modo seguro.
- [ ] Upload/deploy usa o artefato produzido pelo mesmo commit validado.
- [ ] Permissões e configuração de Pages conferidas; nenhum workflow concorrente publica conteúdo divergente.
- [ ] Run de CI verde no SHA exato a integrar.
- [ ] Após merge autorizado: run de publicação concluído e URL real acessível.
- [ ] Smoke test pós-publicação: página inicial, catálogo, filtros, links e console.

## 5. Ordem de integração proposta

A ordem abaixo é uma proposta de dependências, não uma ordem de aprovação automática:

1. **Arquitetura/contrato** — resolver a divergência de `feat/v2p2-architecture` e confirmar a referência canônica.
2. **Build e segurança** — integrar `feat/v2p2-build-security` após revisão do contrato.
3. **Testes** — integrar `feat/v2p2-test-suite` contra o gerador final e obter CI verde.
4. **Catálogo e interface** — conferir alterações de catálogo, landing e design system; resolver conflitos e executar testes visuais/acessíveis.
5. **Acessibilidade/desempenho** — revisar sobreposições com HTML/CSS/JS e registrar evidências.
6. **README e documentação operacional** — integrar depois que nomes, comandos e contrato estiverem estáveis.
7. **Deploy/Pages** — somente depois de build/testes verdes e revisão do workflow.
8. **Release gate** — confirmar todos os checks, aprovações humanas e o diff final; só então autorizar merge/publicação.

Não aprovar PRs em lote apenas pela ordem numérica. Confirmar para cada PR base, head, diff, checks, revisões e sobreposições atuais.

## 6. Encerramento de PRs antigos

Só fechar PR antigo após confirmar que:
- o PR substituto está aberto contra a base correta;
- todo o diff útil foi incorporado e revisado;
- não existem commits exclusivos, comentários pendentes ou requisitos perdidos;
- o CI do substituto cobre o conteúdo substituído.

Registrar no PR antigo o link do substituto e o motivo antes de encerrar. Não encerrar automaticamente durante esta auditoria.

## 7. Procedimento de merge e validação pós-publicação

1. Atualizar a branch candidata com a base aprovada; resolver conflitos sem descartar alterações silenciosamente.
2. Executar os comandos documentados para testes e build; salvar logs e SHA.
3. Confirmar que o artefato não contém conteúdo privado e que o contrato JSON é compatível.
4. Aguardar todos os checks obrigatórios concluírem com sucesso no SHA candidato.
5. Fazer revisão humana do diff final, dos workflows e da lista de arquivos.
6. Aprovar e mesclar somente com autorização explícita do responsável.
7. Aguardar o workflow de Pages terminar com sucesso; isso é distinto do merge.
8. Abrir a URL publicada fornecida pelo próprio GitHub, validar HTTP/carregamento e executar smoke tests em desktop e celular.
9. Registrar URL, SHA publicado, workflow run, data e eventuais limitações.
10. Se falhar, interromper a divulgação e corrigir/reverter por procedimento revisado; não declarar release concluída.

## Decisão

**NO-GO até evidência adicional.** Esta branch documenta o gate e as evidências de comparação; não prova que testes/build/deploy passaram. A aprovação depende da integração real das mudanças, checks verdes no SHA final, revisão do artefato e confirmação humana da publicação.
