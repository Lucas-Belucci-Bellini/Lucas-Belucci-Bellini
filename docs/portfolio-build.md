# Build estático do portfólio V2

A fonte editorial continua sendo `docs/project-catalog.json`, gerada por `scripts/update_profile.py`. Não mantenha uma segunda lista manual concorrente: ajuste os manifestos editoriais e execute o gerador existente.

## Comandos locais

Use Python 3.12, na raiz do repositório:

```bash
python -m py_compile scripts/build_portfolio.py
python -m json.tool docs/project-catalog.json >/dev/null
python -m unittest discover -s tests -v
node --test tests/test_portfolio_frontend.cjs
python scripts/build_portfolio.py --out /tmp/portfolio-dist
```

O workflow de CI executa os mesmos comandos de validação e descoberta de testes antes de construir o artefato. O último comando cria a saída em um diretório temporário fora do repositório; para reproduzir exatamente o caminho padrão usado no CI, execute `python scripts/build_portfolio.py`.

O builder valida o schema `lucas-belucci-bellini/project-catalog@1`, filtra projetos privados/não visíveis, valida slugs e URLs HTTPS e só publica websites marcados como verificados. A saída é determinística e limitada a `dist/` quando o caminho padrão é usado.

## Artefato público

O diretório de saída contém exatamente `index.html`, `assets/site.css`, `assets/site.js`, `assets/favicon.svg`, `data/projects.json` e `.nojekyll`. O JSON público contém somente `name`, `slug`, `description`, `category`, `category_label`, `status`, `github` e, quando verificado, `website`. Campos de controle como `repository`, `private`, `github_visible`, `marketing_priority` e `website_status` não são publicados.

O frontend consome exclusivamente `data/projects.json`, nunca o catálogo editorial bruto. `tests/test_portfolio_contract.py` verifica o contrato do JSON real gerado, a allowlist de campos, a ordenação determinística, a contagem, os URLs e casos de privacidade/campos ausentes. O workflow verifica também a allowlist exata de arquivos do artefato e não publica em produção a partir de pull requests.

## Limites conhecidos

A suíte cobre o builder Python, o contrato do JSON e o comportamento principal do frontend por meio de um harness DOM mínimo executado no Node.js: busca, filtros por categoria/status, links seguros, campos ausentes e estados de erro HTTP/payload. Esse harness não substitui testes em navegador real; layout, acessibilidade integrada e diferenças entre navegadores ainda exigem validação visual/browser.
