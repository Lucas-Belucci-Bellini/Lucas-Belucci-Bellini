"""Casos de paridade do monitor de commits: o Python é a referência, o Rust confere.

    OLD PYTHON ──▶ tests/fixtures/parity/monitor.json ◀── NEW RUST

Cada cenário roda o `main()` de verdade do `.github/scripts/ecosystem_watch.py`
com a função `api()` trocada por respostas fixas (JSON, `HTTPError`,
`URLError`, `TimeoutError`). O que fica registrado é o que o Python produziu:
a saída padrão, o `ECOSYSTEM-COMMIT-STATE.json` e o `ECOSYSTEM-COMMIT-MONITOR.md`
byte a byte (ou que não os reescreveu), ou a exceção que o derrubou.

    cargo test -p profile-core --test parity_commits

confere o `profile-core sync commits` contra os mesmos cenários. Novas
tentativas, timeouts de verdade e paginação pela rede ficam no teste de
ponta a ponta (tests/e2e/github_parity.py) e no crate github-client.

**Versão:** as mensagens de `TypeError`/`AttributeError` gravadas no estado
são do CPython do CI (3.12); o teste só compara na mesma versão menor.

    UPDATE_PARITY=1 python3.12 -m unittest tests.test_parity_monitor
"""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import sys
import tempfile
import unittest
import urllib.error
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "parity" / "monitor.json"
SCRIPT = ROOT / ".github" / "scripts" / "ecosystem_watch.py"
REFERENCE_MINOR = (3, 12)
NOW = "2026-09-26T07:17:00Z"
USER = "o"


def _load():
    spec = importlib.util.spec_from_file_location("ecosystem_watch_parity", SCRIPT)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def sha(seed: str) -> str:
    return (seed * 40)[:40]


def commit(sha_value: Any, message: Any = "feat: x", date: Any = "2026-09-20T10:00:00Z", **extra: Any) -> list:
    entry = {"sha": sha_value, "commit": {"committer": {"date": date}, "message": message},
             "html_url": f"https://github.com/{USER}/r/commit/{sha_value}"}
    entry.update(extra)
    return [entry]


def listing(*repos: dict[str, Any]) -> dict[str, Any]:
    return {f"/users/{USER}/repos?type=owner&per_page=100&page=1": {"json": list(repos)}}


def commits_path(name: str, branch: str = "main") -> str:
    return f"/repos/{USER}/{name}/commits?sha={branch}&per_page=1"


def compare_path(name: str, base: str, head: str) -> str:
    return f"/repos/{USER}/{name}/compare/{base}...{head}"


def state(repositories: dict[str, Any], metrics: dict[str, Any] | None = None) -> str:
    data: dict[str, Any] = {"schema": 4, "scanned_at": "2026-09-26T06:17:00Z", "scan_interval": "hourly",
                            "repositories": repositories}
    if metrics is not None:
        data["metrics"] = metrics
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


def entry(sha_value: str, message: str = "feat: x", branch: str = "main") -> dict[str, Any]:
    return {"branch": branch, "sha": sha_value, "date": "2026-09-20T10:00:00Z", "message": message,
            "url": f"https://github.com/{USER}/r/commit/{sha_value}"}


A1, A2, B1, C1, D1, E1, F1 = (sha(c) for c in "abcdef1")
METRICS = {"tracked_commits": 1700, "project_commits": 1650, "monitor_commits": 50}

