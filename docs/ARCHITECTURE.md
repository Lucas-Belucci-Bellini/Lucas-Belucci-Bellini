# Arquitetura e contrato público

## Visão geral

O repositório é um perfil do GitHub gerado a partir de metadados de repositórios, manifestos editoriais e verificações de URL. A página pública principal é `README.md`. O workflow `.github/workflows/update-profile.yml` executa o gerador Python, a suíte de testes e validadores; quando autorizado e configurado, publica as mudanças geradas.

## Fontes e responsabilidades

- `docs/README_SITES.json`: mapa explícito de repositórios para URLs de site quando a URL não vem do campo `homepage` do GitHub. Não derive URLs a partir do nome de repositório.
- `docs/README_FEATURED.json`: seleção editorial e ordem/prioridade dos projetos destacados.
- `docs/README_STACK.json`: dados editoriais de tecnologias/stack.
- `docs/README_EXCLUDED.json`: exclusões editoriais do perfil.
- `scripts/update_profile.py`: orquestra a atualização do README, snapshot SVG e catálogo.
- `scripts/project_catalog.py`: funções de catálogo, apresentação e checagem de sites importadas pelo gerador.
- `scripts/validate_dynamic_sections.py`, `scripts/validate_exclusions.py`, `scripts/validate_project_links.py`: validam seções geradas, exclusões e links.
- `docs/project-catalog.json`: saída gerada. Seu próprio campo `note` orienta a não editar manualmente; altere as fontes ou o gerador.
- `assets/profile-snapshot.svg` e `assets/lang-stats.svg`: artefatos exibidos no README; o workflow exige que existam e que o snapshot seja consistente com a geração.

Consulte os arquivos versionados para confirmar nomes e responsabilidades antes de introduzir novos scripts. Não trate este resumo como inventário exaustivo de todos os arquivos.

## Contrato do catálogo público

O catálogo atualmente declara o identificador `lucas-belucci-bellini/project-catalog@1` e inclui `counts`, `categories` e uma coleção `projects`. Os registros incluem metadados como repositório, nome, slug, descrição, categoria, status, URL do GitHub, visibilidade e dados de website quando disponíveis. Campos opcionais podem variar conforme o resultado da descoberta e da checagem.

Use `docs/project-catalog.json` como amostra concreta do contrato atual. Não assuma que campos opcionais estarão sempre presentes nem altere o identificador do schema sem revisar consumidores, testes e documentação. O workflow valida a sintaxe JSON, mas isso não equivale a validação formal por JSON Schema.

## Privacidade e limites de publicação

- O gerador documenta que usa metadados e mapas de bytes de linguagens; não busca o conteúdo dos arquivos de repositórios privados.
- A visibilidade privada não autoriza publicar código, nomes de arquivos, estrutura interna, secrets, tokens ou conteúdo de `.env`.
- Descrições e outros metadados de repositórios podem ser públicos. Revise-os antes de publicar e mantenha apenas os campos necessários.
- Uma URL de site precisa vir do campo `homepage` ou de `docs/README_SITES.json`; o catálogo registra a origem e o resultado da checagem.
- A checagem HTTP é uma observação pontual, não garantia de disponibilidade futura, segurança, propriedade do domínio ou correção funcional do site.
- O manifesto `docs/README_SITES.json` funciona como allowlist editorial de URLs explícitas para repositórios cuja `homepage` não está preenchida. A `homepage` válida tem precedência sobre o manifesto. O validador `scripts/validate_project_links.py` exige HTTPS para sites publicados, status `verified` e impede site público em registros privados. Não adicione URLs sem evidência verificável e revisão humana.

## Atualização segura

1. Edite a fonte editorial correta em vez do artefato gerado.
2. Execute os testes e validadores descritos em [OPERATIONS.md](OPERATIONS.md).
3. Revise `git diff`, especialmente as seções delimitadas por marcadores `START/END`.
4. Verifique os links, os dados de visibilidade e a saída de `docs/project-catalog.json`.
5. Abra um PR e aguarde os checks e a revisão humana. Não faça merge automaticamente.

## Decisões documentadas

- O README é a apresentação humana; o catálogo JSON é a saída estruturada para consumidores.
- Manifestos editoriais separam escolhas de apresentação dos dados derivados da API.
- URLs não são inferidas a partir do nome do repositório.
- O catálogo gerado não deve ser editado manualmente.
- Um resultado HTTP bem-sucedido não deve ser descrito como garantia permanente de disponibilidade.
