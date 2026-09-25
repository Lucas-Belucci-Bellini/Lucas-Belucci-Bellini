# Ecosystem Commit Monitor

> Snapshot horário do ecossistema público. O perfil acompanha o último commit de cada repositório e agrega mudanças; ele não espelha o histórico inteiro dos projetos.

**Última varredura:** `2026-09-25T07:47:12Z`  
**Intervalo configurado:** `1 hora`  
**Repositórios acompanhados:** `67`  
**Repositórios com mudanças desde a última varredura:** `3`  
**Falhas de consulta:** `3`

## Contadores

- **Commits rastreados pelo ecossistema:** `3364`
- **Commits dos projetos:** `3143`
- **Commits do próprio monitor:** `221`
- **Commits de projetos detectados nesta hora:** `25`

> O contador acima é uma métrica própria do monitor. Ele não é o mesmo que **GitHub Contributions**. O contador do monitor cresce somente quando há mudança semântica e o snapshot é publicado; varreduras sem mudança são no-op.

## Mudanças detectadas

- **Cronicas-da-Baluarte-Onde-os-Deuses-Sangram** — 1 commit(s) — [1506aec986b2](https://github.com/Lucas-Belucci-Bellini/Cronicas-da-Baluarte-Onde-os-Deuses-Sangram/commit/1506aec986b2f4cb0b987ab10491dd16a8e873a9) — atualização do roteiro do cap 23
- **NEXORA** — 23 commit(s) — [8b9b8245b047](https://github.com/Lucas-Belucci-Bellini/NEXORA/commit/8b9b8245b04785797841f4c8365e5651a906dbe1) — Merge pull request #36: the RHI contract, its null backend and the conformance suite
- **Projeto-Baluarte** — 1 commit(s) — [f3b0cb7f20df](https://github.com/Lucas-Belucci-Bellini/Projeto-Baluarte/commit/f3b0cb7f20dfd8fff2e72587bec71e036ee70ca7) — Atualiza câmbio (dólar, euro, bitcoin) [automático]

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
