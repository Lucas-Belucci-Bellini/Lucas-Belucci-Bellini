"""Casos de paridade do domínio: Python é a referência, Rust é conferido contra ela.

    OLD PYTHON ──▶ tests/fixtures/parity/domain.json ◀── NEW RUST

Este teste calcula, com as funções Python reais, a saída esperada de cada caso
e exige que ela seja igual ao arquivo versionado. O crate
`crates/ecosystem-domain` carrega o MESMO arquivo e exige a mesma saída
(`cargo test -p ecosystem-domain --test parity`). Um lado não muda sem o
outro perceber.

Os casos incluem de propósito as armadilhas de semântica do Python que um
port mecânico erraria (docs/migration/PYTHON-TO-RUST.md, "Armadilhas de
paridade"): `round()` com empate para o par, `str.strip()`/`split()` que
consideram \\x1c–\\x1f espaço, `$` do `re` casando antes de um `\\n` final,
`.days` de timedelta, datas sem fuso.

Mudou uma regra do Python de propósito? Regenere **com o Python do CI (3.12)**
e revise o diff:

    UPDATE_PARITY=1 python3.12 -m unittest tests.test_parity_domain

A leitura de datas (`datetime.fromisoformat`) muda entre versões do CPython;
o grupo `github_timestamp` tem as bordas que o código atual do ramo 3.13 muda
(o 3.13.12 ainda se comporta como o 3.12), para que um Python do CI com essa
mudança reprove este teste em vez de mudar a referência calado.
"""
from __future__ import annotations

import importlib.util
import json
import os
import sys
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "parity" / "domain.json"
sys.path.insert(0, str(ROOT / "scripts"))


def _load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


PC = _load("project_catalog", ROOT / "scripts" / "project_catalog.py")
UP = _load("update_profile", ROOT / "scripts" / "update_profile.py")

NOW = datetime(2026, 9, 25, 12, 0, tzinfo=timezone.utc)
OWNER = "Lucas-Belucci-Bellini"


def ago(days: int, hours: int = 0, seconds: int = 0) -> str:
    """Carimbo no formato do GitHub, `days` antes de NOW."""
    return (NOW - timedelta(days=days, hours=hours, seconds=seconds)).strftime("%Y-%m-%dT%H:%M:%SZ")


def repo(name: str, description: str | None = "", **extra: Any) -> dict[str, Any]:
    base: dict[str, Any] = {
        "name": name,
        "full_name": extra.pop("full_name", f"{OWNER}/{name}"),
        "description": description,
        "private": False,
        "fork": False,
        "archived": False,
        "homepage": None,
        "pushed_at": ago(10),
        "updated_at": ago(10),
        "size": 1000,
    }
    base.update(extra)
    return base


# ---------------------------------------------------------------- entradas

URLS = [
    "https://example.org", "http://a.b", "https://", "https://x", "https://xy", "ftp://example.org",
    "HTTPS://example.org", "https:// example.org", "https://.org", "https://example.org/p?q=1#f",
    "https://exa mple.org", "https://example.org\n", "https://example.org\n\n", " https://example.org",
    "", "https://$x.org", "https://?x", "https://#x", "https://é.org", "https://example.org\t",
    "https:/example.org", "https://ex$ample.org", "https://a b.org",
    # `\s` do re do Python inclui \x1c–\x1f; o do regex do Rust, não.
    "https://\x1cexample.org", "https://ex\x1fample.org", "https://a\x1cb.org", "https://example.org\x1e",
]

SLUG_NAMES = [
    "Veritas", "baluarte-obra-segura", "-BANCO-DE-DADOS-", "Portifolio-Baluarte-Lucas-Belucci-Bellini-",
    "Projeto_01_endless_gnome", "file-D-teste-1-site_nova_era-index.html", "Ações Úteis",
    "Kelvin", "İstanbul", "", "---", "a--b__c", "CamelCase123", "ÇÃO", "日本語-repo",
]

DESCRIPTIONS = [
    None, "", "   ", "  espaços   múltiplos  ", "linha\nquebrada", "com\x1fseparador\x1cde\x1dcampo\x1e",
    "tab\tseparado", " nbsp ", "linha separada", "zero​width", "Descrição pública.",
]

LABELS = sorted(PC.CATEGORY_ALIASES) + ["Desconhecida", ""]

