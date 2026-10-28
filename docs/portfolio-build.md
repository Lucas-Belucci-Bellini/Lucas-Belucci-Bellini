# Build estático do portfólio V2

Execute na raiz do repositório:

```sh
python scripts/build_portfolio.py
# Saída alternativa para auditoria:
python scripts/build_portfolio.py --out /tmp/portfolio-dist
```

Execute os testes de regressão da pipeline:

```sh
python -m unittest discover -s tests -p "test_*.py" -v
```

Os testes cobrem o allowlist público, exclusão de projetos privados/não visíveis, remoção de websites inválidos, saída determinística, limpeza de arquivos antigos e erros de schema/slugs duplicados.

A fonte editorial é `docs/project-catalog.json`, gerado por `scripts/update_profile.py`. Não edite o catálogo à mão; ajuste os manifestos editoriais e rode o gerador existente.

## Arquivos publicados

- `dist/index.html`
- `dist/assets/site.css`
- `dist/assets/site.js`
- `dist/assets/favicon.svg`
- `dist/data/projects.json`
- `dist/.nojekyll` (marcador vazio para hospedagens estáticas compatíveis)

O build não copia o repositório, README, scripts, manifestos, relatórios internos ou arquivos de configuração para `dist/`.

## Contrato de dados públicos

O JSON público permite apenas `name`, `slug`, `description`, `category`, `category_label`, `status`, `github` e `website` opcional. O campo interno `repository` e metadados de origem/verificação/prioridade não são serializados. Projetos privados e projetos sem visibilidade pública explícita são excluídos. Links GitHub que não sejam HTTPS válidos excluem o cartão; um website inválido é descartado sem remover o link GitHub válido. URLs com credenciais, whitespace, localhost ou domínio `.local` são rejeitadas.

O script valida versão do schema, arrays, campos obrigatórios, categoria, status, slugs únicos/seguros e URLs. Ordenação e serialização JSON são estáveis (chaves ordenadas, UTF-8, indentação fixa e newline final). Erros são reportados em stderr e retornam código 2. Cada build cria uma árvore temporária e substitui `dist/` por apenas os arquivos permitidos.

Build local bem-sucedido não comprova deploy nem publicação em produção.
