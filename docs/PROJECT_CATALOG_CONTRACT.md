# Contrato do catálogo público de projetos

## Fontes e responsabilidades

- **Fonte editorial canônica:** `docs/README_SITES.json`, `docs/README_FEATURED.json` e os metadados de repositório consumidos por `scripts/update_profile.py`.
- **Catálogo editorial gerado:** `docs/project-catalog.json`, schema `lucas-belucci-bellini/project-catalog@1`. Não editar manualmente; atualizar as fontes editoriais e executar o gerador.
- **Builder público:** `scripts/build_portfolio.py`. Valida o schema, exclui entradas privadas/ocultas, aplica allowlist e produz os arquivos estáticos.
- **Caminho público preservado:** `data/projects.json` relativo à raiz publicada. O builder gera esse caminho em `dist/data/projects.json`; o frontend não deve depender do caminho do arquivo editorial.

## Contrato do payload servido

A raiz do JSON público contém:

- `schema`: `lucas-belucci-bellini/public-portfolio@1`
- `project_count`: número de itens após validação e filtragem
- `projects`: array ordenado deterministicamente pelo nome

Cada item público pode conter **somente** estes campos:

| Campo | Contrato |
| --- | --- |
| `name` | Nome público não vazio |
| `slug` | Identificador minúsculo com hífens |
| `description` | Texto editorial; pode ser vazio |
| `category` | Categoria da taxonomia declarada |
| `category_label` | Rótulo editorial de exibição |
| `status` | Um dos estados públicos permitidos |
| `github` | URL HTTPS de repositório no GitHub |
| `website` | Opcional; incluído apenas quando marcado como verificado e seguro |

Não publicar `private`, `github_visible`, `repository`, prioridades de marketing, origem/estado/código HTTP de verificação de site, timestamps, credenciais, tokens ou outros campos internos. A lista é uma allowlist, não uma cópia filtrada por exclusão.

## Segurança e renderização

- O builder rejeita registros sem o booleano explícito `private`, slugs inválidos/duplicados e campos obrigatórios ausentes; registros sem link GitHub seguro não viram cartões.
- URLs de site precisam ser HTTPS, sem credenciais, espaços ou hosts locais. URLs inválidas de site são omitidas, não usadas como links.
- O cliente valida novamente schema, campos e URL GitHub antes de renderizar. Um site só aparece quando `website_status` é `verified` na origem e a URL pública é segura.
- Textos de projeto são tratados como texto via `textContent`/criação de nós; não inserir dados editoriais usando `innerHTML`.
- Links externos abrem com `rel="noopener noreferrer"`.
- Falha de rede/HTTP/JSON/schema apresenta estado de erro; um catálogo válido sem itens e uma busca sem correspondências têm estados vazios distintos.
- Busca, categoria e status combinam-se por AND. A ordenação é explícita (nome A–Z, nome Z–A ou status); a contagem informa resultados filtrados sobre o total elegível.

## Acessibilidade

Os controles devem ter rótulos acessíveis, foco visível, e a contagem deve anunciar mudanças com `aria-live="polite"`. O grid usa `aria-busy` apenas durante o carregamento. Mensagens de erro são expostas com `role="status"`.

## Validação local

Na raiz do repositório:

```sh
python -m unittest tests.test_build_portfolio
python scripts/build_portfolio.py --root . --out ./dist
```

O segundo comando valida o **build real** e gera `dist/data/projects.json`; testes isolados do builder não provam, por si só, que todos os assets reais estão presentes nem que a página carregada funciona no navegador. Inspecione também o payload gerado e teste busca/filtros/ordenação no site servido pela raiz `dist/`.


## Descoberta: destaques e categorias

- A interface deriva as opções de categoria e os chips de navegação dos projetos elegíveis do JSON público; não mantém listas de categorias duplicadas no HTML.
- A seção de destaques usa o campo público booleano `featured` projetado pelo builder. Só é verdadeiro quando a fonte editorial marca explicitamente `featured: true`, o projeto é elegível para publicação, a descrição não está vazia nem usa o placeholder `Descrição pública não informada.`, e o status não é arquivado.
- O campo `featured` não expõe prioridades internas de marketing. Os destaques seguem ordenação determinística pelo nome no catálogo; as opções de ordenação do catálogo continuam independentes.
- Selecionar uma categoria usa o mesmo controle de filtro do catálogo; busca, categoria e status continuam combinados. A seção de destaques acompanha os filtros ativos e não inventa projetos paralelos.
- Testes do builder verificam a seleção explícita, a exclusão de privados/placeholders/arquivados, os campos publicados e a ordenação determinística.
