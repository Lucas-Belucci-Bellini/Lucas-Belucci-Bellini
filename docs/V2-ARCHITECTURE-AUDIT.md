# V2 — auditoria de arquitetura e integração

## Decisões consolidadas

1. `docs/project-catalog.json` é a fonte editorial existente e permanece gerada por `scripts/update_profile.py`.
2. `scripts/build_portfolio.py` é a fronteira de privacidade: o catálogo bruto nunca é publicado diretamente.
3. O navegador lê somente `data/projects.json`, criado dentro de `dist/`.
4. O artefato contém uma allowlist exata de seis arquivos e o JSON público possui uma allowlist explícita de campos.
5. URLs GitHub devem ser HTTPS em `github.com`/`www.github.com`; websites só são emitidos quando `website_status` é `verified` e a URL é HTTPS válida.
6. O workflow roda testes, build e validação do artefato em PRs. Apenas `push` ou execução manual na `main` pode publicar Pages; PRs nunca fazem deploy de produção.
7. A existência de build verde não significa que GitHub Pages já está configurado ou que a URL de produção foi validada.

## Gaps que exigem validação no GitHub

- Confirmar o resultado de Actions no PR consolidado após a criação.
- Validar visualmente navegação, busca, filtros, teclado, leitor de tela e layout móvel.
- Configurar/verificar GitHub Pages e registrar a URL somente depois de deploy real.
