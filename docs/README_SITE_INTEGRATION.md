# Contrato de integração: README do perfil e site V2

## Papéis de cada superfície

- **README.md** é a apresentação de entrada no GitHub: identidade visual, resumo pessoal, produtos em destaque, links verificados e informação técnica de consulta rápida.
- **Site V2** será a experiência de navegação completa: catálogo interativo, pesquisa, filtros, páginas/detalhes de projetos e exploração por categoria.
- **docs/project-catalog.json** é o contrato de catálogo legível por máquina já produzido pelo gerador. Deve ser reutilizado como integração somente depois de validar o schema e o consumidor do site; não criar uma segunda lista manual concorrente.
- **docs/README_FEATURED.json**, **docs/README_SITES.json**, **docs/README_STACK.json** e **docs/README_EXCLUDED.json** continuam sendo manifestos editoriais com responsabilidades distintas. Não migrar nem apagar esses dados sem uma decisão documentada, migração verificável e testes de paridade.

## Fonte de verdade e limites

| Informação | Fonte canônica | Regra |
|:---|:---|:---|
| Inventário e visibilidade de repositórios | API do GitHub usada por `scripts/update_profile.py` | Filtrar exclusões editoriais antes de renderizar; não publicar conteúdo privado |
| Linguagens e bytes | mapas de linguagens públicos do GitHub | Agregar pelo gerador, sem copiar estatísticas manualmente |
| Categoria, status editorial e apresentação | regras em `scripts/project_catalog.py` e manifests existentes | Mudanças deliberadas e documentadas; não criar taxonomia paralela no README |
| Sites de projetos | homepage do GitHub e `docs/README_SITES.json`, verificados pelo gerador | Só publicar CTA de site quando o destino for verificado |
| Ferramentas/stack | `docs/README_STACK.json` com evidência pública | Não confundir linguagem detectada com tecnologia confirmada |
| Curadoria de destaques | `docs/README_FEATURED.json` | Preservar a ordem e a curadoria existentes |
| Endereço do próprio site V2 | registro explícito de publicação confirmado pelo responsável | Não inferir URL de nome de repositório, provedor ou preview |

O site deve consumir o catálogo existente ou um export validado dele. Se o schema precisar mudar, versionar a alteração, documentar compatibilidade e testar a paridade antes de alterar a fonte de dados. O README não deve duplicar o catálogo inteiro nem tentar funcionar como banco de dados.

## Regra para anunciar o site V2

**Nenhuma URL do site V2 foi adicionada nesta alteração**, pois o endereço de publicação não foi confirmado nos arquivos e evidências consultados. Não reutilizar URLs de sites de projetos como se fossem a URL do portfólio.

Quando a publicação estiver confirmada:
1. Verificar o endereço final e se ele corresponde ao site V2, não a um preview temporário.
2. Registrar a URL confirmada no local editorial apropriado e adicionar um CTA curto na área estática de navegação do README.
3. Não inserir o CTA dentro de qualquer bloco `START/END`; não alterar conteúdo gerado manualmente.
4. Atualizar esta documentação e testar o README, os marcadores e os links.
5. Manter o site como experiência completa e o README como apresentação compacta.

## Conteúdo técnico e Issues #23–#25

As Issues [#23](https://github.com/Lucas-Belucci-Bellini/Lucas-Belucci-Bellini/issues/23) e [#24](https://github.com/Lucas-Belucci-Bellini/Lucas-Belucci-Bellini/issues/24) definem comparações educacionais de frontend e backend TypeScript. A Issue [#25](https://github.com/Lucas-Belucci-Bellini/Lucas-Belucci-Bellini/issues/25) pede uma trilha curta no README. Esses conteúdos devem ser resumidos no README apenas quando forem pertinentes e confirmados como material de aprendizagem, com links oficiais; o catálogo do site pode oferecer a leitura expandida. Uma tecnologia mencionada em um guia não passa automaticamente a ser declarada como tecnologia usada nos projetos.

## Testes de preservação

A suíte existente `tests/test_golden_profile.py` roda o gerador offline com dados sintéticos e relógio fixo, compara as saídas byte a byte, verifica idempotência e inclui modos de saída sombra que não alteram os arquivos publicados. `scripts/validate_dynamic_sections.py` confirma que mudanças de geração não alteraram o conteúdo estático fora dos marcadores.

Ao revisar mudanças de geração:
- conferir que cada marcador `START/END` esperado continua presente uma única vez;
- executar `python -m unittest discover -s tests -v`;
- executar `python scripts/validate_project_links.py` e `python scripts/validate_dynamic_sections.py --before <cópia-anterior> --after README.md` quando houver refresh;
- revisar o diff do README e do catálogo; não regenerar fixtures golden sem revisar cada diferença;
- nunca rodar `--write` contra inventário parcial sem a proteção de token/inventário já exigida pelo gerador.

Este documento registra o contrato e a política de integração; não afirma que os testes foram executados nesta alteração quando não há resultado de execução disponível.
