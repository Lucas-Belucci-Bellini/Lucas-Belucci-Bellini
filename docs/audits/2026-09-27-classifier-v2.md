# `classifier@2` — o que muda na vitrine (para revisão do dono)

**Status:** proposta (D-039). Nada disto está publicado: o padrão continua
`py-classify@1`, que reproduz o Python byte a byte, inclusive o achado **A6**
(palavra-chave casada como *substring*: `"ai"` em "d**ai**ly", `"java"` em
"**java**script"). A versão nova só entra quando o dono escolher.

## O que o `classifier@2` faz

As mesmas listas de palavras-chave, na mesma precedência, casadas como
**palavras inteiras**:

- tudo que não é letra nem dígito separa palavras, dos dois lados — então
  `teste aula` casa `Teste-aula-git`, e `banco de dados` casa `-BANCO-DE-DADOS-`;
- no **nome** do repositório, `camelCase` e a fronteira letra/dígito também
  separam (`DailyPlanner` → "daily planner", `JarvisCore` → "jarvis core");
- na **descrição**, não: ela é prosa, e "JavaScript" viraria "java script"
  (Academia).

O rótulo decide o domínio da vitrine (WHAT-I-BUILD), a árvore do ECOSYSTEM-MAP,
o status (acadêmico ou não), as contagens do painel e do snapshot, a coluna de
categoria dos projetos privados e o rótulo dos destaques escolhidos pela
heurística. A curadoria editorial (`README_FEATURED.json`) não muda.

## Como foi medido

`tests/e2e/classifier_report.py target/release/profile-core` gera o README duas
vezes sobre a árvore real do perfil (o README publicado, os manifestos e um
inventário reconstruído do `docs/project-catalog.json` versionado — nomes e
descrições reais; `fork` e `homepage` aproximados pelo status e pela origem do
site no catálogo). A lista definitiva sai dos dados de verdade:

```bash
PROFILE_GITHUB_TOKEN=… profile-core render readme --skip-site-check --out-dir /tmp/v1
PROFILE_GITHUB_TOKEN=… profile-core render readme --skip-site-check --classifier classifier@2 --out-dir /tmp/v2
diff -u /tmp/v1/README.md /tmp/v2/README.md
```

## Para ligar

Não é um PR de código: é passar `--classifier classifier@2` no `render` do
workflow que publica (e no `sync github`, para o rótulo do banco acompanhar —
o `classifier_version` de cada projeto registra a versão). Voltar é tirar a
opção.

## Rótulos que mudam (4 de 89 projetos; 2 públicos)

| Projeto | Descrição pública | py-classify@1 | classifier@2 |
|:---|:---|:---|:---|
| -BANCO-DE-DADOS- 🔒 | Descrição pública não informada. | Software & Ferramentas | **Infraestrutura / Backend / Dados** |
| DailyPlanner | Descrição pública não informada. | IA & Automação | **Web** |
| LOCAL-DE-TRABALHO 🔒 | dados para trabalho e criação de sites | Web | **Infraestrutura / Backend / Dados** |
| Teste-aula-git | Descrição pública não informada. | Software & Ferramentas | **Academia** |

## Linhas do README que mudam (68)