CLASSIFY_REPOS = [
    repo("veritas-core", "Simulador"),
    repo("x", "Digital Logic avançada"),
    repo("CHIPS-lab", ""),
    repo("umbra", "Umbra Lima Alfa"),
    repo("UMBRA-LIMA-ALFA", "Calculadora"),
    repo("baluarte-docs", "Documentação"),
    repo("LLBR-site", ""),
    repo("Project-Vanguard", "GPS"),
    repo("Academic-Portfolio", ""),
    repo("Atividade-6", ""),
    repo("Decision-Structures", ""),
    repo("flowgorithm-ex", ""),
    repo("java-exercises", ""),
    repo("python-basics", ""),
    repo("pseudocode", ""),
    repo("Teste aula", ""),
    repo("teste-aula-git", ""),
    repo("my-game", ""),
    repo("GAMES", ""),
    repo("g-mod-pack", ""),
    repo("black mesa", ""),
    repo("fallout-guide", ""),
    repo("mod-pack", ""),
    repo("catacombs", ""),
    repo("ossuary-escape", ""),
    repo("Recycle-game", ""),
    repo("ai-tool", ""),
    repo("artificial-x", ""),
    repo("jarvis", ""),
    repo("claude-notes", ""),
    repo("Kizeo-Forms", ""),
    repo("sujok", ""),
    repo("x", "banco de dados relacional"),
    repo("backend-api", ""),
    repo("LOCAL-DE-TRABALHO", ""),
    repo("backup-01", ""),
    repo("portfolio", ""),
    repo("my-site", ""),
    repo("furniture-shop", ""),
    repo("construction-co", ""),
    repo("birthday-invitation", ""),
    repo("plain-tool", "", homepage="https://tool.example.org"),
    repo("plain-tool", "", homepage=""),
    repo("plain-tool", "", homepage="   "),
    repo("plain-fork", "", fork=True),
    repo("", ""),
    repo("", None),
    repo("plain-tool", "Uma ferramenta comum."),
    # Armadilhas de substring (auditoria, A6) — o comportamento ATUAL é o esperado.
    repo("DailyPlanner", "Agenda diária em TypeScript"),
    repo("mail-helper", "Email automation"),
    repo("details", "Detailed notes"),
    repo("maintain", "Maintenance scripts"),
    repo("portfolio-js", "Personal site built with JavaScript"),
    repo("website-builder", ""),
    repo("Save-Games-BR", "Backups"),
    # Precedência: a primeira lista que casa vence.
    repo("veritas-game", "jarvis python site"),
    repo("baluarte-game", "python"),
    repo("java-game", ""),
    repo("game-ai", ""),
    repo("ai-backend", ""),
    repo("backend-site", ""),
    repo("MiXeD-CaSe-GaMe", ""),
    repo("Ação", "Jogo de AÇÃO"),
]

