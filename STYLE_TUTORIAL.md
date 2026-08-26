# README Style Kit — tutorial de uso

Este kit reproduz a identidade visual do README restaurado: **banner escuro**, acentos em **ouro**, títulos com efeito de digitação, badges compactos, blocos de missão, tabelas técnicas e footer com acabamento editorial. Ele foi separado em templates genéricos para que outra pessoa possa copiar a estrutura sem carregar nome, links, projetos ou informações privadas do perfil original.

## 1. Escolha o ponto de partida

Para um README completo, copie [`templates/README.template.md`](templates/README.template.md). Para uma versão curta, copie [`templates/README.minimal.template.md`](templates/README.minimal.template.md). O arquivo [`templates/profile-config.example.json`](templates/profile-config.example.json) funciona como um mapa de preenchimento: ele organiza identidade, linguagens, ferramentas e projetos antes de você editar o Markdown.

| Arquivo | Uso recomendado |
|:---|:---|
| `templates/README.template.md` | Perfil completo com identidade, build log, Arsenal, missões, mapa e contato. |
| `templates/README.minimal.template.md` | Perfil enxuto, com banner, stack, dois projetos e contato. |
| `templates/profile-config.example.json` | Checklist estruturado dos campos que precisam ser personalizados. |

## 2. Copie o template para seu repositório de perfil

O GitHub usa o README do repositório que tem exatamente o mesmo nome do seu usuário. Faça um fork deste repositório ou abra a branch do kit e copie o template escolhido para o repositório de perfil:

```bash
git clone https://github.com/Lucas-Belucci-Bellini/Lucas-Belucci-Bellini.git
cd Lucas-Belucci-Bellini
git switch template/readme-style-kit
cp templates/README.template.md /caminho/do/seu-repositorio-de-perfil/README.md
```

Se preferir não usar Git, abra o template no GitHub, selecione o conteúdo, crie ou edite o `README.md` do seu repositório de perfil e cole o modelo.

## 3. Substitua todos os placeholders

Procure por `[SEU NOME]`, `[SEU_USUARIO]`, `[SUA_CIDADE]`, `[SUA_ESPECIALIDADE]`, `[REPO]`, `[SEU_SITE]` e `[SEU_PERFIL]`. Troque também as descrições genéricas dos projetos por texto confirmado nos respectivos READMEs. Não copie links de exemplo para produção.

A tabela abaixo indica o que revisar antes do primeiro commit:

| Área | O que personalizar | Regra de qualidade |
|:---|:---|:---|
| Identidade | Nome, frase, especialidades e localização | Use uma descrição curta e verdadeira. |
| Linguagens | Ícones e lista de linguagens | Mostre apenas linguagens usadas em projetos reais. |
| Ferramentas | Git, editores, runtimes, frameworks e serviços | Remova tudo que não fizer parte do seu fluxo. |
| Missões | Nome, foco, status e links | Cada descrição deve apontar para um projeto verificável. |
| Contato | GitHub, LinkedIn, site e canais | Teste todos os links antes de publicar. |

## 4. Personalize a paleta sem perder a identidade

A paleta-base usa `#0e0c16` para o fundo, `#2b1d3b` para o violeta profundo, `#d4a24e` para o ouro principal, `#e8c07a` para o ouro claro, `#3ddc84` para estados ativos e `#f4ecdd` para texto claro. Os serviços `capsule-render`, `readme-typing-svg`, `skillicons.dev` e `shields.io` recebem essas cores por URL.

Para alterar a identidade, substitua os códigos de cor nos links de imagem. Preserve contraste suficiente e evite usar muitas cores em uma mesma seção. A estética funciona melhor quando **o ouro marca títulos**, **o verde indica estado ativo** e **o violeta sustenta a estrutura**.

## 5. Configure os ícones de linguagens e ferramentas

Os ícones do [Skill Icons](https://skillicons.dev/) usam a forma `?i=python,js,ts,html,css`. O nome do ícone precisa ser compatível com o serviço. Para linguagens sem ícone disponível, mantenha a linguagem na tabela Markdown e não invente um ícone.

```markdown
[![Languages](https://skillicons.dev/icons?i=python,js,ts,html,css,rust,go&theme=dark)](https://skillicons.dev)

[![Tools](https://skillicons.dev/icons?i=git,github,vscode,linux,nodejs,vite,react,docker&theme=dark)](https://skillicons.dev)
```

O template completo aceita também uma seção textual de evidência. Use-a para explicar a relação entre uma ferramenta e seus projetos, especialmente quando um ícone não comunica contexto suficiente.

## 6. Evite expor informações privadas

Não publique tokens, chaves de API, arquivos `.env`, caminhos locais, conteúdo de repositórios privados ou links que revelem dados internos. Se você criar uma automação para atualizar métricas, prefira métricas públicas e mantenha qualquer token em **Secrets** do GitHub Actions; jamais escreva o valor do token no README, no log ou no código versionado.

Para um perfil manual, a atualização pode ser feita sem workflow. Para um perfil automatizado, crie primeiro uma cópia de segurança, valide o diff e permita que o job altere somente blocos marcados com `START/END`.

## 7. Checklist antes de publicar

```bash
# Verificar placeholders ainda existentes
grep -RInE '\[SEU_|\[SUA_|\[REPO\]|\[COLCHETES\]' README.md

# Verificar links e espaços acidentais
git diff --check

# Confirmar que nenhum segredo foi incluído
grep -nE 'sk-|ghp_|BEGIN .* PRIVATE KEY|postgresql://|mysql://' README.md || true
```

Depois, abra o README renderizado no GitHub e confirme se os banners, títulos animados, ícones e badges carregam corretamente. Se uma imagem externa falhar, mantenha um fallback textual próximo dela.

## 8. Licença e adaptação

Você pode adaptar o template para seu próprio perfil. Preserve os créditos do kit se fizer uma cópia direta e remova qualquer conteúdo que não represente seu trabalho. O estilo é um ponto de partida, não uma obrigação: a prioridade é que a apresentação visual seja legível, verificável e coerente com seus projetos.


## Referências

[1] [Capsule Render — gerador de headers e footers para README](https://github.com/kyeongrok-lee/capsule-render)

[2] [readme-typing-svg — títulos animados para README](https://github.com/DenverCoder1/readme-typing-svg)

[3] [Skill Icons — ícones de linguagens e ferramentas](https://github.com/tandpfun/skill-icons)

[4] [Shields.io — badges de status e métricas](https://shields.io/)

[5] [GitHub Actions — automação de workflows](https://docs.github.com/en/actions)