SCENARIOS: list[dict[str, Any]] = [
    {
        "name": "primeira varredura: filtros, repositório vazio, 409 e formatos estranhos",
        "previous": None,
        "api": {
            **listing(
                {"name": "alpha", "default_branch": "main"},
                {"name": "beta", "default_branch": None},
                {"name": "conflito"},
                {"name": "fork", "fork": True},
                {"name": "segredo", "private": True},
                {"name": "O"},
                {"name": "o-irmao"},
                {"name": "dev-branch", "default_branch": "feature/x y"},
                {"name": "mensagens"},
                {"name": "vazia-dict"},
                {"name": "dict-cheio"},
                {"name": "texto"},
                {"name": "nulo-na-lista"},
                {"name": "commit-nulo"},
                {"name": "sem-mensagem"},
                {"name": "mensagem-nula"},
                {"name": "sha-numero"},
                {"name": "sem-sha"},
                {"name": "sem-data"},
                {"name": "timeout"},
                {"name": "recusado"},
            ),
            commits_path("alpha"): {"json": commit(A1, "feat: primeira\n\ncorpo longo")},
            commits_path("beta"): {"json": []},
            commits_path("conflito"): {"http": 409, "reason": "Conflict"},
            commits_path("o-irmao"): {"json": commit(B1)},
            commits_path("dev-branch", "feature/x%20y"): {"json": commit(C1, "branch com barra e espaço")},
            commits_path("mensagens"): {"json": commit(D1, "é" * 200 + "\nresto")},
            commits_path("vazia-dict"): {"json": {}},
            commits_path("dict-cheio"): {"json": {"message": "Not Found"}},
            commits_path("texto"): {"json": "abc"},
            commits_path("nulo-na-lista"): {"json": [None]},
            commits_path("commit-nulo"): {"json": [{"sha": E1, "commit": None}]},
            commits_path("sem-mensagem"): {"json": commit(E1, "")},
            commits_path("mensagem-nula"): {"json": commit(E1, None)},
            commits_path("sha-numero"): {"json": commit(5)},
            commits_path("sem-sha"): {"json": [{"commit": {"message": "m"}}]},
            commits_path("sem-data"): {"json": [{"sha": F1, "commit": {"message": "m"}}]},
            commits_path("timeout"): {"timeout": True},
            commits_path("recusado"): {"url_error": "[Errno 111] Connection refused"},
        },
    },
    {
        "name": "mensagem de erro longa é cortada em 180 caracteres",
        "previous": None,
        "api": {**listing({"name": "alpha"}), commits_path("alpha"): {"http": 500, "reason": "motivo " * 40}},
    },
    {
        "name": "99 itens: uma página só",
        "previous": None,
        "api": {
            f"/users/{USER}/repos?type=owner&per_page=100&page=1": {
                "json": [{"name": f"r{i:03d}", "fork": True} for i in range(98)] + [{"name": "unico"}]},
            commits_path("unico"): {"json": commit(A1)},
        },
    },
    {
        "name": "mudanças: contagem, falha da comparação, repositório novo e sumido",
        "previous": state({
            "alpha": entry(A1), "beta": entry(B1), "gama": entry(C1), "delta": entry(D1),
            "erro-antes": {"branch": "main", "error": "HTTP Error 409: Conflict"},
            "sumiu": entry(E1), "Zeta": entry(A1), "Ábaco": entry(A1), "_sub": entry(A1),
        }, METRICS),
        "api": {
            **listing(*({"name": n} for n in (
                "alpha", "beta", "gama", "delta", "erro-antes", "novo", "Zeta", "Ábaco", "_sub"))),
            commits_path("alpha"): {"json": commit(A2, "fix: segunda")},
            compare_path("alpha", A1, A2): {"json": {"ahead_by": 3, "status": "ahead"}},
            commits_path("beta"): {"json": commit(A2)},
            compare_path("beta", B1, A2): {"http": 404, "reason": "Not Found"},
            commits_path("gama"): {"json": commit(C1)},
            commits_path("delta"): {"json": commit(A2)},
            compare_path("delta", D1, A2): {"json": {"ahead_by": " 7 "}},
            commits_path("erro-antes"): {"json": commit(F1)},
            commits_path("novo"): {"json": commit(F1)},
            commits_path("Zeta"): {"json": commit(A2)},
            compare_path("Zeta", A1, A2): {"json": {"status": "diverged"}},
            commits_path("Ábaco".replace("Á", "%C3%81")): {"json": commit(A2)},
            compare_path("Ábaco".replace("Á", "%C3%81"), A1, A2): {"json": {"ahead_by": None}},
            commits_path("_sub"): {"json": commit(A2)},
            compare_path("_sub", A1, A2): {"json": {"ahead_by": 2.9}},
        },
    },
    {
        "name": "sem mudança semântica: nada reescrito",
        "previous": state({"alpha": entry(A1)}, METRICS),
        "api": {**listing({"name": "alpha"}), commits_path("alpha"): {"json": commit(A1)}},
    },
    {
        "name": "estado ilegível volta ao baseline 1538",
        "previous": "{isto não é json",
        "api": {**listing({"name": "alpha"}), commits_path("alpha"): {"json": commit(A1)}},
    },
    {
        "name": "métricas em texto e sem metrics",
        "previous": state({"alpha": entry(A1)}, {"project_commits": "2000", "monitor_commits": " 5"}),
        "api": {**listing({"name": "alpha"}), commits_path("alpha"): {"json": commit(A2)},
                compare_path("alpha", A1, A2): {"json": {"ahead_by": True}}},
    },
    {
        "name": "estado sem metrics conta a partir do baseline",
        "previous": state({"alpha": entry(A1)}),
        "api": {**listing({"name": "alpha"}), commits_path("alpha"): {"json": commit(A2)},
                compare_path("alpha", A1, A2): {"json": {"ahead_by": 1}}},
    },
    {
        "name": "métrica ilegível derruba antes da rede",
        "previous": state({"alpha": entry(A1)}, {"project_commits": "abc"}),
        "api": {},
    },
    {
        "name": "estado que não é objeto derruba",
        "previous": "[]\n",
        "api": {},
    },
    {
        "name": "listagem que falha derruba sem gravar",
        "previous": state({"alpha": entry(A1)}, METRICS),
        "api": {f"/users/{USER}/repos?type=owner&per_page=100&page=1": {"http": 502, "reason": "Bad Gateway"}},
    },
    {
        "name": "duas páginas: página cheia pede a próxima",
        "previous": None,
        "api": {
            f"/users/{USER}/repos?type=owner&per_page=100&page=1": {
                "json": [{"name": f"r{i:03d}", "fork": i % 10 == 0} for i in range(100)]},
            f"/users/{USER}/repos?type=owner&per_page=100&page=2": {"json": [{"name": "ultimo"}]},
            **{commits_path(f"r{i:03d}"): {"json": []} for i in range(100)},
            commits_path("ultimo"): {"json": commit(A1)},
        },
    },
]


