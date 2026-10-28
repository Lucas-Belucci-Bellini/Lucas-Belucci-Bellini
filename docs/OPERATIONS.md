# Guia operacional do portfólio

Este guia descreve os comandos confirmados no workflow `.github/workflows/update-profile.yml` da branch de referência. Execute comandos a partir da raiz do repositório. Os comandos abaixo foram identificados no workflow; a execução local ainda depende das versões e dependências disponíveis na máquina.

## Pré-requisitos

- Git e Python 3.12 (a versão configurada no workflow).
- Acesso ao repositório e, para atualizar o inventário que consulta a API do GitHub, um token com permissões adequadas. O workflow lê o secret `PROFILE_README_TOKEN`; nunca copie tokens para arquivos ou logs.
- Não é necessário `npm` para os testes/validações documentados pelo workflow: os comandos configurados usam Python.

## Testes e validações

O workflow executa:

```bash
python3 -m unittest discover -s tests
python3 -m py_compile scripts/update_profile.py scripts/project_catalog.py scripts/check_websites.py scripts/validate_dynamic_sections.py scripts/validate_exclusions.py scripts/validate_project_links.py
python3 scripts/validate_exclusions.py
python3 scripts/validate_project_links.py
python3 -m json.tool docs/README_SITES.json >/dev/null
python3 -m json.tool docs/README_FEATURED.json >/dev/null
python3 -m json.tool docs/README_STACK.json >/dev/null
python3 -m json.tool docs/README_EXCLUDED.json >/dev/null
python3 -m json.tool docs/project-catalog.json >/dev/null
```

A validação de mudanças nas seções geradas precisa de uma cópia anterior do README, como faz o workflow:

```bash
cp README.md /tmp/README.before.md
python3 scripts/update_profile.py --write
python3 scripts/validate_dynamic_sections.py --before /tmp/README.before.md --after README.md
```

A última sequência escreve no README e nos artefatos gerados. Rode-a somente em uma branch de trabalho e revise `git diff` antes de commitar. O gerador pode precisar de acesso à API do GitHub; não interprete um refresh ignorado como atualização bem-sucedida.

## Fluxo de trabalho com Git e PR

```bash
git status --short
git switch feat/profile-v2-consolidated
git pull --ff-only
git switch -c docs/minha-alteracao
# edite os arquivos necessários
git diff --check
git diff
git add docs/ README.md
git commit -m "docs: describe profile operations"
git push -u origin docs/minha-alteracao
```

Os comandos de Git acima são exemplos do fluxo esperado; confirme os nomes de branch e remoto antes de executá-los. Abra um PR para a branch-alvo aprovada. Não faça merge até que os checks e a revisão humana estejam concluídos.

Na revisão do PR:
1. Confirme que a base é a branch correta e que o diff contém apenas mudanças intencionais.
2. Aguarde os checks obrigatórios e leia os logs do primeiro passo que falhou.
3. Verifique se o workflow de atualização não publicou conteúdo inesperado no README ou nos artefatos.
4. Faça a revisão manual de privacidade, URLs e descrições.
5. Se a branch-base mudou, atualize sua branch e repita os checks.

## Troubleshooting

- **Testes falham:** rode `python3 -m unittest discover -s tests` e corrija a primeira falha reproduzível antes de alterar snapshots.
- **JSON inválido:** use o `python3 -m json.tool arquivo.json` correspondente para localizar erro de sintaxe.
- **Links recusados ou expirados:** confira a URL declarada e a saída de `scripts/validate_project_links.py`; não invente URL substituta a partir do nome do repositório.
- **Refresh sem atualização:** confira o secret `PROFILE_README_TOKEN`, permissões de leitura de metadados e logs do workflow. O workflow declara que falha ao final se o refresh for ignorado por ausência do token.
- **Diff gerado muito grande:** não faça commit automático. Inspecione as seções geradas e compare com o README anterior.
- **Branch conflitada:** preserve suas alterações, atualize a base conscientemente e resolva conflitos por seção. Não sobrescreva arquivos gerados sem compreender a fonte editorial.

## Checklist de release

- [ ] PR direcionado à base correta; nenhuma alteração direta na branch protegida.
- [ ] `git diff --check` sem erros.
- [ ] Suíte `unittest` aprovada.
- [ ] Validações Python e JSON aprovadas.
- [ ] Marcadores das seções geradas presentes no README.
- [ ] Catálogo conferido contra as fontes editoriais.
- [ ] URLs públicas revisadas e nenhum destino deduzido por convenção.
- [ ] Conteúdo privado, tokens, dados pessoais desnecessários e detalhes de repositórios privados não foram expostos.
- [ ] Checks do PR verdes e revisão manual concluída.
- [ ] Aprovação explícita antes do merge.

## Limites

Este guia não promete que todo site esteja sempre disponível nem que cada comando funcione em qualquer ambiente local. O workflow e os scripts versionados são a referência operacional; valide novamente quando eles mudarem.