```diff
--- README (py-classify@1)
+++ README (classifier@2)
-`39 projetos` · `7 com site`
+`40 projetos` · `7 com site`
-`3 projetos` · `0 com site`
-
-<sub>ex.: DailyPlanner</sub>
+`2 projetos` · `0 com site`
+
+<sub>ex.: Kizeo-Forms</sub>
+`9 projetos` · `0 com site`
+
+<sub>ex.: Teste-</sub>
+
+</td>
+<td width="33%" valign="top" align="center">
+
+### 🎓
+
+**ACADEMIC & LABS**
+
+<sub>Trabalhos de curso, estudos dirigidos e experimentos.</sub>
+
-
-<sub>ex.: Teste-aula-git</sub>
-
-</td>
-<td width="33%" valign="top" align="center">
-
-### 🎓
-
-**ACADEMIC & LABS**
-
-<sub>Trabalhos de curso, estudos dirigidos e experimentos.</sub>
-
-`9 projetos` · `0 com site`
-├── Web ........................  7 projetos · 2 com site
-│   └─ CodeVibe-Academy ● · Essence-Custom-Furniture ● · Ark-Initiative · DriveTax-Motors · +3
+├── Web ........................  8 projetos · 2 com site
+│   └─ CodeVibe-Academy ● · Essence-Custom-Furniture ● · DailyPlanner · Ark-Initiative · +4
-├── Software & Ferramentas ..... 10 projetos · 0 com site
-│   └─ Cookie-Clicker-Bot · Cosmos · FanVerse · Lucas-Belucci-Bellini · +6
+├── Academia ................... 10 projetos · 0 com site
+│   └─ Academic-Portfolio · Atividade-6 · Decision-Structures · Flowgorithm- · +6
-├── Academia ...................  9 projetos · 0 com site
-│   └─ Academic-Portfolio · Atividade-6 · Decision-Structures · Flowgorithm- · +5
+├── Software & Ferramentas .....  9 projetos · 0 com site
+│   └─ Cookie-Clicker-Bot · Cosmos · FanVerse · Lucas-Belucci-Bellini · +5
-└── IA & Automação .............  3 projetos · 0 com site
-    └─ DailyPlanner · AI-second-brain-with-Claude-and-Obsidian · Kizeo-Forms
+└── IA & Automação .............  2 projetos · 0 com site
+    └─ AI-second-brain-with-Claude-and-Obsidian · Kizeo-Forms
-| **-BANCO-DE-DADOS-** | Software & Ferramentas | `Rust` | 🔒 Private | [código](https://github.com/Lucas-Belucci-Bellini/-BANCO-DE-DADOS-) |
+| **-BANCO-DE-DADOS-** | Infraestrutura / Backend / Dados | `Rust` | 🔒 Private | [código](https://github.com/Lucas-Belucci-Bellini/-BANCO-DE-DADOS-) |
-| **DailyPlanner** | IA & Automação | `JavaScript` `CSS` `Portugol` | 🟢 Active | [código](https://github.com/Lucas-Belucci-Bellini/DailyPlanner) |
+| **DailyPlanner** | Web | `JavaScript` `CSS` `Portugol` | 🟢 Active | [código](https://github.com/Lucas-Belucci-Bellini/DailyPlanner) |
-| **LOCAL-DE-TRABALHO** | Web | — | 🔒 Private | [código](https://github.com/Lucas-Belucci-Bellini/LOCAL-DE-TRABALHO) |
+| **LOCAL-DE-TRABALHO** | Infraestrutura / Backend / Dados | — | 🔒 Private | [código](https://github.com/Lucas-Belucci-Bellini/LOCAL-DE-TRABALHO) |
-| **Teste-aula-git** | Software & Ferramentas | `SQF` | 🟡 In Development | [código](https://github.com/Lucas-Belucci-Bellini/Teste-aula-git) |
+| **Teste-aula-git** | Academia | `SQF` | 🟣 Academic | [código](https://github.com/Lucas-Belucci-Bellini/Teste-aula-git) |
-| **19** | **9** | **17** | **3** |
+| **19** | **10** | **17** | **3** |
-| **DailyPlanner** | IA & Automação | 🟢 Active | [código](https://github.com/Lucas-Belucci-Bellini/DailyPlanner) |
+| **DailyPlanner** | Web | 🟢 Active | [código](https://github.com/Lucas-Belucci-Bellini/DailyPlanner) |
-| **Teste-aula-git** | Software & Ferramentas | 🟡 In Development | [código](https://github.com/Lucas-Belucci-Bellini/Teste-aula-git) |
+| **Teste-aula-git** | Academia | 🟣 Academic | [código](https://github.com/Lucas-Belucci-Bellini/Teste-aula-git) |
-| **-BANCO-DE-DADOS-** | Software & Ferramentas | Descrição pública não informada | 🔒 Private | [GitHub](https://github.com/Lucas-Belucci-Bellini/-BANCO-DE-DADOS-) |
+| **-BANCO-DE-DADOS-** | Infraestrutura / Backend / Dados | Descrição pública não informada | 🔒 Private | [GitHub](https://github.com/Lucas-Belucci-Bellini/-BANCO-DE-DADOS-) |
-| **LOCAL-DE-TRABALHO** | Web | dados para trabalho e criação de sites | 🔒 Private | [GitHub](https://github.com/Lucas-Belucci-Bellini/LOCAL-DE-TRABALHO) |
+| **LOCAL-DE-TRABALHO** | Infraestrutura / Backend / Dados | dados para trabalho e criação de sites | 🔒 Private | [GitHub](https://github.com/Lucas-Belucci-Bellini/LOCAL-DE-TRABALHO) |
```
