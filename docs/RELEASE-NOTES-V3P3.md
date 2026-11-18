# V3P3 — Notas de release (rascunho)

**Status:** rascunho para revisão; não é anúncio de lançamento.  
**Versão/commit publicado:** ainda não definido.  
**Deployment de produção:** não confirmado.

## Objetivo

Organizar o portfólio como ponto de entrada confiável para os projetos, com catálogo público canônico, descoberta por busca e filtros, e informações ampliadas por projeto. O README de perfil deve permanecer uma apresentação curta e direcionar visitantes ao catálogo.

## Mudanças previstas para validar no candidato consolidado

- Catálogo público gerado a partir da fonte canônica `docs/project-catalog.json`.
- Busca e filtros no catálogo.
- Navegação para detalhes de projetos e links de código/site.
- Regras de privacidade para impedir que dados privados ou campos internos entrem no artefato estático.
- Verificação de links de sites antes de anunciá-los como disponíveis.
- Layout responsivo, navegação por teclado e tratamento de estados vazios/erros.

> Esta lista descreve os objetivos do trabalho V3P3; não afirma que todos os itens foram integrados ou aprovados. Confirmar cada item no diff da branch candidata e nos testes antes de publicar.

## Verificações exigidas antes de liberar

- [ ] PR candidato revisado; base e head corretos; diff sem alterações fora de escopo.
- [ ] `V2 Validation` aprovado.
- [ ] `Portfolio website — build and deploy` aprovado no PR.
- [ ] Testes de busca/filtros, detalhes, links, acessibilidade e mobile executados com resultados registrados.
- [ ] Artefato público contém somente a allowlist definida no workflow.
- [ ] Catálogo e URLs de sites revalidados.
- [ ] SHA completo do commit candidato e runs dos checks registrados.
- [ ] Aprovação explícita para merge e publicação obtida separadamente.

## Riscos conhecidos

- O SHA da versão final e a URL de produção ainda não foram registrados.
- A disponibilidade dos sites externos pode mudar após a verificação.
- O workflow de deploy tem gatilhos distintos para PR, push e execução manual; uma execução de PR não significa publicação.
- Rollback requer identificar uma versão saudável por SHA e deployment real.

Consulte [Plano de release, implantação e rollback](./RELEASE-PLAN-V3P3.md) para a sequência operacional e o checklist pós-deploy.
