# Ecosystem Commit Monitor

> Snapshot horário do ecossistema público. O perfil acompanha o último commit de cada repositório e agrega mudanças; ele não espelha o histórico inteiro dos projetos.

**Última varredura:** `2026-10-09T02:08:33Z`  
**Intervalo configurado:** `1 hora`  
**Repositórios acompanhados:** `67`  
**Repositórios com mudanças desde a última varredura:** `2`  
**Falhas de consulta:** `3`

## Contadores

- **Commits rastreados pelo ecossistema:** `3934`
- **Commits dos projetos:** `3672`
- **Commits do próprio monitor:** `262`
- **Commits de projetos detectados nesta hora:** `319`

> O contador acima é uma métrica própria do monitor. Ele não é o mesmo que **GitHub Contributions**. O contador do monitor cresce somente quando há mudança semântica e o snapshot é publicado; varreduras sem mudança são no-op.

## Mudanças detectadas

- **Cosmos** — 75 commit(s) — [c5453aa33e44](https://github.com/Lucas-Belucci-Bellini/Cosmos/commit/c5453aa33e44e81c6962b4f3365db351fcf90135) — docs(index): link Claude Code protocol and canonical master plan
- **NEXORA** — 244 commit(s) — [b686b181fb23](https://github.com/Lucas-Belucci-Bellini/NEXORA/commit/b686b181fb23a5002a78aa2a4d60b3eea169e977) — Merge pull request #49 from Lucas-Belucci-Bellini/chore/asset-prompts-from-issues

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
       ├── estado dos projetos
       ├── commits dos projetos
       ├── + 1 commit do monitor
       └── contador acumulado
       │
       ▼
snapshot agregado a cada hora
```

### Regras de estabilidade

1. O perfil faz uma varredura programada por hora.
2. Cada snapshot publicado acrescenta exatamente 1 ao contador de commits do monitor; varreduras sem mudança semântica não geram commit.
3. As mudanças dos projetos são agregadas: um snapshot pode registrar quantos commits cada repositório recebeu desde a varredura anterior, sem copiar esses commits para o perfil.
4. Retries e backoff protegem contra falhas transitórias da API.
5. Repositórios novos do usuário são descobertos automaticamente; forks são ignorados.
6. O contador próprio do ecossistema não tenta reproduzir a métrica oficial de GitHub Contributions.

A varredura continua horária para detectar mudanças, mas o histórico só recebe commits quando há alteração semântica; o scheduler do GitHub pode atrasar a execução real.
