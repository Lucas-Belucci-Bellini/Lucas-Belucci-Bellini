# README Style Kit — Field Manual

Este diretório é o **template público reutilizável** da estética Baluarte / Spartan / Field Manual. Ele transforma um README de perfil em uma pequena interface editorial: banner escuro, acentos dourados, status em verde, badges, Arsenal categorizado, missões verificáveis, métricas com fonte e footer.

> **Como funciona:** copie um template, preencha a ficha de configuração, substitua os placeholders, valide links e segredos e publique no seu repositório de perfil. O passo a passo completo está em [`STYLE_TUTORIAL.md`](STYLE_TUTORIAL.md).

## Acesse o kit público

A branch pública está disponível em [Lucas-Belucci-Bellini/Lucas-Belucci-Bellini/tree/template/readme-style-kit](https://github.com/Lucas-Belucci-Bellini/Lucas-Belucci-Bellini/tree/template/readme-style-kit). Ela contém os templates, snippets, configuração de exemplo, workflow de referência e este tutorial. Você pode copiar o conteúdo, adaptar a paleta e remover tudo que não representar seu próprio trabalho.

| Recurso | Uso |
|:---|:---|
| [`STYLE_TUTORIAL.md`](STYLE_TUTORIAL.md) | Tutorial completo: instalação, placeholders, categorias, badges, métricas, segurança e troubleshooting. |
| [`templates/README.template.md`](templates/README.template.md) | Template completo com narrativa, Arsenal em cinco camadas, missões e observabilidade opcional. |
| [`templates/README.minimal.template.md`](templates/README.minimal.template.md) | Template curto para um perfil de leitura rápida. |
| [`templates/profile-config.example.json`](templates/profile-config.example.json) | Ficha estruturada de identidade, stack, projetos, fontes e métricas. |
| [`templates/section-snippets.md`](templates/section-snippets.md) | Seções avulsas para copiar badges, métricas e categorias. |
| [`templates/workflows/profile-traffic.example.yml`](templates/workflows/profile-traffic.example.yml) | Exemplo opcional de Action para tráfego oficial de repositórios. |

## Princípios do kit

A identidade visual usa fundo quase preto (`#0e0c16`), violeta profundo (`#2b1d3b`), ouro (`#d4a24e` e `#e8c07a`), verde de estado (`#3ddc84`) e texto claro (`#f4ecdd`). O ouro marca títulos e evidências; o verde indica estado ativo ou validação aprovada; o violeta sustenta a estrutura.

A estética não substitui evidência. Linguagens devem vir de repositórios reais ou de uma declaração manual consciente. Ferramentas devem ser sustentadas por projetos, documentação, manifests ou fluxo de trabalho verificável. Métricas devem registrar fonte, janela de tempo e data de coleta.

## Conteúdo seguro por padrão

O kit não autoriza publicar tokens, chaves, arquivos `.env`, caminhos locais, conteúdo privado ou URLs internas. Para automações, use Secrets do GitHub Actions, marque blocos dinâmicos com `START/END` e execute o checklist do tutorial antes do merge.

O contador de visualizações de perfil mostrado nos exemplos é externo. Ele deve ser rotulado como indicador de carregamentos do contador, não como analytics oficial do GitHub. Para tráfego oficial, use os endpoints de tráfego de cada repositório com acesso de escrita e respeite a janela limitada documentada pelo GitHub.

## Créditos e adaptação

Preserve os créditos quando fizer uma cópia direta. Você pode adaptar cores, textos, ícones, categorias e quantidade de projetos. Remova qualquer conteúdo de exemplo e substitua descrições por fatos do seu próprio portfólio.
