# Ecosystem Commit Monitor

> Snapshot horário do ecossistema público. O perfil acompanha o último commit de cada repositório e agrega mudanças; ele não espelha o histórico inteiro dos projetos.

**Última varredura:** `2026-08-31T05:33:06Z`  
**Intervalo configurado:** `1 hora`  
**Repositórios acompanhados:** `61`  
**Repositórios com mudanças desde a última varredura:** `2`  
**Falhas de consulta:** `3`

## Contadores

- **Commits rastreados pelo ecossistema:** `2809`
- **Commits dos projetos:** `2662`
- **Commits do próprio monitor:** `147`
- **Commits de projetos detectados nesta hora:** `22`

> O contador acima é uma métrica própria do monitor. Ele não é o mesmo que **GitHub Contributions**. O contador do monitor cresce somente quando há mudança semântica e o snapshot é publicado; varreduras sem mudança são no-op.

## Mudanças detectadas

- **Cronicas-da-Baluarte-Onde-os-Deuses-Sangram** — 3 commit(s) — [697205a4d5c0](https://github.com/Lucas-Belucci-Bellini/Cronicas-da-Baluarte-Onde-os-Deuses-Sangram/commit/697205a4d5c0a8b6877e56dc6599990a0985ee29) — **nota:**((não sei fazer coisa de romance (se teve antes foi porque eu pedi ajuda de ia para escrever roteiro de romance, pode ser que no fu
- **Subnautica-Unhinged-mod-** — 19 commit(s) — [842ca30dd5d4](https://github.com/Lucas-Belucci-Bellini/Subnautica-Unhinged-mod-/commit/842ca30dd5d4aac7921408d188741ba1562bfea7) — Fechar o buraco do portao que passava vazio

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
