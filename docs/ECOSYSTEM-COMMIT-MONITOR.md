# Ecosystem Commit Monitor

> Snapshot horário do ecossistema público. O perfil acompanha o último commit de cada repositório e agrega mudanças; ele não espelha o histórico inteiro dos projetos.

**Última varredura:** `2026-09-17T07:38:39Z`  
**Intervalo configurado:** `1 hora`  
**Repositórios acompanhados:** `68`  
**Repositórios com mudanças desde a última varredura:** `3`  
**Falhas de consulta:** `3`

## Contadores

- **Commits rastreados pelo ecossistema:** `3219`
- **Commits dos projetos:** `3016`
- **Commits do próprio monitor:** `203`
- **Commits de projetos detectados nesta hora:** `20`

> O contador acima é uma métrica própria do monitor. Ele não é o mesmo que **GitHub Contributions**. O contador do monitor cresce somente quando há mudança semântica e o snapshot é publicado; varreduras sem mudança são no-op.

## Mudanças detectadas

- **Cronicas-da-Baluarte-Onde-os-Deuses-Sangram** — 2 commit(s) — [03eaaf6dccbc](https://github.com/Lucas-Belucci-Bellini/Cronicas-da-Baluarte-Onde-os-Deuses-Sangram/commit/03eaaf6dccbc585aaf3e6642c58a8b12db6c8b17) — Cronicas-da-Baluarte-Onde-os-Deuses-Sangram/roteiro.md
- **Projeto-Baluarte** — 1 commit(s) — [d97d6eed3658](https://github.com/Lucas-Belucci-Bellini/Projeto-Baluarte/commit/d97d6eed365878764863b536135c9262cdea358a) — Atualiza câmbio (dólar, euro, bitcoin) [automático]
- **PromoRadar** — 17 commit(s) — [6e37abb6f9df](https://github.com/Lucas-Belucci-Bellini/PromoRadar/commit/6e37abb6f9df40e300a278f2803955adcfb55651) — feat: add transparency page

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
