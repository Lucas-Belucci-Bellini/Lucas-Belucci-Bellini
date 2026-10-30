# Contrato de integração: README e site V2

- README.md é a apresentação compacta do perfil no GitHub.
- docs/PROFILE-ARCHIVE.md é o arquivo detalhado gerado automaticamente.
- scripts/update_profile.py atualiza o arquivo de arquivo, sem expandir o README público.
- O site Dark Legion é a experiência interativa de navegação e busca de projetos.
- docs/project-catalog.json continua sendo a fonte editorial gerada; contém somente projetos públicos. O campo counts.private é apenas agregado e não inclui nomes, descrições, URLs ou stacks privados. Não criar uma segunda lista manual.
- docs/README_FEATURED.json, docs/README_SITES.json, docs/README_STACK.json e docs/README_EXCLUDED.json mantêm suas responsabilidades editoriais.

## Contrato de build

O navegador consome data/projects.json dentro do artefato estático, nunca docs/project-catalog.json diretamente. O catálogo editorial é transformado pelo builder e os campos internos são removidos antes do deploy. Websites só entram no artefato se a fonte marcar website_status: verified e o endereço passar pela validação HTTPS.

## Compatibilidade do gerador

O gerador Python seleciona docs/PROFILE-ARCHIVE.md quando o arquivo existe e mantém README.md como fallback para fixtures antigos. Os validadores de links, exclusões, badges e identidade visual verificam o arquivo detalhado; a validação de exclusões também examina o README compacto.

O renderer Rust já reconhece docs/PROFILE-ARCHIVE.md e mantém fallback para fixtures legadas. O publicador profile-core continua protegido por PROFILE_README_ARCHIVE_MODE=ready; só habilite essa variável depois que CI e os testes de paridade estiverem verdes e a publicação agendada for validada.

Não adicionar o URL do portfólio ao README como publicado até que o deploy real exista e a URL seja verificada.


## Política de divulgação de projetos privados

O perfil pode apresentar somente:
- contagem agregada de projetos privados;
- categorias amplas de finalidade, agregadas por categoria e sem associação a nomes;
- nomes de linguagens agregados, sem relacioná-los a um repositório específico.

É proibido publicar nomes ou URLs de repositórios privados, descrições individuais, status, stack por projeto, dependências, arquitetura, funcionalidades internas, estrutura de arquivos, código, issues, secrets, credenciais ou dados de configuração. O catálogo JSON do portal continua contendo somente projetos públicos. Os metadados privados são usados apenas localmente para calcular categorias e nomes de linguagens agregados; nenhum arquivo privado é lido.


## Política de divulgação de projetos privados

O perfil pode apresentar somente:
- contagem agregada de projetos privados;
- categorias amplas de finalidade, agregadas por categoria e sem associação a nomes;
- nomes de linguagens agregados, sem relacioná-los a um repositório específico.

É proibido publicar nomes ou URLs de repositórios privados, descrições individuais, status, stack por projeto, dependências, arquitetura, funcionalidades internas, estrutura de arquivos, código, issues, credenciais ou dados de configuração. O catálogo JSON do portal continua contendo somente projetos públicos. Os metadados privados são usados apenas localmente para calcular categorias e nomes de linguagens agregados; nenhum arquivo privado é lido.
