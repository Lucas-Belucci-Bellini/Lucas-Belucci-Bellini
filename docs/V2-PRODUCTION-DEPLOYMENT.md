# Atualização automática de produção após push no `main`

## Escolha recomendada

Para uma aplicação hospedada em um provedor com integração Git, prefira a integração nativa do provedor: o provedor observa o `main`, cria o build e publica o commit aprovado. Isso evita manter um endpoint público adicional.

Quando a integração nativa não estiver disponível, use um deploy hook protegido acionado por GitHub Actions. Um webhook próprio só é necessário quando o ambiente de produção precisa controlar a fila, o rollout ou múltiplos servidores.

| Abordagem | Trade-off | Custo | Complexidade |
| --- | --- | --- | --- |
| Integração Git nativa do provedor | Menos código e menos superfície pública; depende do provedor | Normalmente incluso no provedor | Baixa |
| GitHub Actions + deploy hook secreto | Valida antes de publicar e não exige endpoint GitHub público | Depende do provedor e dos runners | Média |
| Webhook próprio `push` + fila de deploy | Controle total, observabilidade e rollout customizado; exige servidor sempre disponível | Hospedagem do endpoint e do worker | Alta |

## GitHub Actions + deploy hook

1. Crie no provedor de produção um endpoint de deploy hook exclusivo para o projeto.
2. No repositório, abra **Settings → Secrets and variables → Actions → New repository secret**.
3. Cadastre o segredo `PRODUCTION_DEPLOY_HOOK_URL` sem gravar a URL no código.
4. Crie um workflow que seja disparado apenas por `push` em `main`, execute testes/build e só então faça `POST` para o hook.
5. Configure `concurrency` para cancelar uma publicação antiga quando uma nova versão do `main` chegar.
6. Faça o endpoint registrar o SHA recebido e valide a saúde da versão publicada antes de marcar o deploy como concluído.

Exemplo de etapa de publicação:

```yaml
- name: Acionar deploy de produção
  if: ${{ secrets.PRODUCTION_DEPLOY_HOOK_URL != '' }}
  env:
    DEPLOY_HOOK_URL: ${{ secrets.PRODUCTION_DEPLOY_HOOK_URL }}
  run: |
    curl --fail-with-body --retry 3 --retry-delay 5 \
      -X POST "$DEPLOY_HOOK_URL"
```

No Projeto-Baluarte, a configuração de hospedagem declara funções compatíveis com Vercel, mas o endpoint/segredo de produção não está disponível para configuração automática nesta sessão. Portanto, a etapa correta é adicionar o segredo no repositório e conectar o hook do provedor; não se deve inventar ou gravar uma URL de produção.

## Webhook próprio do GitHub

1. No repositório, abra **Settings → Webhooks → Add webhook**.
2. Informe uma URL HTTPS pública para um endpoint `POST`, selecione `application/json` e crie um segredo aleatório de alta entropia.
3. Selecione **Let me select individual events** e habilite somente **Pushes**.
4. Ative o webhook. O GitHub enviará um evento `ping` inicial.
5. O endpoint deve aceitar somente a branch `refs/heads/main`, validar `X-Hub-Signature-256` com HMAC-SHA256 antes de interpretar o corpo, deduplicar por `X-GitHub-Delivery` e responder rapidamente com `2xx`.
6. O deploy deve ser assíncrono: enfileirar o SHA exato, fazer checkout desse SHA, rodar testes/build, publicar, executar health check e permitir rollback.

A verificação deve usar o corpo bruto recebido, não um JSON reserializado. O segredo fica somente no servidor. Nunca aceite uma requisição apenas porque o caminho contém o nome do repositório.

Exemplo mínimo de verificação em Python:

```python
import hashlib
import hmac

def verify_github_signature(raw_body: bytes, secret: str, header: str | None) -> bool:
    if not header or not header.startswith("sha256="):
        return False
    expected = "sha256=" + hmac.new(
        secret.encode("utf-8"), raw_body, hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(expected, header)
```

## Estado atual

O perfil não possui um servidor de produção próprio; sua publicação principal é o próprio repositório e seus artefatos públicos. A V2 implementada neste marco reduz commits automáticos e adiciona validação. Para o site do Projeto-Baluarte, a publicação automática depende da integração Vercel ou do segredo `PRODUCTION_DEPLOY_HOOK_URL`; essa parte permanece pendente até o endpoint real ser fornecido/configurado no provedor.

## Referências

[1]: https://docs.github.com/en/webhooks/using-webhooks/creating-webhooks "GitHub — Creating webhooks"
[2]: https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries "GitHub — Validating webhook deliveries"
[3]: https://docs.github.com/webhooks/webhook-events-and-payloads "GitHub — Webhook events and payloads"
[4]: https://docs.github.com/actions/using-workflows/events-that-trigger-workflows "GitHub — Events that trigger workflows"
