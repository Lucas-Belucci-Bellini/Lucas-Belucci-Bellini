# Guia operacional — Portfólio V2

> Este guia registra os contratos observados nas branches consultadas e separa instruções de validações realmente executadas. Build, testes e deploy ainda precisam de execução com logs antes da aprovação de release.

## 1. Pré-requisitos

- Git e acesso ao repositório.
- Python 3.12 para o workflow de atualização do README; o builder do site usa apenas a biblioteca padrão do Python.
- Navegador moderno para revisão manual.
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
   Se a branch não estiver disponível remotamente, confirme o nome/base; não substitua silenciosamente por `main`.
3. Confira a árvore antes de validar:
   ```bash
   git status --short
   git branch --show-current
   ```
4. Depois que as branches de build/testes forem integradas e seus contratos alinhados, execute a partir da raiz:
   ```bash
   python3 scripts/build_portfolio.py --out /tmp/portfolio-dist
   python3 -m unittest discover -s tests
   ```
   Esses comandos são derivados dos arquivos inspecionados, mas **não foram executados nesta revisão**. No estado observado, os testes de build e o builder parecem incompatíveis; não interprete a suíte como aprovada até essa divergência ser resolvida.
5. Inspecione `/tmp/portfolio-dist/data/projects.json` e compare os arquivos gerados com a allowlist esperada. Sirva o diretório estático localmente e revise navegação, links, estados vazios e assets no navegador.

## 3. Fonte de dados e catálogo

- `docs/project-catalog.json` inclui uma nota de que é gerado por `scripts/update_profile.py`; não o edite manualmente como fonte de verdade.
- Consulte `docs/README_DATA.md` e os manifestos `docs/README_SITES.json`, `docs/README_FEATURED.json`, `docs/README_STACK.json` e `docs/README_EXCLUDED.json` antes de alterar os dados do README.
- Preserve os marcadores gerados do README. Não edite manualmente o conteúdo entre marcadores que o gerador controla.
- O builder observado usa schema `lucas-belucci-bellini/project-catalog@1` e uma allowlist explícita de campos de projeto. Ele exclui projetos com `private: true` e os que não têm `github_visible: true`; também rejeita projetos públicos sem um `private` booleano explícito.
- O builder valida slugs, categoria, status e URLs HTTPS. Uma URL de GitHub inválida faz o cartão ser omitido; uma URL de site inválida não é incluída no JSON público.
- O build deve falhar quando o catálogo está ausente, inválido ou não contém projetos públicos válidos. Revise a saída gerada antes de publicar.

## 4. Pipeline README → catálogo → site → build → deploy

1. **Dados do README:** `scripts/update_profile.py` consulta metadados do GitHub e utiliza manifestos em `docs/`; o catálogo `docs/project-catalog.json` é uma saída gerada.
2. **README:** `.github/workflows/update-profile.yml` atualiza diariamente ou manualmente o README e assets gerados. Esse workflow tem permissão `contents: write` e depende de `PROFILE_README_TOKEN` para executar a atualização; não confunda essa automação com deploy do site.
3. **Site:** `index.html`, `assets/site.css` e `assets/site.js` são entradas estáticas do site.
4. **Build:** `scripts/build_portfolio.py` copia somente os arquivos estáticos esperados, gera `data/projects.json` e `.nojekyll` dentro da pasta de saída. O padrão observado é `dist/`; use `--out` para escolher outra pasta.
5. **CI:** confirme que os workflows reais validam a branch e executam testes compatíveis com a implementação final. Não suponha que a existência do workflow do README valide o site.
6. **Deploy:** a publicação só é considerada concluída após workflow de deploy bem-sucedido, URL configurada confirmada e inspeção manual do site publicado.

Não presuma que GitHub Pages, domínio personalizado, secrets, permissões ou URL já estejam configurados. Verifique Settings → Pages, os workflows e o resultado da execução antes de anunciar publicação.

## 5. Checklist antes de abrir/atualizar PR

- [ ] Branch criada a partir de `feat/profile-readme-website-v2`.
- [ ] Nenhuma alteração fora do escopo da tarefa.
- [ ] Catálogo gerado pelo fluxo oficial e diff revisado.
- [ ] Builder e testes compartilham o mesmo contrato de entrada/saída.
- [ ] Build e testes executados; comandos, commit, logs e códigos de saída registrados.
- [ ] Artefato contém somente os arquivos esperados e campos públicos aprovados.
- [ ] Navegação por teclado, foco visível, skip link, contraste e reduced motion revisados.
- [ ] Layout verificado em largura móvel e desktop.
- [ ] Links internos/externos verificados; links externos abrem de forma segura.
- [ ] Sem erros de console, imagens quebradas ou estados de erro invisíveis.
- [ ] Sem segredos, arquivos `.env`, dados privados ou conteúdo interno no diff/artefato.
- [ ] PR descreve mudanças, comandos executados, limitações e evidências.

## 6. Troubleshooting

| Sintoma | Diagnóstico e ação |
|---|---|
| Branch de integração não encontrada | Rode `git fetch origin`; confirme o nome exato e não use `main` como substituto sem autorização. |
| Build não encontra catálogo | Confirme que `docs/project-catalog.json` existe e que o comando foi executado a partir da raiz correta. |
| Testes referenciam constantes ausentes ou schema divergente | Alinhe o contrato entre builder e testes; os testes observados esperam constantes/schema/exceções diferentes do script de build atual. |
| README perde edições | Confira os marcadores gerados e limite o gerador às regiões de sua responsabilidade. |
| Projeto privado aparece no site | Bloqueie o release, corrija fonte/allowlist e revise todos os artefatos e logs públicos. |
| URL de projeto não funciona | Marque como não verificada ou remova o link; não rotule como site ativo sem validação. |
| Workflow do README falha | Leia os logs públicos, valide dependências/permissões e confirme a configuração do secret sem imprimir seu valor. |
| Pages retorna 404 | Confirme source/branch/diretório de publicação, artefato, base path e URL indicada nas configurações do repositório. |
| CSS/JS não carrega após deploy | Confira caminhos relativos, capitalização dos nomes, base path e lista de arquivos do artefato. |

## 7. Release e manutenção

1. Integre as frentes somente após revisão dos PRs e resolução de conflitos.
2. Alinhe e execute a suíte de testes; atualize a matriz de aceitação com logs e commits.
3. Gere os artefatos a partir de uma árvore limpa e confira o diff/inventário de arquivos.
4. Revise manualmente o README renderizado no GitHub e o site em desktop/mobile.
5. Publique somente após aprovação dos critérios obrigatórios e configuração explícita de hospedagem.
6. Após publicar, teste a URL real, navegação, assets, links e console.
7. Registre commit publicado, execução de CI, URL e procedimento de rollback. Não apague branches de trabalho nem faça merge em `main` sem aprovação.
