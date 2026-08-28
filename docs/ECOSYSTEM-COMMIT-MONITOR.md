# Ecosystem Commit Monitor

> Snapshot horário do ecossistema público. O perfil acompanha o último commit de cada repositório e agrega mudanças; ele não espelha o histórico inteiro dos projetos.

**Última varredura:** `2026-08-28T05:23:41Z`  
**Intervalo configurado:** `1 hora`  
**Repositórios acompanhados:** `59`  
**Repositórios com mudanças desde a última varredura:** `3`  
**Falhas de consulta:** `3`

## Contadores

- **Commits rastreados pelo ecossistema:** `2641`
- **Commits dos projetos:** `2503`
- **Commits do próprio monitor:** `138`
- **Commits de projetos detectados nesta hora:** `75`

> O contador acima é uma métrica própria do monitor. Ele não é o mesmo que **GitHub Contributions**. O contador do monitor cresce somente quando há mudança semântica e o snapshot é publicado; varreduras sem mudança são no-op.

## Mudanças detectadas

- **Java-activities** — 1 commit(s) — [9f201c2faacd](https://github.com/Lucas-Belucci-Bellini/Java-activities/commit/9f201c2faacdf5efeb15a87b115acd5a42ee332e) — Update Main.java
- **Project-Vanguard** — 61 commit(s) — [c0ca17176943](https://github.com/Lucas-Belucci-Bellini/Project-Vanguard/commit/c0ca1717694327d708aeb5c7699e0dfa879400ea) — docs(v2): fixar hash do artifact de teste
- **Projeto-Baluarte** — 13 commit(s) — [169a32ddebf8](https://github.com/Lucas-Belucci-Bellini/Projeto-Baluarte/commit/169a32ddebf80b94e44a4e1cdf7d8d7c13b35379) — Merge pull request #535 — a sonda de saúde do Core responde em vez de levantar

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
