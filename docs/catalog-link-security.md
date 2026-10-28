# Política de segurança para URLs do catálogo

O catálogo público é produzido por `scripts/build_portfolio.py` a partir de `docs/project-catalog.json`. Não crie uma lista paralela de projetos nem anuncie um website manualmente fora dos manifestos e do gerador existentes.

## URLs permitidas

- Links externos devem ser URLs absolutas com protocolo `https:`.
- Rejeitar protocolos como `javascript:`, `data:`, `file:`, `http:` e URLs relativas/protocol-relative.
- Rejeitar espaços/caracteres de controle, barras invertidas, credenciais embutidas e hosts locais (`localhost`, subdomínios `.localhost` e `.local`).
- Link de código só é renderizado quando aponta para `github.com` ou `www.github.com` e contém exatamente os segmentos de caminho de proprietário e repositório. Nomes de repositório com ponto são legítimos e devem ser aceitos.
- Validar tanto na geração do catálogo público quanto no navegador; a validação do navegador é defesa em profundidade, não substitui a validação de build.

## Sites e estado de verificação

Um website só pode ser publicado no JSON de saída quando `website_status` for `verified` e a URL passar na política HTTPS. A verificação de sintaxe/host não prova que o deployment está ativo; o estado `verified` deve vir do processo de verificação existente. Se a URL falhar ou não estiver verificada, omitir o campo `website`; manter o link GitHub válido.

## Renderização e links externos

- Inserir nomes, categorias, status e descrições do catálogo usando `textContent`/nós DOM, nunca interpolando conteúdo do catálogo em `innerHTML`.
- Definir `href` apenas após validação e não usar URLs do catálogo em atributos HTML montados por concatenação.
- Links externos que abrem nova aba devem usar `target="_blank"` com `rel="noopener noreferrer"`.
- Não interpretar descrição/categoria como HTML ou Markdown confiável. O texto é conteúdo simples.
- Falha de URL deve remover apenas a ação insegura; não deve ocultar um projeto público com link GitHub válido.

## Testes de regressão

Executar:

```bash
python -m py_compile scripts/build_portfolio.py
python -m unittest discover -s tests -p "test_build_portfolio.py" -v
python scripts/build_portfolio.py
```

Os testes devem cobrir protocolos perigosos, URLs HTTPS válidas, credenciais/hosts locais, campos de saída allowlisted e repositórios GitHub com pontos no nome.