STATUS_CASES = [
    (repo("p", "", private=True), "Web"),
    (repo("p", "", private=True, archived=True), "Academia"),
    (repo("a", "", archived=True), "Academia"),
    (repo("course", ""), "Academia"),
    (repo("Projeto-Baluarte", "", pushed_at=ago(1)), "Ecossistema Baluarte"),
    (repo("Ark-Initiative", "", pushed_at=ago(900)), "Web"),
    (repo("CHIPS-Digital-Logic-Sim-Lucas-Belucci", ""), "Digital Logic / Hardware"),
    (repo("baluarte-core", "", pushed_at=ago(1)), "Ecossistema Baluarte"),
    (repo("baluarte-obra-segura", "", pushed_at=ago(1)), "Ecossistema Baluarte"),
    (repo("Baluarte-x", "", pushed_at=ago(900)), "Ecossistema Baluarte"),
    (repo("f", "", fork=True, pushed_at=None), "Experimentos"),
    (repo("f", "", fork=True, pushed_at=""), "Experimentos"),
    (repo("f", "", fork=True, pushed_at=ago(1)), "Experimentos"),
    (repo("d0", "", pushed_at=ago(0)), "Web"),
    (repo("d59", "", pushed_at=ago(59)), "Web"),
    (repo("d60", "", pushed_at=ago(60)), "Web"),
    (repo("d60s", "", pushed_at=ago(60, seconds=1)), "Web"),
    (repo("d61", "", pushed_at=ago(61)), "Web"),
    (repo("d364", "", pushed_at=ago(364)), "Web"),
    (repo("d365", "", pushed_at=ago(365)), "Web"),
    (repo("d366", "", pushed_at=ago(366)), "Web"),
    (repo("future", "", pushed_at=ago(-3)), "Web"),
    (repo("none", "", pushed_at=None, updated_at=None), "Web"),
    (repo("fallback", "", pushed_at=None, updated_at=ago(100)), "Web"),
    (repo("empty", "", pushed_at="", updated_at=ago(400)), "Web"),
    (repo("naive", "", pushed_at="2026-09-20T10:00:00", updated_at=None), "Web"),
    (repo("date-only", "", pushed_at="2026-09-20", updated_at=None), "Web"),
    (repo("offset", "", pushed_at="2026-07-27T11:00:00+03:00", updated_at=None), "Web"),
    (repo("offset-neg", "", pushed_at="2026-07-27T08:00:00-03:00", updated_at=None), "Web"),
    (repo("fraction", "", pushed_at="2026-09-20T10:00:00.123Z", updated_at=None), "Web"),
    (repo("garbage", "", pushed_at="not a date", updated_at=None), "Web"),
    (repo("lower-z", "", pushed_at="2026-09-20T10:00:00z", updated_at=None), "Web"),
    (repo("space-sep", "", pushed_at="2026-09-20 10:00:00+00:00", updated_at=None), "Web"),
    (repo("compact-offset", "", pushed_at="2026-09-20T10:00:00+0000", updated_at=None), "Web"),
] + [
    # Formas que datetime.fromisoformat() aceita ou recusa, uma por caso: o
    # parser Rust reproduz exatamente este subconjunto (data válida → Active,
    # recusada → Experimental). Divergências conhecidas, fora do fixture de
    # propósito, estão documentadas em docs/migration/PYTHON-TO-RUST.md.
    (repo(f"iso-{index}", "", pushed_at=value, updated_at=None), "Web")
    for index, value in enumerate([
        "2026-09-20T10:00+00:00", "2026-09-20T10+00:00", "2026-09-20T10:00:00.1+00:00",
        "2026-09-20T10:00:00.1234567+00:00", "2026-09-20T10:00:00,5+00:00", "2026-09-20T10:00:00+03",
        "2026-09-20T10:00:00+0300", "2026-09-20T10:00:00+03:00:30", "2026-09-20x10:00:00+00:00",
        "2026-09-20t10:00:00+00:00", "2026-09-20T10:00:00-00:00", "2026-09-20T1000+00:00",
        "2026-09-20T100000+00:00", "20260920T100000+00:00", "2026-09-20T10:00:00+00:00:00.5",
        "2026-09-20T24:00:00+00:00", "2026-02-30T10:00:00+00:00", "2026-09-20T10:00:60+00:00",
        "2026-09-20T10:00:00+24:00", "+2026-09-20T10:00:00+00:00", "2026-9-20T10:00:00+00:00",
        "2026-09-2010:00:00+00:00", "2026-09-20T10:00:00Z", "2026-09-20TZ", "2026-02-29T10:00:00+00:00",
        "2024-02-29T10:00:00+00:00", "2026-09-20T1:00:00+00:00", "2026-09-20T10:0:00+00:00",
        "2026-09-20T10:00:00+3:00", "2026-09-20T10:00:00.123456789012+00:00",
    ])
]