def run(module, scenario: dict[str, Any]) -> dict[str, Any]:
    responses = scenario["api"]
    calls: list[str] = []

    def api(path: str):
        calls.append(path)
        response = responses.get(path)
        if response is None:
            raise urllib.error.HTTPError(module.API + path, 404, "Not Found", None, None)
        if "json" in response:
            return json.loads(json.dumps(response["json"]))
        if "http" in response:
            raise urllib.error.HTTPError(module.API + path, response["http"], response["reason"], None, None)
        if "url_error" in response:
            errno, text = response["url_error"].removeprefix("[Errno ").split("] ", 1)
            raise urllib.error.URLError(ConnectionRefusedError(int(errno), text))
        raise TimeoutError("timed out")

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        docs = root / "docs"
        docs.mkdir()
        if scenario["previous"] is not None:
            (docs / "ECOSYSTEM-COMMIT-STATE.json").write_text(scenario["previous"], encoding="utf-8")
        module.api = api
        module.USER, module.PROFILE_REPOSITORY_NAME, module.TOKEN = USER, USER, ""
        out = io.StringIO()
        result: dict[str, Any] = {}
        try:
            with contextlib.redirect_stdout(out):
                module.main(["--root", str(root), "--now", NOW])
        except Exception as exc:  # noqa: BLE001 — o que derruba o script é o resultado
            result["crash"] = type(exc).__name__
        result["stdout"] = out.getvalue()
        # A sequência de chamadas também é paridade: chamada a mais gasta rate limit.
        result["calls"] = calls
        state_file = docs / "ECOSYSTEM-COMMIT-STATE.json"
        state_text = state_file.read_text(encoding="utf-8") if state_file.exists() else None
        result["state"] = None if state_text == scenario["previous"] else state_text
        report = docs / "ECOSYSTEM-COMMIT-MONITOR.md"
        result["report"] = report.read_text(encoding="utf-8") if report.exists() else None
        return result


def build() -> dict[str, Any]:
    module = _load()
    cases = [{"name": s["name"], "input": {"previous": s["previous"], "api": s["api"]}, "expected": run(module, s)}
             for s in SCENARIOS]
    return {
        "schema": "lucas-belucci-bellini/parity-monitor@1",
        "note": "Gerado por tests/test_parity_monitor.py a partir do ecosystem_watch.py real. Não editar à mão.",
        "python": sys.version.split()[0],
        "now": NOW,
        "user": USER,
        "cases": cases,
    }


def render(data: dict[str, Any]) -> str:
    return json.dumps(data, ensure_ascii=False, indent=2) + "\n"


class MonitorParityTests(unittest.TestCase):
    def setUp(self) -> None:
        if sys.version_info[:2] != REFERENCE_MINOR:
            self.skipTest(f"fixture do CPython {REFERENCE_MINOR[0]}.{REFERENCE_MINOR[1]} (o do CI)")

    def test_fixture_reflete_o_python_atual(self) -> None:
        data = build()
        if os.environ.get("UPDATE_PARITY") == "1":
            FIXTURE.write_text(render(data), encoding="utf-8")
        recorded = json.loads(FIXTURE.read_text(encoding="utf-8"))
        data["python"] = recorded["python"]
        self.assertEqual(
            FIXTURE.read_text(encoding="utf-8"), render(data),
            "monitor.json não bate com o ecosystem_watch.py atual. Se o monitor mudou de propósito, "
            "regenere com a versão do CI e porte a mudança para o Rust no mesmo PR.",
        )

    def test_armadilhas_estao_cobertas(self) -> None:
        cases = {c["name"]: c["expected"] for c in build()["cases"]}
        first = json.loads(cases["primeira varredura: filtros, repositório vazio, 409 e formatos estranhos"]["state"])
        repositories = first["repositories"]
        self.assertNotIn("fork", repositories)
        self.assertNotIn("segredo", repositories)
        self.assertNotIn("O", repositories, "o perfil fica de fora sem diferenciar caixa")
        self.assertEqual("HTTP Error 409: Conflict", repositories["conflito"]["error"])
        self.assertEqual({"branch": "main", "empty": True}, repositories["beta"])
        self.assertEqual("list index out of range", repositories["sem-mensagem"]["error"])
        self.assertEqual(140, len(repositories["mensagens"]["message"]))
        self.assertEqual(5, repositories["sha-numero"]["sha"])
        # Estado ilegível zera o contador acumulado (achado A23).
        broken = json.loads(cases["estado ilegível volta ao baseline 1538"]["state"])
        self.assertEqual(1538, broken["metrics"]["project_commits"])
        self.assertIsNone(cases["sem mudança semântica: nada reescrito"]["state"])
        self.assertEqual("ValueError", cases["métrica ilegível derruba antes da rede"].get("crash"))
        self.assertEqual("HTTPError", cases["listagem que falha derruba sem gravar"].get("crash"))


if __name__ == "__main__":
    unittest.main()
