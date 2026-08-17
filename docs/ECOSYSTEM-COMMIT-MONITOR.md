# Ecosystem Commit Monitor

> Snapshot horário do ecossistema público. O perfil acompanha o último commit de cada repositório e agrega mudanças; ele não espelha o histórico inteiro dos projetos.

**Última varredura:** `2026-08-17T14:21:10Z`  
**Intervalo configurado:** `1 hora`  
**Repositórios acompanhados:** `59`  
**Repositórios com mudanças desde a última varredura:** `1`

## Mudanças detectadas

- **Lucas-Belucci-Bellini** — quantidade não determinada — [798ae3a567c3](https://github.com/Lucas-Belucci-Bellini/Lucas-Belucci-Bellini/commit/798ae3a567c35ccb2ed314e4a945bdc1a50564e1) — docs(bot): define hourly ecosystem snapshots

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

### Regra de estabilidade

O perfil faz **um snapshot por hora**, independentemente de haver mudanças nos projetos. Isso mantém uma cadência previsível de atividade no próprio perfil.

As mudanças dos projetos continuam agregadas: um único snapshot pode registrar quantos commits cada repositório recebeu desde a varredura anterior, sem copiar esses commits para o perfil.

Em um ano comum, uma execução horária representa no máximo 8.760 snapshots programados; atrasos do scheduler do GitHub podem fazer o horário real variar.
