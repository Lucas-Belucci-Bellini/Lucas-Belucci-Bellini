# Contrato de integração: README e site V2

- `README.md` continua sendo a apresentação compacta do perfil no GitHub.
- O site V2 é a experiência interativa de navegação e busca de projetos.
- `docs/project-catalog.json` continua sendo a fonte editorial gerada pelo gerador existente; não criar uma segunda lista manual.
- `docs/README_FEATURED.json`, `docs/README_SITES.json`, `docs/README_STACK.json` e `docs/README_EXCLUDED.json` mantêm suas responsabilidades editoriais.

## Contrato de build

O navegador consome `data/projects.json` dentro do artefato estático, nunca `docs/project-catalog.json` diretamente. O catálogo editorial é transformado pelo builder e os campos internos são removidos antes do deploy. Websites só entram no artefato se a fonte marcar `website_status: verified` e o endereço passar pela validação HTTPS. A lista exata de arquivos e campos permitidos está em `docs/portfolio-build.md`.

Não adicionar o URL do portfólio ao README como publicado até que o GitHub Pages tenha um deploy real bem-sucedido e a URL seja verificada.