# Leitura direta de carimbos: a expressão exata do Python
# (`fromisoformat(str(v).replace("Z", "+00:00"))`), comparada pelo instante em
# microssegundos — o status acima só enxerga faixas de dias. O parser Rust é um
# port do C do CPython 3.12 (a versão do CI); cada peculiaridade está aqui.
TIMESTAMPS = [r["pushed_at"] for r, _ in STATUS_CASES if r["pushed_at"]] + [
    # hora
    "2026-09-20T10.5+00:00", "2026-09-20T10,5+00:00", "2026-09-20T10:00.5+00:00", "2026-09-20T10:00:00:5+00:00",
    "2026-09-20T10:0000+00:00", "2026-09-20T1000:00+00:00", "2026-09-20T1000005+00:00", "2026-09-20T10:00:00 +00:00",
    "2026-09-20Tx10:00:00+00:00", "2026-09-20T10:00:00x+00:00", "2026-09-20T10:00:00xy+00:00", "2026-09-20T10.+00:00",
    "2026-09-20T10:00:00.1234567x+00:00", "2026-09-20T10:00:00.12x4+00:00", "2026-09-20T10:00:00.1234567x",
    "2026-09-20T10:00:00.12", "2026-09-20T10:00:00.123456", "2026-09-20T23:60:00+00:00", "2026-09-20T23:59:60+00:00",
    "2026-09-20T", "2026-09-20T10", "2026-09-20T10:00:00", "2026-09-20T+00:00", "2026-09-20T10:00:00+",
    "2026-09-20T10:00:00++00:00", "2026-09-20T10:00:00+00:00 ", " 2026-09-20T10:00:00+00:00",
    "2026-09-20T10:00:00ZZ", "2026-09-20T10:00:00Z0",
    # fuso: sem checagem de faixa por campo; total estritamente entre -24 h e +24 h
    "2026-09-20T10:00:00+00:60", "2026-09-20T10:00:00+00:00:60", "2026-09-20T10:00:00+0099",
    "2026-09-20T10:00:00+23:59:59.999999", "2026-09-20T10:00:00-23:59:59.999999", "2026-09-20T10:00:00+23:59:60",
    "2026-09-20T10:00:00-24:00", "2026-09-20T10:00:00+01:00:00.5", "2026-09-20T10:00:00-01:00:00.5",
    "2026-09-20T10:00:00+03:00:30.25", "2026-09-20T10:00:00-03:00", "2026-09-20T10:00:00+00:1440",
    # as duas bordas que o ramo 3.13 do CPython muda: um Python do CI com isso reprova o fixture
    "2026-09-20T10:00:00.+00:00", "2026-09-20T10:00:00+00:00.5", "2026-09-20T10:00:00-00:00:00.5",
    # separador: qualquer caractere, inclusive multibyte e NUL
    "2026-09-20 10:00:00+00:00", "2026-09-20é10:00:00+00:00", "2026-09-20日10:00:00+00:00",
    "2026-09-20😀10:00:00+00:00", "2026-09-20\x0010:00:00+00:00", "2026-09-20-10:00:00+00:00",
    # data
    "20260920", "2026-09-20", "2026-09", "2026", "", "None", "2026-0920T10:00:00+00:00", "202609-20T10:00:00+00:00",
    "2026-13-01T10:00:00+00:00", "2026-00-10T10:00:00+00:00", "2026-09-31T10:00:00+00:00",
    "0000-01-01T00:00:00+00:00", "0001-01-01T00:00:00+00:01", "9999-12-31T23:59:59.999999-23:59",
    "２０２６-09-20T10:00:00+00:00", "2026-263T10:00:00+00:00", "2026263T10:00:00+00:00",
    # semana ISO
    "2026-W38-7T10:00:00+00:00", "2026W387T10:00:00+00:00", "2026-W38T10:00:00+00:00", "2026W38T10:00:00+00:00",
    "2026-W53-1T10:00:00+00:00", "2025-W53-1T10:00:00+00:00", "2020-W53-7T10:00:00+00:00", "2026-W00-1T10:00:00+00:00",
    "2026-W38-0T10:00:00+00:00", "2026-W38-8T10:00:00+00:00", "0000-W01-1T10:00:00+00:00",
    "9999-W52-7T10:00:00+00:00", "9999-W52-5T10:00:00+00:00", "2026-W38-10:00:00+00:00", "2026W381000+00:00",
    "2026-W38-", "2026-W38", "2026W38", "2026W3871000+00:00", "2026W38T1000+00:00", "2026W38T1000005+00:00",
    "2026-09-20T10:00:00.1234567",
]
TIMESTAMPS = list(dict.fromkeys(TIMESTAMPS))  # sem repetidos, na ordem

# `fromisoformat` sem o `.replace("Z", "+00:00")`: o `Z` do próprio parser só
# aparece aqui, porque o código de produção troca todo `Z` antes.
RAW_ISO = [
    "2026-09-20T10:00:00Z", "2026-09-20T10:00:00Z0", "2026-09-20T10:00:00z", "2026-09-20T10:00:00ZZ",
    "2026-09-20T10:00Z", "2026-09-20T10Z", "2026-09-20TZ", "2026-09-20T10:00:00.5Z", "2026-09-20T10:00:00 Z",
    "2026-09-20T10:00:00Z+00:00", "2026-09-20T10:00:00+00:00Z",
]

