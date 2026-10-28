# Testes do portfólio V2

## Requisitos

- Python 3.12 (o workflow de validação usa esta versão).
- Nenhuma dependência Python adicional para os testes do build: usam `unittest` da biblioteca padrão.

## Executar localmente

Na raiz do repositório:

```bash
python -m unittest tests.test_build_portfolio -v
python -m unittest discover -s tests -v
python -m py_compile .github/scripts/*.py scripts/*.py tests/*.py
python scripts/validate_language_badges.py
python scripts/validate_project_links.py
python scripts/validate_restored_style.py
```

No Windows, use `py -3.12 -m unittest discover -s tests -v` se o comando `python` não apontar para Python 3.12.

## Contratos cobertos

- Geração determinística da estrutura esperada em `dist/`: `index.html`, três assets, `data/projects.json` e `.nojekyll`.
- Exclusão de projetos privados e ocultos, e descarte de URLs GitHub não HTTPS válidas.
- Rejeição de esquemas HTTP/FTP e URLs HTTPS sem host.
- Falha explícita para JSON malformado, campo `projects` que não seja array, catálogo ausente e catálogo público vazio.
- Allowlist dos campos públicos: somente `name`, `description`, `category`, `category_label`, `status`, `github`, `github_visible`, `website_status` e `website` podem sair no JSON publicado.

Os testes usam um diretório temporário e um catálogo sintético; não chamam GitHub, sites hospedados nem serviços externos. O builder não foi alterado para acomodar os testes.
