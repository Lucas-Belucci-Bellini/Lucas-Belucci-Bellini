# Build estático do portfólio V2

A fonte editorial continua sendo `docs/project-catalog.json`, gerada por `scripts/update_profile.py`. Não mantenha uma segunda lista manual concorrente: ajuste os manifestos editoriais e execute o gerador existente.

## Comandos locais

```bash
python -m py_compile scripts/build_portfolio.py
python -m unittest discover -s tests -v
python scripts/build_portfolio.py
```

O builder valida o schema `lucas-belucci-bellini/project-catalog@1`, filtra projetos privados/não visíveis, valida slugs e URLs HTTPS, e só publica websites marcados como verificados. A saída é determinística e limitada a `dist/`.

## Artefato público

Execute também `python -m json.tool docs/project-catalog.json >/dev/null` para validar a sintaxe do catálogo. O workflow de CI usa o mesmo comando de descoberta de testes mostrado acima, antes de construir o artefato. Para reproduzir a validação completa do artefato, execute o builder e confira os arquivos listados abaixo.\n\nO diretório `dist/` contém exatamente `index.html`, `assets/site.css`, `assets/site.js`, `assets/favicon.svg`, `data/projects.json` e `.nojekyll`. O JSON público contém somente `name`, `slug`, `description`, `category`, `category_label`, `status`, `github` e, quando verificado, `website`. Campos de controle como `repository`, `private`, `github_visible`, `marketing_priority` e `website_status` não são publicados.

O frontend consome exclusivamente `data/projects.json`, nunca o catálogo editorial bruto. `tests/test_portfolio_contract.py` verifica o contrato do JSON real gerado, a allowlist de campos, a ordenação determinística, a contagem, os URLs e os casos de privacidade/campos ausentes. O workflow valida a allowlist exata do artefato e não publica produção em pull requests.\n\n## Reproduzir a suíte\n\nUse Python 3.12 e execute, na raiz do repositório:\n\n```bash\npython -m py_compile scripts/build_portfolio.py\npython -m json.tool docs/project-catalog.json >/dev/null\npython -m unittest discover -s tests -v\npython scripts/build_portfolio.py --out /tmp/portfolio-dist\n```\n\nOs comandos acima são os comandos previstos para CI. Registre como executados somente os comandos que você de fato rodou no ambiente local; o commit/PR por si só não comprova execução.
