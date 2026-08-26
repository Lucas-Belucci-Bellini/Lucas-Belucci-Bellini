# README Style Kit — tutorial de uso

Este kit é um ponto de partida reutilizável para criar um README de perfil com estética **Baluarte / Spartan / Field Manual**: banners escuros, ouro como acento, status em verde, títulos de terminal, badges compactos, missões e footer editorial. O kit é público e foi separado do perfil original para não carregar nomes, projetos, links ou informações privadas.

> **Kit público:** [Lucas-Belucci-Bellini/Lucas-Belucci-Bellini — branch `template/readme-style-kit`](https://github.com/Lucas-Belucci-Bellini/Lucas-Belucci-Bellini/tree/template/readme-style-kit). Neste repositório você encontra os templates, os snippets e este tutorial explicando como tudo funciona.

## 1. Escolha o ponto de partida

Use o template completo quando quiser um perfil com narrativa, Arsenal categorizado, missões, catálogo recolhido e contato. Use o template mínimo quando quiser uma página rápida com banner, stack, dois projetos e links principais.

| Arquivo | Quando usar |
|:---|:---|
| [`templates/README.template.md`](templates/README.template.md) | Perfil completo com identidade, build log, cinco categorias de Arsenal, missões e observabilidade opcional. |
| [`templates/README.minimal.template.md`](templates/README.minimal.template.md) | Perfil curto, com foco em leitura rápida e poucos projetos. |
| [`templates/profile-config.example.json`](templates/profile-config.example.json) | Ficha de preparação com identidade, categorias, projetos e fontes das métricas. |
| [`templates/section-snippets.md`](templates/section-snippets.md) | Trechos independentes para copiar apenas uma seção. |
| [`templates/workflows/profile-traffic.example.yml`](templates/workflows/profile-traffic.example.yml) | Exemplo opcional de Action para consultar views e clones de repositórios. |

## 2. Copie o template para o repositório de perfil

O README de perfil é publicado pelo repositório que tem exatamente o mesmo nome do usuário. Você pode copiar o arquivo pela interface do GitHub ou usar Git:

```bash
git clone https://github.com/Lucas-Belucci-Bellini/Lucas-Belucci-Bellini.git
cd Lucas-Belucci-Bellini
git switch template/readme-style-kit
cp templates/README.template.md /caminho/do/seu-repositorio-de-perfil/README.md
```

Depois, faça a personalização em uma branch própria. Não edite diretamente `main` até revisar o diff e executar o checklist deste tutorial.

## 3. Preencha a ficha antes do Markdown

Comece por [`profile-config.example.json`](templates/profile-config.example.json). Ele funciona como uma ficha editorial: identidade, linguagens, categorias de ferramentas, missões, fontes e métricas opcionais.

Substitua todos os placeholders como `[SEU NOME]`, `[SEU_USUARIO]`, `[SUA_CIDADE]`, `[SUA_ESPECIALIDADE]`, `[REPO]`, `[SEU_SITE]` e `[SEU_PERFIL]`. Troque também descrições genéricas por fatos confirmados nos READMEs públicos dos projetos.

| Campo | Regra de qualidade |
|:---|:---|
| Identidade | Use uma frase curta, verdadeira e compreensível fora do seu contexto. |
| Linguagens | Liste apenas linguagens detectadas em repositórios reais ou que você decidiu declarar manualmente. |
| Ferramentas | Separe linguagens, frameworks, infraestrutura, IA/conhecimento e hardware/simulação. |
| Missões | Escolha projetos públicos com README, demo ou evidência verificável. |
| Sites | Teste a URL e indique o status observado; não chame uma URL instável de “online”. |
| Contadores | Informe a fonte, a janela temporal e se a métrica é oficial ou externa. |

## 4. Organize o Arsenal em cinco camadas

A organização recomendada é:

1. **Linguagens:** a camada pode ser calculada por linguagens detectadas nos repositórios públicos.
2. **Frameworks & Web:** bibliotecas, frameworks e ferramentas de interface.
3. **Infraestrutura & DevOps:** Git, CI, sistemas, runtimes, containers e serviços de dados.
4. **IA & Conhecimento:** APIs de agentes, MCP, ferramentas de conhecimento e automação assistida.
5. **Hardware & Simulação:** eletrônica, jogos, motores, CAD, simuladores e fundamentos digitais.

A separação é editorial e não deve inventar experiência. Quando uma ferramenta não estiver evidenciada, remova o ícone ou explique a fonte manualmente.

## 5. Use badges sem quebrar caracteres

No texto alternativo, escreva o nome legível: `C#`, `PL/pgSQL`, `C++` e assim por diante. Na URL, o Shields.io pode precisar de escape, como `%23` para `#` e `%2F` para `/`. O escape pertence à URL, não ao label exibido.

```markdown
[![C#](https://img.shields.io/badge/C%23-uso%20público-239120?style=flat-square&labelColor=0e0c16)](https://github.com/SEU_USUARIO?tab=repositories&language=C%23)
[![PL/pgSQL](https://img.shields.io/badge/PL%2FpgSQL-uso%20público-336791?style=flat-square&labelColor=0e0c16)](https://github.com/SEU_USUARIO?tab=repositories&language=PLpgSQL)
```

Mantenha os badges em linhas curtas. Em telas estreitas, quatro a seis badges por linha normalmente são mais legíveis do que uma única fileira extensa.

## 6. Métricas avançadas que podem entrar no README

Use somente as métricas que ajudam a contar a história do portfólio:

| Métrica | Fonte | Limite/observação |
|:---|:---|:---|
| Commits semanais | REST `stats/commit_activity` | Último ano; útil para tendência. |
| Adições e deleções | REST `stats/code_frequency` | Pode retornar 422 em repositórios com 10.000 commits ou mais. |
| Participação autor/equipe | REST `stats/participation` | Últimas 52 semanas; separa proprietário e total. |
| Horário de atividade | REST `stats/punch_card` | Mostra dia/hora dos commits, sem inferir produtividade. |
| Última atividade | GraphQL `lastContributionDate` | Métrica de saúde; requer o cabeçalho beta correspondente. |
| Commits na branch padrão | GraphQL `commitCount` | Contagem monotonicamente crescente; também está em beta pública. |
| Stars, forks e issues abertas | REST/GraphQL de repositório | Mostrar por projeto destacado, não como “qualidade” automática. |
| Releases e última publicação | REST/GraphQL de releases | Útil para demonstrar cadência de entrega. |
| Views, visitantes e clones | REST de tráfego do repositório | Exige acesso de escrita e cobre somente os últimos 14 dias. |

Inclua uma nota com data de coleta, janela e fonte. Não misture um contador oficial do GitHub com um contador de terceiros sem rotular a diferença.

## 7. Adicione visitantes com transparência

O GitHub não documenta um contador oficial e estável de visitas ao perfil pessoal equivalente ao tráfego de um repositório. Para um indicador visual simples, você pode usar um contador externo:

```markdown
[![External profile view counter](https://komarev.com/ghpvc/?username=SEU_USUARIO&style=for-the-badge&color=d4a24e&labelColor=0e0c16&label=EXTERNAL+PROFILE+VIEWS)](https://github.com/SEU_USUARIO)
```

O badge registra carregamentos do próprio contador e deve ser descrito como **indicador externo**, não como analytics oficial do perfil. Para métricas oficiais, um workflow com token armazenado em `Secrets` pode consultar `traffic/views` e `traffic/clones` de repositórios em que você tem acesso de escrita, mas esses dados duram apenas 14 dias. Não publique o token nem os dados de repositórios privados.

## 8. Automatize somente o que tem fonte e contrato

Para um README dinâmico, marque blocos com `START/END` e faça o workflow alterar somente esses trechos. Mantenha manifests editoriais fora do README e execute validações em cada pull request:

```yaml
on:
  pull_request:
    paths:
      - "README.md"
      - "scripts/**"
      - ".github/workflows/**"
```

O workflow deve compilar scripts, validar placeholders, conferir exclusões, testar links críticos e verificar que o diff fora dos blocos dinâmicos está vazio. Tokens entram apenas em **Settings → Secrets and variables → Actions**. Para um exemplo pronto de tráfego oficial de repositórios, veja [`templates/workflows/profile-traffic.example.yml`](templates/workflows/profile-traffic.example.yml); revise a lista de repositórios e os marcadores antes de habilitá-lo.

## 9. Checklist de qualidade antes de publicar

```bash
# Placeholders restantes
grep -RInE '\[SEU_|\[SUA_|\[REPO\]|\[COLCHETES\]' README.md

# Espaços e alterações suspeitas
git diff --check
git diff -- README.md

# Padrões que nunca devem aparecer no commit
grep -nE 'sk-|ghp_|github_pat_|BEGIN .* PRIVATE KEY|postgresql://|mysql://' README.md || true
```

Abra a prévia renderizada no GitHub e confirme que banners, títulos, ícones, tabelas e badges carregam. Se um serviço externo falhar, mantenha um fallback textual próximo da imagem e não trate o recurso como obrigatório.

## 10. Proteja a branch principal

Depois de confirmar o nome do check no GitHub Actions, abra **Settings → Branches → Add branch protection rule**, use o padrão `main`, marque **Require status checks to pass before merging**, selecione o check do workflow e salve. Ative também **Require a pull request before merging** se quiser impedir pushes diretos.

## 11. Como adaptar e creditar

Você pode adaptar cores, textos, categorias e componentes. Preserve os créditos quando fizer uma cópia direta e remova tudo que não representa seu trabalho. O kit é um ponto de partida visual; a prioridade é manter **legibilidade, evidência, privacidade e manutenção simples**.

## Referências

- [GitHub Actions](https://docs.github.com/en/actions)
- [GitHub REST API — repository statistics](https://docs.github.com/en/rest/metrics/statistics)
- [GitHub REST API — repository traffic](https://docs.github.com/en/rest/metrics/traffic)
- [GitHub GraphQL API](https://docs.github.com/graphql)
- [Capsule Render](https://github.com/kyeongrok-lee/capsule-render)
- [readme-typing-svg](https://github.com/DenverCoder1/readme-typing-svg)
- [Skill Icons](https://github.com/tandpfun/skill-icons)
- [Shields.io](https://shields.io/)
- [Komarev GitHub Profile Views Counter](https://github.com/antonkomarev/github-profile-views-counter)

## 12. Mantenha o template atualizado

Trate o template como um produto pequeno, não como uma lista infinita de logos. Faça uma revisão mensal ou a cada mudança relevante de stack. Primeiro atualize o manifesto de configuração; depois confira a documentação oficial da tecnologia, o identificador aceito pelo Skill Icons e a evidência em um projeto público.

| Etapa | Pergunta de controle |
|:---|:---|
| Descoberta | A tecnologia aparece em um projeto real ou foi apenas testada uma vez? |
| Evidência | Existe README, manifest, configuração, release ou demo público que sustente a inclusão? |
| Categoria | Ela pertence a linguagem, framework/web, infraestrutura/DevOps, IA/conhecimento ou hardware/simulação? |
| Badge | O ícone, label e URL carregam sem escapes visíveis ou endpoint depreciado? |
| Impacto | A nova linha melhora a leitura ou apenas aumenta ruído visual? |
| Regressão | Os testes, links e marcadores dinâmicos continuam passando? |

Adicione uma tecnologia somente depois de responder essas perguntas. Para frameworks com versões frequentes, prefira o nome do framework sem fixar uma versão no badge; registre a versão no README do projeto ou em um changelog. Remova tecnologias abandonadas, duplicadas ou sem evidência. Quando uma ferramenta muda de nome, mantenha uma nota de migração no changelog em vez de exibir dois badges equivalentes.

No processo de contribuição, abra uma branch para cada atualização, revise o diff renderizado, rode o validador de badges e peça revisão antes do merge. Se o template mudar a semântica das métricas ou dos marcadores, atualize também `profile-config.example.json`, `section-snippets.md` e este tutorial. Assim, quem copiar o kit recebe sempre a mesma regra em todos os pontos de entrada.
