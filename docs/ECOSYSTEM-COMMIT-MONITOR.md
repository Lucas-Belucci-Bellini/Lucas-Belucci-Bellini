# Ecosystem Commit Monitor

> Snapshot horário do ecossistema público. O perfil acompanha o último commit de cada repositório e agrega mudanças; ele não espelha o histórico inteiro dos projetos.

**Última varredura:** `2026-08-17T14:30:55Z`  
**Intervalo configurado:** `1 hora`  
**Repositórios acompanhados:** `59`  
**Repositórios com mudanças desde a última varredura:** `1`  
**Falhas de consulta:** `3`

## Mudanças detectadas

- **Lucas-Belucci-Bellini** — quantidade não determinada — [fd5bf4655326](https://github.com/Lucas-Belucci-Bellini/Lucas-Belucci-Bellini/commit/fd5bf46553262d35904a98e1860d7af96da442e5) — fix(bot): harden ecosystem watcher for scale

## Erros de consulta

- **LLBR-Innovations-** — `latest_commit` — HTTP Error 409: Conflict
- **MOD-PACK-MINE-BACKUP** — `latest_commit` — HTTP Error 409: Conflict
- **Projeto-Baluarte-Social-Media** — `latest_commit` — HTTP Error 409: Conflict

## Arquitetura

```text
projetos individuais
       │
       │ latest SHA + comparação
       ▼
ecosystem_watch.py
       │
       ├── ECOSYSTEM-COMMIT-STATE.json
       └── ECOSYSTEM-COMMIT-MONITOR.md
       │
       ▼
snapshot agregado a cada hora
```

### Regras de estabilidade

1. O perfil faz uma varredura programada por hora.
2. Cada execução atualiza o timestamp do snapshot; portanto o heartbeat horário não depende de haver commits nos projetos.
3. As mudanças dos projetos são agregadas: um snapshot pode registrar quantos commits cada repositório recebeu desde a varredura anterior, sem copiar esses commits para o perfil.
4. Erros transitórios da API são repetidos com backoff; falhas persistentes são registradas sem derrubar toda a varredura.
5. Repositórios novos do usuário são descobertos automaticamente; forks são ignorados.

Em um ano comum, uma execução horária representa no máximo 8.760 snapshots programados; o scheduler do GitHub pode atrasar a execução real.
