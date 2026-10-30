# Build estático do portfólio V2

A fonte editorial continua sendo `docs/project-catalog.json`, gerada por `scripts/update_profile.py`. Não mantenha uma segunda lista manual concorrente: ajuste os manifestos editoriais e execute o gerador existente.

## Comandos locais

```bash
python -m py_compile scripts/build_portfolio.py
python -m unittest discover -s tests -p "test_build_portfolio.py" -v
python scripts/build_portfolio.py
```

O builder valida o schema `lucas-belucci-bellini/project-catalog@1`, filtra projetos privados/não visíveis, valida slugs e URLs HTTPS, e só publica websites marcados como verificados. A saída é determinística e limitada a `dist/`.

## Artefato público

O diretório `dist/` contém exatamente `index.html`, `assets/site.css`, `assets/site.js`, `assets/favicon.svg`, `data/projects.json`, `data/stack-inventory.json` e `.nojekyll`. O JSON público contém somente `name`, `slug`, `description`, `category`, `category_label`, `status`, `github` e, quando verificado, `website`. Campos de controle como `repository`, `private`, `github_visible`, `marketing_priority` e `website_status` não são publicados.

O frontend consome `data/projects.json` para os projetos e `data/stack-inventory.json` para a matriz profissional. O builder gera a matriz a partir do arquivo de perfil e do manifesto `docs/README_STACK.json`, vinculando evidências somente a projetos públicos do catálogo. O workflow valida a allowlist exata do artefato e não publica produção em pull requests.
