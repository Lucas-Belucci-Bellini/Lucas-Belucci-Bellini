# Guia operacional — Portfólio V2

> Estado: guia operacional inicial. Os comandos de build/teste abaixo são um roteiro de validação; não representam testes já executados. Confirme os scripts reais em `package.json`, `pyproject.toml` ou documentação do repositório antes de executá-los.

## 1. Pré-requisitos

- Git e acesso ao repositório.
- Python 3, caso os scripts `scripts/update_profile.py` e `scripts/build_portfolio.py` continuem sendo a implementação canônica.
- Node.js/npm somente se a branch integrada introduzir ferramentas JavaScript que os exijam.
- Não copie tokens, arquivos `.env`, credenciais nem dados de repositórios privados para o diretório público do site.

## 2. Execução local

1. Clone o repositório e entre na pasta:
   ```bash
   git clone https://github.com/Lucas-Belucci-Bellini/Lucas-Belucci-Bellini.git
   cd Lucas-Belucci-Bellini
   ```
2. Busque a branch de integração e crie uma branch de trabalho:
   ```bash
   git fetch origin
   git switch --track origin/feat/profile-readme-website-v2
   ```
   Se a branch não estiver disponível remotamente, interrompa e confirme o nome/base com a equipe; não substitua silenciosamente por `main`.
3. Inspecione os comandos e contratos reais antes de rodar qualquer script:
   ```bash
   git status --short
   git branch --show-current
   git ls-files
   ```
4. Instale apenas dependências declaradas pelo projeto. Não invente um comando de instalação sem confirmar o gerenciador e os arquivos de lock.
5. Execute os validadores e o build que estiverem realmente definidos no repositório. Registre comando, código de saída, commit testado e artefato produzido.
6. Sirva localmente o diretório de saída gerado usando um servidor estático disponível no ambiente; valide rotas, assets e navegação no navegador.

## 3. Fonte de dados e catálogo

- Trate o catálogo documentado pelo projeto como fonte canônica. Antes de editar, leia `docs/README_DATA.md`, `scripts/update_profile.py`, `scripts/build_portfolio.py` e as Issues #23–#25, caso esses caminhos existam na branch integrada.
- Preserve os marcadores gerados do README. Não edite manualmente o conteúdo entre marcadores que o gerador controla.
- Exiba somente metadados permitidos e públicos. Nunca publique código, conteúdo interno, secrets, tokens, caminhos locais ou descrições privadas.
- Use uma allowlist explícita de campos públicos. Rejeite URLs malformadas e não apresente URL como site ativo sem verificação.
- Ao alterar o catálogo: valide o formato, gere o README/site pelo fluxo oficial, revise o diff e confirme que nenhum projeto privado ou campo inesperado entrou no artefato.
- Catálogo vazio ou inválido deve produzir erro explícito, não uma publicação aparentemente bem-sucedida.

## 4. Pipeline README → catálogo → site → build → deploy

1. **Dados:** a fonte canônica fornece apenas campos aprovados para apresentação.
2. **README:** o gerador atualiza somente as regiões delimitadas por marcadores conhecidos; alterações manuais fora delas devem ser preservadas.
3. **Site:** a página consome os dados públicos e renderiza os cartões/filtros sem inserir conteúdo não confiável como HTML.
4. **Build:** o gerador valida entradas, limpa/recria a saída e produz artefatos determinísticos; confirme os nomes e diretórios reais na implementação.
5. **CI:** workflows validam código e artefatos em pull requests. Permissões devem ser mínimas e segredos só devem ser usados quando estritamente necessários.
6. **Deploy:** a publicação só é considerada concluída após workflow bem-sucedido, URL configurada confirmada e inspeção manual do site publicado.

Não presuma que GitHub Pages, domínio personalizado, secrets, permissões ou URL já estejam configurados. Verifique Settings → Pages, os workflows e o resultado da execução antes de anunciar publicação.

## 5. Checklist antes de abrir PR

- [ ] Branch criada a partir de `feat/profile-readme-website-v2`.
- [ ] Nenhuma alteração fora do escopo da tarefa.
- [ ] README gerado pelo processo oficial e diff revisado.
- [ ] Catálogo contém apenas metadados autorizados.
- [ ] Build não carrega arquivos inesperados para a saída.
- [ ] Testes/validadores reais executados e resultados registrados.
- [ ] Navegação por teclado, foco visível, skip link e contraste revisados.
- [ ] Layout verificado em largura móvel e desktop.
- [ ] Links internos/externos verificados; links externos abrem de forma segura.
- [ ] Sem erros de console, imagens quebradas ou estados de erro invisíveis.
- [ ] Sem segredos, arquivos `.env`, dados privados ou conteúdo interno no diff.
- [ ] PR descreve mudanças, comandos executados, limitações e evidências.

## 6. Troubleshooting

| Sintoma | Diagnóstico e ação |
|---|---|
| Branch de integração não encontrada | Rode `git fetch origin`; confirme o nome exato e não use `main` como substituto sem autorização. |
| Build não encontra catálogo | Confirme diretório de trabalho, caminhos relativos e existência do arquivo fonte; falhe explicitamente em vez de publicar saída antiga. |
| README perde edições | Confira os marcadores gerados e limite o gerador às regiões de sua responsabilidade. |
| Projeto privado aparece no site | Bloqueie o release, remova a exposição na fonte/allowlist e revise todos os artefatos e logs públicos. |
| URL de projeto não funciona | Marque como não verificada ou remova o link; não rotule como site ativo sem validação. |
| Filtros não retornam resultados | Verifique schema, normalização de categoria/status e estado vazio acessível. |
| Workflow falha | Leia logs públicos do job, valide dependências/permissions e reproduza localmente; nunca imprima secrets para depurar. |
| Pages retorna 404 | Confirme source/branch/diretório de publicação, artefato, base path e URL indicada nas configurações do repositório. |
| CSS/JS não carrega após deploy | Confira caminhos relativos, capitalização dos nomes, base path e lista de arquivos do artefato. |

## 7. Release e manutenção

1. Integre as frentes somente após revisão de PRs e resolução de conflitos.
2. Execute a matriz de aceitação e anexe logs ou links de execução.
3. Gere os artefatos a partir de uma árvore limpa e confira o diff.
4. Revise manualmente o README renderizado no GitHub e o site em desktop/mobile.
5. Publique somente após aprovação dos critérios obrigatórios e configuração explícita de hospedagem.
6. Após publicar, teste a URL real, navegação, assets, links e console.
7. Registre commit publicado, execução de CI, URL e procedimento de rollback. Não apague branches de trabalho nem faça merge em `main` sem aprovação.