FEATURED_SCORE_REPOS = [
    repo("Projeto-Baluarte", "desc", homepage="https://x.org", size=50000, pushed_at=ago(5)),
    repo("Veritas", "desc", size=0, pushed_at=ago(100)),
    repo("baluarte-obra-segura", None, size=None, pushed_at=ago(400)),
    repo("unknown", "", size=1234, pushed_at=ago(90)),
    repo("unknown", "", size=1234, pushed_at=ago(91)),
    repo("unknown", "", size=1234, pushed_at=ago(365)),
    repo("unknown", "", size=1234, pushed_at=ago(366)),
    repo("unknown", "", size=29999, fork=True, pushed_at=None, updated_at=None),
    repo("unknown", "", size=30001, pushed_at="not a date"),
    repo("unknown", "", size=333, homepage="https://y.org", pushed_at=None, updated_at=ago(1)),
    repo("Digital-Logic-Sim-CE", "x", size=7, fork=True, pushed_at="2026-09-20T10:00:00"),
]


def check(status: str | None, http: int = 200, final: str | None = None) -> dict[str, Any] | None:
    if status is None:
        return None
    return {"status": status, "http_status": http, "final_url": final}


MARKETING_CASES: list[dict[str, Any]] = []
for flags in ({}, {"private": True}, {"fork": True}, {"archived": True}, {"private": True, "fork": True, "archived": True}):
    for chk in (None, check("verified"), check("unreachable", 404)):
        for order in (None, 1, 2, 9, 10):
            for desc_ok in (True, False):
                MARKETING_CASES.append({"repo": repo("m", "", **flags), "check": chk, "featured_order": order,
                                        "featured_priority": None, "description_ok": desc_ok})
# Varredura completa de `priority`: os empates (p ≡ 2 mod 4) exercitam o
# arredondamento para o par do round() do Python.
for priority in list(range(-5, 0)) + list(range(0, 101)) + [150]:
    MARKETING_CASES.append({"repo": repo("m", ""), "check": check("verified"), "featured_order": 3,
                            "featured_priority": priority, "description_ok": True})

PRESENTATION_CASES = [
    {"repo": repo("live"), "check": check("verified"), "website": "https://live.example.org",
     "website_source": "github_homepage", "category": "Web", "status": "🟢 Active",
     "description": "Descrição pública longa o bastante.", "featured_order": 1, "featured_priority": 90},
    {"repo": repo("redirect"), "check": check("verified", 200, "https://www.redirect.example.org/"),
     "website": "https://redirect.example.org", "website_source": "manifest", "category": "Ecossistema Baluarte",
     "status": "🟡 In Development", "description": "Curta.", "featured_order": None, "featured_priority": None},
    {"repo": repo("same-final"), "check": check("verified", 200, "https://same.example.org"),
     "website": "https://same.example.org", "website_source": "github_homepage", "category": "Games",
     "status": "🟢 Active", "description": "x" * 25, "featured_order": None, "featured_priority": None},
    {"repo": repo("down"), "check": check("unreachable", 404), "website": "https://down.example.org",
     "website_source": "github_homepage", "category": "Web", "status": "🔵 Experimental",
     "description": "—", "featured_order": 2, "featured_priority": None},
    {"repo": repo("refused"), "check": check("unreachable", 0), "website": "https://refused.example.org",
     "website_source": "manifest", "category": "Academia", "status": "🟣 Academic",
     "description": "   ", "featured_order": None, "featured_priority": None},
    {"repo": repo("private", private=True), "check": check("verified"), "website": "https://private.example.org",
     "website_source": "github_homepage", "category": "IA & Automação", "status": "🔒 Private",
     "description": "Privado com site verificado nunca publica o site.", "featured_order": None, "featured_priority": None},
    {"repo": repo("no-site"), "check": None, "website": None, "website_source": "none",
     "category": "Software & Ferramentas", "status": "🟢 Active",
     "description": "Sem site nenhum, só o código.", "featured_order": None, "featured_priority": None},
    {"repo": repo("unknown-label", fork=True, archived=True), "check": None, "website": None,
     "website_source": "none", "category": "Rótulo Desconhecido", "status": "⚪ Archived",
     "description": "com\x1fseparador de 25+ caracteres", "featured_order": None, "featured_priority": None},
    {"repo": repo("acentos"), "check": check("verified"), "website": "https://acentos.example.org",
     "website_source": "github_homepage", "category": "Web", "status": "🟢 Active",
     "description": "Ação útil é ótima — sim", "featured_order": None, "featured_priority": None},
    {"repo": repo("Nome Com Espaços_e-Símbolos!"), "check": None, "website": None, "website_source": "none",
     "category": "Experimentos", "status": "🔵 Experimental", "description": "\x1f" * 30,
     "featured_order": None, "featured_priority": None},
]

