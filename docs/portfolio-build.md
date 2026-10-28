# Contrato de build estático e segurança dos dados — V2

A fonte editorial canônica continua sendo `docs/project-catalog.json`, gerada por `scripts/update_profile.py`. Não edite o catálogo gerado manualmente; altere os manifestos editoriais ou a lógica do gerador apenas quando houver justificativa e evidência. O builder consome o catálogo, mas nunca o copia para o artefato público.

## Allowlist de publicação

O artefato `dist/` deve conter exatamente estes seis arquivos:

- `index.html`
- `assets/site.css`
- `assets/site.js`
- `assets/favicon.svg`
- `data/projects.json`
- `.nojekyll`

O JSON público tem o schema `lucas-belucci-bellini/public-portfolio@1` e contém somente `schema`, `project_count` e `projects`. Cada projeto publicado pode conter somente `name`, `slug`, `description`, `category`, `category_label`, `status`, `github` e `website` (este último opcional). Campos editoriais/de controle como `repository`, `private`, `github_visible`, `marketing_priority`, `website_status`, `website_http_status`, `website_source`, `website_declared`, CTAs e metadados internos nunca são serializados.

## Regras de validação

- Schema de entrada: `lucas-belucci-bellini/project-catalog@1`; campos raiz inesperados são rejeitados.
- `categories` e `projects` devem ser arrays não vazios; categorias devem ser strings não vazias e únicas.
- Projetos privados (`private: true`) e não visíveis (`github_visible !== true`) não são publicados. Um projeto elegível precisa ter `private: false` explícito e todos os campos obrigatórios.
- Slugs devem corresponder a `^[a-z0-9]+(?:-[a-z0-9]+)*$` e ser únicos.
- Categorias e status devem pertencer às listas declaradas/permitidas.
- Link principal só é aceito em HTTPS para `github.com` ou `www.github.com`; URLs inválidas fazem o cartão ser excluído.
- Website só é publicado com `website_status: "verified"` e URL HTTPS segura. URLs com credenciais, hosts locais, IPs não globais, espaços ou portas não padrão são rejeitadas.
- Saída é ordenada e serializada com chaves estáveis, UTF-8 e newline final. O builder exige que o destino seja exatamente `<root>/dist`, rejeita `dist/` como symlink e valida a allowlist do estágio antes de substituir a saída.
- Arquivos extras no repositório não são copiados. O catálogo bruto nunca deve ser disponibilizado no site.

## Reprodução local

Na raiz do repositório, com Python 3:

```bash
python -m py_compile scripts/build_portfolio.py
python -m unittest discover -s tests -p "test_build_portfolio.py" -v
python scripts/build_portfolio.py
```

Inspecione o resultado com:

```bash
python -c "import json; print(json.load(open('dist/data/projects.json', encoding='utf-8')))"
python -c "from pathlib import Path; print('\n'.join(sorted(p.relative_to('dist').as_posix() for p in Path('dist').rglob('*') if p.is_file())))"
```

O teste negativo cobre schema/campos raiz inesperados, dados obrigatórios ausentes, projetos privados/não visíveis, slugs malformados/duplicados, destinos/URLs inválidos, website sem verificação, assets ausentes, arquivos extras e saída fora de `dist/`. Não considere build/testes aprovados até executá-los no ambiente local ou no CI da PR. Nenhum merge ou deploy é realizado por este procedimento.
