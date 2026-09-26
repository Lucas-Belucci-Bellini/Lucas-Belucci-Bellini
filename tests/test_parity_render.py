"""Casos de paridade do README: o Python é a referência, o Rust confere.

    OLD PYTHON ──▶ tests/fixtures/parity/render.json ◀── NEW RUST

Cada cenário é uma raiz sintética inteira (inventário, linguagens, checagens,
manifestos e o README com os marcadores) sobre a qual o `update_profile.py`
real roda como no CI. O que fica registrado é o que o Python produziu: a
saída padrão e o README, o `profile-snapshot.svg` e o catálogo byte a byte,
ou o código de saída e a exceção que o derrubou.

    cargo test -p profile-core --test parity_render

roda o `profile-core render readme` sobre as mesmas raízes. O golden
(`tests/fixtures/profile`) é o caso "bem comportado"; aqui ficam as bordas:
empates, repetidos, nomes que só diferem na caixa, manifestos tortos,
datas com fuso, README com CRLF, estados vazios e os tracebacks.

**Versão:** a saída depende só de regras do próprio gerador (e do
`fromisoformat` do 3.12, nas datas); o teste só compara no CPython do CI.

    UPDATE_PARITY=1 python3.12 -m unittest tests.test_parity_render
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "parity" / "render.json"
SCRIPT = ROOT / "scripts" / "update_profile.py"
REFERENCE_MINOR = (3, 12)
NOW = "2026-09-25T12:00:00Z"
OUTPUTS = ("README.md", "assets/profile-snapshot.svg", "docs/project-catalog.json")
MARKERS = (
    "PROFILE-DASHBOARD", "WHAT-I-BUILD", "PRODUCT-CARDS", "FEATURED-PROJECTS", "WEBSITE-DIRECTORY",
    "ARSENAL-STACK", "LANGUAGE-BADGES", "LANGUAGE-STATS", "PUBLIC-PROJECTS", "PRIVATE-PROJECTS",
    "LIVE-PROJECTS", "ECOSYSTEM-MAP", "PROJECT-MAP",
)


def repo(name: str, description: Any = None, *, owner: str = "o", pushed_at: Any = "2026-09-20T10:00:00Z",
         **extra: Any) -> dict[str, Any]:
    data: dict[str, Any] = {
        "name": name, "full_name": f"{owner}/{name}", "description": description, "private": False,
        "fork": False, "archived": False, "homepage": None, "pushed_at": pushed_at, "size": 10,
    }
    data.update(extra)
    return data


def dumps(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2)


def readme(*, extra: str = "", missing: str | None = None, newline: str = "\n") -> str:
    parts = ["# Perfil de teste", "", "Texto fixo com \\1 e \\g<0> que não é bloco.", ""]
    for marker in MARKERS:
        if marker == missing:
            continue
        parts += [f"## {marker.title()}", f"<!-- {marker}:START -->", "conteúdo antigo", f"<!-- {marker}:END -->", ""]
    parts.append(extra)
    return newline.join(parts)


def tree(repos: list[dict[str, Any]], *, featured: Any = None, stack: Any = None, sites: Any = None,
         excluded: Any = None, checks: dict[str, Any] | None = None, languages: dict[str, Any] | None = None,
         template: str | None = None) -> dict[str, str]:
    files = {
        "repos.json": dumps(repos),
        "README.md": readme() if template is None else template,
        "docs/README_EXCLUDED.json": dumps({"repositories": []} if excluded is None else excluded),
        "docs/README_FEATURED.json": dumps({"projects": []} if featured is None else featured),
        "docs/README_SITES.json": dumps({} if sites is None else sites),
        "site-checks.json": dumps(checks or {}),
    }
    if stack is not None:
        files["docs/README_STACK.json"] = stack if isinstance(stack, str) else dumps(stack)
    for full_name, value in (languages or {}).items():
        files[f"languages/{full_name.replace('/', '__')}.json"] = value if isinstance(value, str) else dumps(value)
    return files


def ok(url: str, status: int = 200, final: str | None = None) -> dict[str, Any]:
    return {"status": "verified", "http_status": status, "final_url": final or url}


FILE_ARGS = ["--input-repos", "{root}/repos.json", "--languages-dir", "{root}/languages",
             "--site-checks-fixture", "{root}/site-checks.json"]


# ------------------------------------------------------------ cenário cheio

def vitrine_cheia() -> dict[str, Any]:
    tools = [f"tool-{n:02d}" for n in range(1, 12)]
    repos = [
        repo("Veritas", "Calculadora de tabelas verdade.", homepage="https://veritas.example.org", size=4000),
        repo("Projeto-Baluarte", "Plataforma baluarte.", homepage=" https://baluarte.example.org/ ", size=90000),
        repo("demo-site-a", "Portfolio site para mostrar trabalho.", homepage="https://a.example.org",
             pushed_at="2026-09-24T23:30:00-03:00"),
        # Mesmo instante que o de cima, noutro fuso: o empate fica com o primeiro.
        repo("demo-same-instant", "Mesmo instante, outro fuso.", pushed_at="2026-09-25T02:30:00Z"),
        repo("demo-site-b", "Página publicada pelo manifesto."),
        repo("demo-site-c", "Site c com construção.", homepage="https://c.example.org"),
        repo("demo-site-d", "Site d.", homepage="https://d.example.org", pushed_at="2025-06-01T10:00:00Z"),
        repo("demo-site-e", "Site e com convite.", homepage="https://e.example.org", pushed_at="2023-01-01T00:00:00Z"),
        repo("demo-manifest-obj", "Site declarado como objeto no manifesto."),
        repo("zeta-down", "Outro site fora do ar, antes no inventário.", homepage="https://zeta-down.example.org"),
        repo("demo-down", "Site fora do ar.", homepage="https://down.example.org"),
        repo("demo-invalid", "Site com URL que o monitor recusa.", homepage="https://bad host.example.org"),
        repo("Alpha", "Ferramenta de linha de comando para testes."),
        repo("alpha", "Ferramenta de linha de comando para testes."),
        repo("dup", "Repositório de mesmo nome, dono o."),
        repo("dup", "Repositório de mesmo nome, dono outro.", owner="outro"),
        repo("demo-long", ("Descrição longa que passa do limite do card, " * 5).strip() + " fim."),
        repo("demo-nospace", "x" * 180),
        repo("demo-exact", "y" * 150),
        repo("demo-pipe", "Usa | pipes, \\1 e \\g<0>, <b>&amp;</b>\ncom   quebra."),
        repo("demo-game", "A game about survival."),
        # Mesma prioridade e mesmo nome sem caixa: o max() fica com o primeiro.
        repo("Arena", "A game arena for testing parity."),
        repo("arena", "A game arena for testing parity."),
        repo("demo-course", "Atividade da disciplina."),
        repo("demo-ai", "Assistente jarvis local."),
        repo("demo-backend", "backend e banco de dados."),
        repo("demo-chips", "chips de lógica digital."),
        repo("baluarte-x", "Domínio extraído."),
        repo("demo-fork", None, fork=True, pushed_at=None),
        repo("demo-archived", "Projeto antigo.", archived=True, pushed_at="2020-01-01T00:00:00Z"),
        *(repo(name, f"Utilitário número {name[-2:]}.") for name in tools),
        repo("segredo", "Privado | com\nquebra\re retorno", private=True, visibility="private",
             homepage="https://segredo.example.org"),
        repo("segredo-2", None, private=True, visibility="private"),
        repo("demo-site-c", "Site c com construção.", homepage="https://c.example.org"),  # repetido
        repo("Lucas-Belucci-Bellini", "Perfil.", owner="Lucas-Belucci-Bellini", pushed_at="2026-09-25T11:59:00Z"),
        repo("demo-excluded", "Excluído por nome.", homepage="https://excluded.example.org"),
        repo("excluded-full", "Excluído pelo nome completo.", owner="outro"),
    ]
    checks = {
        "https://veritas.example.org": ok("https://veritas.example.org"),
        "https://baluarte.example.org/": ok("https://baluarte.example.org/", 301, "https://www.baluarte.example.org/"),
        "https://a.example.org": ok("https://a.example.org"),
        "https://b.example.org": ok("https://b.example.org"),
        "https://c.example.org": ok("https://c.example.org"),
        "https://d.example.org": ok("https://d.example.org", 308, "https://d.example.org/"),
        "https://e.example.org": ok("https://e.example.org"),
        "https://obj.example.org": ok("https://obj.example.org"),
        "https://down.example.org": {"status": "unreachable", "http_status": 503},
        "https://zeta-down.example.org": {"status": "unreachable", "http_status": 500},
        "https://bad host.example.org": {"status": "invalid"},
    }
    languages = {
        "o/Veritas": {"TypeScript": 2_500_000, "HTML": 40_000, "CSS": 9_000},
        "o/Projeto-Baluarte": {"JavaScript": 1_200_000, "Python": 300_000, "PLpgSQL": 5_000, "Batchfile": 800,
                               "Dockerfile": 120, "Shell": 50},
        "o/demo-site-c": {"HTML": 1_000},
        "o/demo-game": {"C#": 30_000, "ShaderLab": 4_000},
        "o/demo-ai": {"Jupyter Notebook": 70_000, "Python": 1_000},
        "o/tool-01": {"Foo": 5_000},
        "o/tool-02": {"foo": 5_000, "GDScript": 5_000},
        "o/tool-03": {"Zig": 0},
        "o/tool-04": {"Ünïcode": 12},
        "o/tool-05": {"PowerShell": 2048},
        "o/tool-06": {"Java": 1023},
        "o/tool-07": {"Rust": 1_048_576},
        "o/tool-08": {"Til~de": 3},
        "outro/dup": {"Go": 700},
        "o/segredo": {"Rust": 999, "Zig": 1},
        "o/demo-excluded": {"COBOL": 10_000_000},
    }
    featured = {
        "intro": "Curadoria | editorial com pipe.",
        "projects": [
            {"name": "demo-site-a", "order": 3, "label": "LANÇAMENTO | WEB",
             "focus": "Foco   com\nespaços  e | pipe", "website_required": True},
            {"name": "Veritas", "order": "1", "label": "LÓGICA"},
            {"name": "demo-down", "order": 2, "label": None, "website_required": True},
            {"name": "segredo", "order": 4, "label": "PRIVADO"},
            {"name": "fantasma", "order": 5},
            {"order": 6, "label": "SEM NOME"},
            {"name": "dup", "order": 7, "focus": 42},
            {"name": "demo-long", "focus": ""},
            {"name": "Projeto-Baluarte", "order": 1, "priority": 70, "label": "NÚCLEO", "focus": "  "},
        ],
    }
    stack = {"tools": [
        {"name": "Vite", "category": "Frameworks & Web", "family": "build | bundler", "evidence": "vite.config.js"},
        {"name": "Docker", "category": "Infraestrutura & DevOps", "family": "containers", "evidence": "Dockerfile"},
        {"name": "Zed", "category": "Zeta", "family": "editor"},
        {"name": "Awk", "category": "Alfa", "evidence": "scripts"},
        {"name": "Ábaco", "category": "ábaco"},
        {"name": 7, "category": "Hardware & Simulação", "family": None, "evidence": True},
        {"name": "Sem categoria"},
        {"name": "React", "category": "Frameworks & Web", "family": "ui", "evidence": "package.json"},
    ]}
    sites = {
        "o/demo-site-b": "https://b.example.org",
        "o/demo-down": "https://ignorado.example.org",
        "o/segredo": "https://segredo-manifesto.example.org",
        "o/demo-manifest-obj": {"website": "https://obj.example.org"},
    }
    template = readme(extra="<!-- PROJECT-MAP:START -->\nsegunda cópia, fica como está\n<!-- PROJECT-MAP:END -->\n")
    return {
        "name": "vitrine cheia: empates, repetidos, caixa, manifestos tortos, fuso",
        "files": tree(repos, featured=featured, stack=stack, sites=sites, checks=checks, languages=languages,
                      excluded={"repositories": ["demo-excluded", "outro/excluded-full"]}, template=template),
        "args": [*FILE_ARGS, "--now", NOW, "--write", "--catalog-out", "{root}/catalog-copy.json"],
        "outputs": [*OUTPUTS, "catalog-copy.json"],
    }


# ------------------------------------------------------------ bordas vazias

def sem_publicos() -> dict[str, Any]:
    repos = [
        repo("cofre", "Só metadados.", private=True, pushed_at="2026-09-20T10:00:00"),
        repo("cofre-2", None, private=True, pushed_at="2026-09-21T08:15:00"),
        repo("publico-excluido", "Excluído."),
    ]
    return {
        "name": "sem públicos: estados vazios, README com CRLF, datas sem fuso, sem checagem",
        "files": tree(repos, featured={"intro": None}, stack={}, excluded={"repositories": ["publico-excluido"]},
                      template=readme(newline="\r\n")),
        "args": ["--input-repos", "{root}/repos.json", "--skip-site-check", "--now", "2026-09-25T12:00:00+02:00",
                 "--out-dir", "{root}/out"],
        "outputs": ["out/README.md", "out/profile-snapshot.svg", "out/project-catalog.json"],
    }


def poucos_publicos() -> dict[str, Any]:
    exata = ("Descrição com exatamente cento e cinquenta caracteres " * 3)[:150]
    repos = [
        repo("um", exata, pushed_at=None),
        repo("dois", "curta", homepage="https://dois.example.org", pushed_at=None),
        repo("tres", "Jogo game de teste.", pushed_at=None, updated_at=None),
        repo("quatro", "Privado.", private=True, pushed_at=None),
        repo("cinco", "z" * 180, pushed_at=None),
        repo("seis", "Atividade, " * 20, pushed_at=None),
    ]
    return {
        "name": "poucos públicos: nenhum site no ar, carimbo do relógio com fuso, --out-dir",
        "files": tree(
            repos, featured={"projects": {}}, stack={"tools": ""},
            checks={"https://dois.example.org": {"status": "unreachable", "http_status": 404}},
            # Bytes que se anulam: o total cai no `or 1` e a participação passa de 100%.
            languages={"o/um": "{ não é json", "o/dois": {"Python": 10}, "o/tres": {"Python": "doze"},
                       "o/cinco": {"Neg": -10}},
        ),
        "args": [*FILE_ARGS, "--now", "2026-09-25T12:00:00-03:00", "--out-dir", "{root}/out"],
        "outputs": ["out/README.md", "out/profile-snapshot.svg", "out/project-catalog.json"],
    }


# ------------------------------------------------------------ tracebacks e recusas

def base(**overrides: Any) -> dict[str, str]:
    repos = overrides.pop("repos", [repo("a", "Projeto a.", homepage="https://a.example.org"), repo("b", "Projeto b.")])
    params: dict[str, Any] = {"featured": {"projects": [{"name": "a", "order": 1}]}, "stack": {"tools": []},
                              "checks": {"https://a.example.org": ok("https://a.example.org")}}
    params.update(overrides)
    return tree(repos, **params)


def falha(name: str, files: dict[str, str], *, drop: str | None = None, args: list[str] | None = None) -> dict[str, Any]:
    if drop:
        files = {path: text for path, text in files.items() if path != drop}
    return {"name": name, "files": files, "args": args or [*FILE_ARGS, "--now", NOW, "--write"], "outputs": []}


def falhas() -> list[dict[str, Any]]:
    return [
        falha("curadoria que é objeto com chaves", base(featured={"projects": {"a": 1}})),
        falha("order ilegível numa entrada fora do inventário",
              base(featured={"projects": [{"name": "fantasma", "order": "x"}]})),
        falha("order que é lista numa entrada do inventário",
              base(featured={"projects": [{"name": "a", "order": [1]}]})),
        falha("curadoria nula", base(featured={"projects": None})),
        falha("arsenal nulo", base(stack={"tools": None})),
        falha("arsenal com texto no lugar de objeto", base(stack={"tools": ["x"]})),
        falha("arsenal ausente", base(stack=None)),
        falha("datas com e sem fuso no mesmo inventário",
              base(repos=[repo("a", "Projeto a."), repo("b", "Projeto b.", pushed_at="2026-09-21T10:00:00")],
                   checks={})),
        falha("README sem um dos marcadores", base(template=readme(missing="LIVE-PROJECTS"))),
        falha("README ausente", base(), drop="README.md"),
        falha("exclusões que não são objeto", base(excluded=[])),
        falha("tudo excluído", base(excluded={"repositories": ["a", "b"]})),
        falha("checagem sem resultado para um site", base(checks={})),
        falha("--now sem fuso", base(), args=[*FILE_ARGS, "--now", "2026-09-25T12:00:00"]),
    ]


SCENARIOS = [vitrine_cheia(), sem_publicos(), poucos_publicos(), *falhas()]


def run(scenario: dict[str, Any]) -> dict[str, Any]:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp) / "raiz"
        for path, text in scenario["files"].items():
            target = root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(text.encode("utf-8"))
        (root / "languages").mkdir(exist_ok=True)
        args = [arg.replace("{root}", str(root)) for arg in scenario["args"]]
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(root), *args],
            capture_output=True, check=False,
            env={key: value for key, value in os.environ.items() if "TOKEN" not in key and key != "GITHUB_API_URL"},
        )
        expected: dict[str, Any] = {"exit": result.returncode, "stdout": result.stdout.decode("utf-8")}
        stderr = result.stderr.decode("utf-8").strip().splitlines()
        if result.returncode == 1:
            expected["crash"] = stderr[-1].split(":", 1)[0]
        elif result.returncode == 2:
            expected["message"] = stderr[-1].removeprefix("profile refresh failed: ").replace(str(root), "{root}")
        outputs = scenario["outputs"] if result.returncode == 0 else []
        # Bytes, não read_text: as quebras universais trocariam o `\r` que o
        # gerador grava de fato (descrição privada com retorno de carro).
        expected["files"] = {path: (root / path).read_bytes().decode("utf-8") for path in outputs}
        expected["untouched"] = result.returncode != 0 and not (root / "assets").exists()
        return expected


def build() -> dict[str, Any]:
    cases = [{"name": s["name"], "files": s["files"], "args": s["args"], "expected": run(s)} for s in SCENARIOS]
    return {
        "schema": "lucas-belucci-bellini/parity-render@1",
        "note": "Gerado por tests/test_parity_render.py a partir do update_profile.py real. Não editar à mão.",
        "python": sys.version.split()[0],
        "cases": cases,
    }


def render(data: dict[str, Any]) -> str:
    return json.dumps(data, ensure_ascii=False, indent=2) + "\n"


class RenderParityTests(unittest.TestCase):
    data: dict[str, Any]

    @classmethod
    def setUpClass(cls) -> None:
        if sys.version_info[:2] != REFERENCE_MINOR:
            raise unittest.SkipTest(f"fixture do CPython {REFERENCE_MINOR[0]}.{REFERENCE_MINOR[1]} (o do CI)")
        cls.data = build()
        if os.environ.get("UPDATE_PARITY") == "1":
            FIXTURE.write_text(render(cls.data), encoding="utf-8")

    def test_fixture_reflete_o_python_atual(self) -> None:
        recorded = json.loads(FIXTURE.read_text(encoding="utf-8"))
        data = dict(self.data, python=recorded["python"])
        self.assertEqual(
            FIXTURE.read_text(encoding="utf-8"), render(data),
            "render.json não bate com o update_profile.py atual. Se o gerador mudou de propósito, "
            "regenere com a versão do CI e porte a mudança para o profile-render no mesmo PR.",
        )

    def test_armadilhas_estao_cobertas(self) -> None:
        cases = {c["name"]: c["expected"] for c in self.data["cases"]}
        full = cases["vitrine cheia: empates, repetidos, caixa, manifestos tortos, fuso"]
        self.assertEqual(0, full["exit"], full)
        text = full["files"]["README.md"]
        self.assertIn("<br>", text, "badges em mais de uma linha")
        self.assertIn("**EXTENSÕES DO PORTFÓLIO**", text)
        self.assertIn("atualizado em `2026-09-24 23:30 UTC`", text, "hora local do fuso, perfil ignorado")
        self.assertIn("segunda cópia, fica como está", text, "só o primeiro par de marcadores é trocado")
        self.assertIn("\\1 e \\g<0>", text)
        self.assertIn("Privado \\| com quebra\re retorno", text, "só o \\n vira espaço na tabela dos privados")
        self.assertIn("| **segredo** | Web | `Rust` `Zig` |", text, "A20 continua registrado")
        self.assertIn("### 🧰 ábaco", text)
        self.assertIn("<sub>ex.: Arena</sub>", text)
        self.assertIn("Til~de", text)
        self.assertIn('"unreachable_sites": ["demo-down", "zeta-down"]', full["stdout"])
        self.assertNotIn("demo-excluded", text)
        self.assertIn(" +", text.split("<!-- ECOSYSTEM-MAP:START -->")[1].split("<!-- ECOSYSTEM-MAP:END -->")[0])

        empty = cases["sem públicos: estados vazios, README com CRLF, datas sem fuso, sem checagem"]
        self.assertEqual(0, empty["exit"])
        self.assertNotIn("\r", empty["stdout"], "o README é lido com quebras universais")
        self.assertIn("\"site_check_skipped\": true", empty["stdout"])

        few = cases["poucos públicos: nenhum site no ar, carimbo do relógio com fuso, --out-dir"]
        self.assertIn("atualizado em `2026-09-25 12:00 UTC`", few["files"]["out/README.md"])
        self.assertIn("`1000.00%`", few["files"]["out/README.md"])
        self.assertIn('<td width="33%"></td>', few["files"]["out/README.md"], "linha incompleta de domínios")
        self.assertIn('<td width="50%"></td>', few["files"]["out/README.md"], "card ímpar")
        self.assertIn("Atividade, Atividade…", few["files"]["out/README.md"], "vírgula sai antes das reticências")
        self.assertIn("| — | Nenhum site verificado nesta auditoria | — | — |", few["files"]["out/README.md"])

        self.assertEqual("AttributeError", cases["curadoria que é objeto com chaves"].get("crash"))
        self.assertEqual("ValueError", cases["order ilegível numa entrada fora do inventário"].get("crash"))
        self.assertEqual("TypeError", cases["datas com e sem fuso no mesmo inventário"].get("crash"))
        self.assertEqual("ValueError", cases["README sem um dos marcadores"].get("crash"))
        self.assertEqual("FileNotFoundError", cases["README ausente"].get("crash"))
        self.assertEqual("GitHub returned no repositories", cases["tudo excluído"].get("message"))
        for name, expected in cases.items():
            if expected["exit"] != 0:
                self.assertTrue(expected["untouched"], f"{name}: o Python não grava nada antes de quebrar")


if __name__ == "__main__":
    unittest.main()