DISCOVERY_CASES = [
    {"repo": repo("h", homepage="https://home.example.org"), "overrides": {f"{OWNER}/h": "https://manifest.example.org"}},
    {"repo": repo("h", homepage="  https://home.example.org  "), "overrides": {}},
    {"repo": repo("h", homepage="not a url"), "overrides": {f"{OWNER}/h": "https://manifest.example.org"}},
    {"repo": repo("h", homepage=""), "overrides": {f"{OWNER}/h": {"website": " https://obj.example.org "}}},
    {"repo": repo("h", homepage=None), "overrides": {f"{OWNER}/h": {"website": ""}}},
    {"repo": repo("h", homepage=None), "overrides": {f"{OWNER}/h": {"note": "sem website"}}},
    {"repo": repo("h", homepage=None), "overrides": {f"{OWNER}/h": "ftp://manifest.example.org"}},
    {"repo": repo("h", homepage=None), "overrides": {f"{OWNER}/outro": "https://outro.example.org"}},
    {"repo": repo("h", homepage=None), "overrides": ["não", "é", "objeto"]},
    {"repo": repo("h", homepage=None), "overrides": {f"{OWNER}/h": 42}},
    {"repo": repo("h", homepage="https://home.example.org\n"), "overrides": {}},
    {"repo": repo("h", homepage="\x1fhttps://home.example.org"), "overrides": {}},
]


# Manifesto `docs/README_SITES.json` nos dois formatos, e o que não é nenhum
# deles. Float, lista e objeto em `website` ficam de fora de propósito: ali o
# Rust diverge na conversão para texto (documentado em discovery.rs).
OVERRIDES = [
    {f"{OWNER}/a": "https://a.example.org", f"{OWNER}/b": {"website": "https://b.example.org", "note": "x"}},
    {f"{OWNER}/a": {"website": ""}, f"{OWNER}/b": {"website": None}, f"{OWNER}/c": {"website": False}},
    {f"{OWNER}/a": {"website": 0}, f"{OWNER}/b": {"website": []}, f"{OWNER}/c": {"website": {}}},
    {f"{OWNER}/a": {"website": True}, f"{OWNER}/b": {"website": 42}, f"{OWNER}/c": {"website": -1}},
    {f"{OWNER}/a": {"note": "sem website"}, f"{OWNER}/b": 42, f"{OWNER}/c": None, f"{OWNER}/d": ["https://x.org"]},
    {f"{OWNER}/a": "", f"{OWNER}/b": "  ", f"{OWNER}/c": "não é url"},
    {}, [], "texto", None, 42,
]


# ---------------------------------------------------------------- saídas

def _check_obj(data: dict[str, Any] | None, url: str | None):
    if data is None:
        return None
    return PC.WebsiteCheck(url or "", data["status"], data["http_status"], data["final_url"] or (url or ""), "fixture")


