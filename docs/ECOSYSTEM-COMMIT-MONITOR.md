# Ecosystem Commit Monitor

> Snapshot horário do ecossistema público. O perfil acompanha o último commit de cada repositório e agrega mudanças; ele não espelha o histórico inteiro dos projetos.

**Última varredura:** `2026-08-21T07:07:28Z`  
**Intervalo configurado:** `1 hora`  
**Repositórios acompanhados:** `59`  
**Repositórios com mudanças desde a última varredura:** `2`  
**Falhas de consulta:** `3`

## Contadores

- **Commits rastreados pelo ecossistema:** `2025`
- **Commits dos projetos:** `1939`
- **Commits do próprio monitor:** `86`
- **Commits de projetos detectados nesta hora:** `14`

> O contador acima é uma métrica própria do monitor. Ele não é o mesmo que **GitHub Contributions**. O contador do monitor cresce somente quando há mudança semântica e o snapshot é publicado; varreduras sem mudança são no-op.

## Mudanças detectadas

- **Projeto-Baluarte** — 6 commit(s) — [7c2ff15123c4](https://github.com/Lucas-Belucci-Bellini/Projeto-Baluarte/commit/7c2ff15123c4a63b7c5a582069b6abd03b925ffa) — feat(auth): add identity release preflight
- **Veritas** — 8 commit(s) — [eab7b65d5f77](https://github.com/Lucas-Belucci-Bellini/Veritas/commit/eab7b65d5f7761c8f376aad609d853e36c63120f) — feat: add wireless channel contract

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