def py_timestamp(value: str, *, replace_z: bool = True) -> dict[str, Any]:
    """A leitura de carimbo de `status_for`/`featured_score`, exposta caso a caso."""
    try:
        parsed = datetime.fromisoformat(str(value).replace("Z", "+00:00") if replace_z else value)
    except ValueError:
        return {"kind": "invalid"}
    if parsed.utcoffset() is None:
        return {"kind": "naive"}
    return {"kind": "aware", "micros_before_now": (NOW - parsed) // timedelta(microseconds=1)}


def build() -> dict[str, Any]:
    cases: dict[str, list[dict[str, Any]]] = {}
    cases["looks_like_http_url"] = [{"input": u, "expected": PC._looks_like_http_url(u)} for u in URLS]
    cases["slug"] = [
        {"input": n, "expected": PC.Presentation(
            repository="", name=n, description="", category="", category_label="", website=None,
            website_declared=None, website_status="none", website_http_status=0, website_source="none",
            website_final_url="", github="", primary_cta="github", secondary_cta=None, status="",
            private=False, featured=False, marketing_priority=0).slug}
        for n in SLUG_NAMES
    ]
    cases["describe"] = [{"input": d, "expected": UP.describe({"description": d})} for d in DESCRIPTIONS]
    cases["canonical_category"] = [{"input": label, "expected": PC.canonical_category(label)} for label in LABELS]
    cases["classify"] = [{"input": r, "expected": UP.classify(r)} for r in CLASSIFY_REPOS]
    cases["status_for"] = [
        {"input": {"repo": r, "category": c}, "expected": UP.status_for(r, c, NOW)} for r, c in STATUS_CASES
    ]
    cases["github_timestamp"] = [{"input": v, "expected": py_timestamp(v)} for v in TIMESTAMPS]
    cases["fromisoformat"] = [{"input": v, "expected": py_timestamp(v, replace_z=False)} for v in RAW_ISO]
    cases["featured_score"] = [{"input": r, "expected": UP.featured_score(r, NOW)} for r in FEATURED_SCORE_REPOS]
    cases["marketing_priority"] = [
        {"input": c, "expected": PC.marketing_priority(
            c["repo"], _check_obj(c["check"], "https://m.example.org"),
            featured_order=c["featured_order"], featured_priority=c["featured_priority"],
            description_ok=c["description_ok"])}
        for c in MARKETING_CASES
    ]
    cases["resolve_presentation"] = [
        {"input": c, "expected": PC.catalog_entry(PC.resolve_presentation(
            c["repo"], check=_check_obj(c["check"], c["website"]), website=c["website"],
            website_source=c["website_source"], category=c["category"], status=c["status"],
            description=c["description"], featured_order=c["featured_order"],
            featured_priority=c["featured_priority"]))}
        for c in PRESENTATION_CASES
    ]
    cases["normalize_site_overrides"] = [
        {"input": raw, "expected": {key: entry["website"] for key, entry in PC.normalize_site_overrides(raw).items()}}
        for raw in OVERRIDES
    ]
    cases["discover_project_website"] = [
        {"input": c, "expected": list(PC.discover_project_website(c["repo"], PC.normalize_site_overrides(c["overrides"])))}
        for c in DISCOVERY_CASES
    ]
    # Todo code point que str.isspace() considera espaço: o Rust confere o
    # py_is_space() contra esta lista em todos os 1,1 milhão de code points.
    cases["python_whitespace"] = [c for c in range(0x110000) if chr(c).isspace()]
    return {
        "schema": "lucas-belucci-bellini/parity-domain@1",
        "note": "Gerado por tests/test_parity_domain.py a partir das funções Python. Não editar à mão.",
        "now": NOW.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "cases": cases,
    }


def render(data: dict[str, Any]) -> str:
    return json.dumps(data, ensure_ascii=False, indent=2) + "\n"


class ParityFixtureTests(unittest.TestCase):
    def test_fixture_reflete_o_python_atual(self) -> None:
        rendered = render(build())
        if os.environ.get("UPDATE_PARITY") == "1":
            FIXTURE.parent.mkdir(parents=True, exist_ok=True)
            FIXTURE.write_text(rendered, encoding="utf-8")
        self.assertEqual(
            FIXTURE.read_text(encoding="utf-8"), rendered,
            "tests/fixtures/parity/domain.json não bate com o Python; se a mudança foi de propósito, "
            "rode UPDATE_PARITY=1 e revise o diff (o Rust vai precisar acompanhar)",
        )

    def test_armadilhas_estao_cobertas(self) -> None:
        # Se um destes casos sumir, a paridade deixa de provar o que promete.
        data = build()["cases"]
        prioridades = {c["input"]["featured_priority"]: c["expected"] for c in data["marketing_priority"]
                       if c["input"]["featured_priority"] is not None}
        self.assertEqual(45 + 13 + 10 + 10 + 5 + 5, prioridades[50] + 1)  # round(12.5) == 12 no Python
        self.assertTrue(any(c["input"] == "https://example.org\n" and c["expected"] for c in data["looks_like_http_url"]))
        self.assertTrue(any(c["input"] and "\x1f" in c["input"] for c in data["describe"]))
        self.assertEqual("IA & Automação", next(c["expected"] for c in data["classify"] if c["input"]["name"] == "DailyPlanner"))


if __name__ == "__main__":
    unittest.main()
