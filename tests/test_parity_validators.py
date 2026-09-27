"""Casos de paridade dos validadores: o Python é a referência, o Rust confere.

    OLD PYTHON ──▶ tests/fixtures/parity/validators.json ◀── NEW RUST

Cada cenário é uma árvore sintética do perfil (README com os 13 marcadores,
manifestos, catálogo e assets) com uma mudança por cima; sobre ela os scripts
reais rodam como no CI — cada um copiado para `scripts/` da árvore, porque é
de lá que eles acham a raiz. Fica registrado o que cada um fez: saída padrão,
stderr e código de saída, ou a exceção do traceback. A raiz vira `<root>`.

    cargo test -p profile-core --test parity_validate

roda `profile_core::validate` sobre as mesmas árvores. Os validadores:

    validate_dynamic_sections.py  (README.before.md → README.md)
    validate_exclusions.py
    validate_project_links.py
    validate_restored_style.py
    validate_language_badges.py   (sem a rede: check_urls vira no-op)
    validate_profile.py           (sem a rede; só a lista de erros, sem os
                                   do YAML do workflow — ver D-037)

**Versão:** as mensagens vêm da biblioteca padrão (JSON, OSError, repr);
o teste só compara no CPython do CI.

    UPDATE_PARITY=1 python3.12 -m unittest tests.test_parity_validators
"""
from __future__ import annotations

import copy
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any, Callable

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "parity" / "validators.json"
REFERENCE_MINOR = (3, 12)
SCRIPTS = ("validate_dynamic_sections.py", "validate_exclusions.py", "validate_project_links.py",
           "validate_restored_style.py", "validate_language_badges.py", "validate_profile.py")
CLEAN_ENV = {key: value for key, value in os.environ.items()
             if "TOKEN" not in key and not key.startswith("GITHUB_") and key != "DATABASE_URL"}

BADGES_OFFLINE = """
import sys
sys.path.insert(0, sys.argv[1])
import validate_language_badges as v
v.check_urls = lambda urls: None
sys.exit(v.main())
"""

PROFILE_OFFLINE = """
import json, sys
from urllib.error import URLError
sys.path.insert(0, sys.argv[1])
import validate_profile as v
def offline(*_args, **_kwargs):
    raise URLError("offline")
v.urlopen = offline
sys.exit(v.main())
"""

BADGE = "https://img.shields.io/badge/{}-x?style=flat-square&labelColor=0e0c16"
SYNTHETIC_WORKFLOW = """on:
  workflow_dispatch:
  schedule:
    - cron: "0 0 * * *"
permissions:
  contents: write
jobs:
  refresh:
    steps:
      - run: python3 scripts/update_profile.py --write
      - run: python3 scripts/validate_dynamic_sections.py && python3 scripts/validate_exclusions.py
      - run: git diff --quiet || git commit -am refresh
"""


def block(marker: str, body: str) -> str:
    return f"<!-- {marker}:START -->\n{body}\n<!-- {marker}:END -->"


def base_readme(**bodies: str) -> str:
    parts = {
        "PROFILE-DASHBOARD": "| **Painel** | 3 repos |",
        "WHAT-I-BUILD": "Construo ferramentas.",
        "PRODUCT-CARDS": "| **Alpha** | [▸ Abrir site](https://alpha.example.org) · [código](https://github.com/o/alpha) |",
        "FEATURED-PROJECTS": (
            f"[![ABRIR SITE]({BADGE.format('ABRIR%20SITE')})](https://alpha.example.org) "
            f"[![CÓDIGO]({BADGE.format('C%C3%93DIGO')})](https://github.com/o/alpha)"
        ),
        "ARSENAL-STACK": "### 🌐 Frameworks & Web\n### ⚙️ Infraestrutura & DevOps\n### 🧠 IA & Conhecimento\n"
                         "### 🔧 Hardware & Simulação",
        "LANGUAGE-BADGES": " ".join(f"[![{label}]({BADGE.format(code)})](#)"
                                    for label, code in [("C#", "C%23"), ("PL/pgSQL", "PL%2FpgSQL"), ("Rust", "Rust")]),
        "LANGUAGE-STATS": "| # | Linguagem | Bytes | % | Repos |\n|---|---|---|---|---|\n"
                          "| 1 | **C#** | `1 KB` | `50%` | 1 |\n| 2 | **PL/pgSQL** | `1 KB` | `25%` | 1 |\n"
                          "| 3 | **Rust** | `1 KB` | `25%` | 1 |",
        "PUBLIC-PROJECTS": "| **alpha** | Web |\n| **beta** | Ferramentas |",
        "PRIVATE-PROJECTS": "| **gama** | privado |",
        "WEBSITE-DIRECTORY": "- Alpha — [▸ Abrir site](https://alpha.example.org) · [código](https://github.com/o/alpha)",
        "LIVE-PROJECTS": "| Alpha | [▸ Abrir site](https://alpha.example.org) | [código](https://github.com/o/alpha) |",
        "ECOSYSTEM-MAP": "```\nalpha → beta\n```",
        "PROJECT-MAP": "| alpha | Rust |",
    }
    parts.update(bodies)
    header = "\n".join([
        '<div align="center">',
        '<img src="https://capsule-render.vercel.app/api?type=waving" />',
        '<img src="https://readme-typing-svg.demolab.com?lines=Ol%C3%A1" />',
        '<img src="https://skillicons.dev/icons?i=rust" />',
        "![Projetos](./assets/profile-projects.svg)",
        "![Console](./assets/jarvis-console.svg)",
        '<img src="assets/profile-stats.svg" /> <img src="assets/profile-top-langs.svg" />',
        '<img src="assets/profile-streak.svg" /> <img src="assets/profile-trophies.svg" />',
        '<img src="https://raw.githubusercontent.com/o/o/output/github-contribution-grid-snake-dark.svg" />',
        '<img src="https://komarev.com/ghpvc/?username=o" />',
        "</div>",
    ])
    order = ["PROFILE-DASHBOARD", "WHAT-I-BUILD", "PRODUCT-CARDS", "FEATURED-PROJECTS", "ARSENAL-STACK",
             "LANGUAGE-BADGES", "LANGUAGE-STATS", "PUBLIC-PROJECTS", "PRIVATE-PROJECTS", "WEBSITE-DIRECTORY",
             "LIVE-PROJECTS", "ECOSYSTEM-MAP", "PROJECT-MAP"]
    return header + "\n\n" + "\n\n".join(block(m, parts[m]) for m in order) + "\n\nRodapé estático.\n"


def project(name: str, **extra: Any) -> dict[str, Any]:
    data: dict[str, Any] = {
        "name": name, "repository": f"o/{name}", "slug": name, "category": "Web", "private": False,
        "website": None, "website_declared": None, "website_status": "none", "primary_cta": "github",
        "secondary_cta": None, "github_visible": True,
    }
    data.update(extra)
    return data


ALPHA = project("alpha", website="https://alpha.example.org", website_declared="https://alpha.example.org",
                website_status="verified", primary_cta="website", secondary_cta="github")


def catalog(*projects: dict[str, Any], **extra: Any) -> str:
    data: dict[str, Any] = {"schema": "catalog@1", "categories": ["Web", "Ferramentas"],
                            "projects": list(projects) or [ALPHA, project("beta", category="Ferramentas")]}
    data.update(extra)
    return json.dumps(data, ensure_ascii=False, indent=2) + "\n"


def base_tree() -> dict[str, Any]:
    readme = base_readme()
    files: dict[str, Any] = {
        "README.md": readme,
        "README.before.md": readme,
        "docs/README_SITES.json": "{}\n",
        "docs/README_FEATURED.json": '{"projects": []}\n',
        "docs/README_STACK.json": "{}\n",
        "docs/README_EXCLUDED.json": '{"repositories": ["segredo", "o/oculto"]}\n',
        "docs/project-catalog.json": catalog(),
        # O validate_profile.py lê este arquivo fora do try; o conteúdo é
        # sintético para o fixture não depender do workflow de verdade.
        ".github/workflows/update-profile.yml": SYNTHETIC_WORKFLOW,
    }
    for asset in ("jarvis-console.svg", "lang-stats.svg", "profile-projects.svg", "profile-stats.svg",
                  "profile-streak.svg", "profile-top-langs.svg", "profile-trophies.svg", "profile-snapshot.svg"):
        files[f"assets/{asset}"] = "<svg/>\n"
    return {"files": files, "delete": [], "mkdir": []}


Patch = Callable[[dict[str, Any]], None]


def readme(text: str, *, before: str | None = None) -> Patch:
    def apply(tree: dict[str, Any]) -> None:
        tree["files"]["README.md"] = text
        tree["files"]["README.before.md"] = text if before is None else before
    return apply


def delete(*paths: str) -> Patch:
    def apply(tree: dict[str, Any]) -> None:
        for path in paths:
            tree["files"].pop(path, None)
            tree["delete"].append(path)
    return apply


def mkdir(*paths: str) -> Patch:
    def apply(tree: dict[str, Any]) -> None:
        for path in paths:
            tree["files"].pop(path, None)
            tree["mkdir"].append(path)
    return apply


def raw(path: str, data: bytes) -> Patch:
    def apply(tree: dict[str, Any]) -> None:
        tree["files"][path] = {"hex": data.hex()}
    return apply


def files(mapping: dict[str, Any]) -> Patch:
    def apply(tree: dict[str, Any]) -> None:
        tree["files"].update(mapping)
    return apply


BASE = base_readme()
CRLF = BASE.replace("\n", "\r\n")
LONG = "Projeto com nome comprido e acentuação — " * 3

SCENARIOS: list[tuple[str, list[Patch]]] = [
    ("árvore boa: todos passam", []),
    ("CRLF no README e só um bloco mudou", [readme(CRLF.replace("Construo ferramentas.", "Construo bancos."),
                                                  before=BASE)]),
    ("texto fora dos marcadores mudou", [readme(BASE.replace("Rodapé estático.", "Rodapé novo."), before=BASE)]),
    ("marcador ausente depois da geração", [readme(BASE.replace("<!-- PROJECT-MAP:END -->", ""), before=BASE)]),
    ("marcador ausente antes da geração", [readme(BASE, before=BASE.replace("<!-- WHAT-I-BUILD:START -->", ""))]),
    ("marcador repetido e END antes do START", [readme(
        BASE.replace("<!-- LIVE-PROJECTS:START -->", "<!-- ECOSYSTEM-MAP:END -->\n<!-- LIVE-PROJECTS:START -->")
        + block("PROFILE-DASHBOARD", "de novo") + "\n")]),
    ("README ausente", [delete("README.md")]),
    ("README não é UTF-8", [raw("README.md", BASE.encode()[:40] + b"\xe2\x82x" + BASE.encode()[40:])]),
    ("README é um diretório", [mkdir("README.md")]),
    ("README antes da geração ausente", [delete("README.before.md")]),
    ("excluídos presentes, texto vazio e dicionário", [
        files({"docs/README_EXCLUDED.json": '{"repositories": ["segredo", "", "alpha", "segredo"]}'}),
        readme(BASE.replace("Rodapé estático.", "Rodapé segredo."))]),
    ("excluídos como texto (itera caracteres)", [files({"docs/README_EXCLUDED.json": '{"repositories": "zq"}'})]),
    ("excluídos como objeto (itera chaves)", [
        files({"docs/README_EXCLUDED.json": '{"repositories": {"alpha": 1, "zzz": 2}}'})]),
    ("excluídos com número", [files({"docs/README_EXCLUDED.json": '{"repositories": 7}'})]),
    ("excluídos com valores não textuais", [
        files({"docs/README_EXCLUDED.json": '{"repositories": [1, true, null, 2.5, 1e16, ["x"], {"k": "v"}]}'}),
        readme(BASE.replace("Rodapé estático.", "Rodapé 1 True None 2.5 1e+16 ['x'] {'k': 'v'}"))]),
    ("manifesto de exclusões é lista", [files({"docs/README_EXCLUDED.json": '["segredo"]'})]),
    ("manifesto de exclusões sem a chave", [files({"docs/README_EXCLUDED.json": "{}"})]),
    ("manifesto de exclusões com vírgula final", [files({"docs/README_EXCLUDED.json": '{"repositories": ["a",]}'})]),
    ("manifesto de exclusões ausente", [delete("docs/README_EXCLUDED.json")]),
    ("catálogo ausente", [delete("docs/project-catalog.json")]),
    ("catálogo com BOM", [files({"docs/project-catalog.json": "\ufeff" + catalog()})]),
    ("catálogo sem projetos", [files({"docs/project-catalog.json": catalog(ALPHA, projects=[])})]),
    ("catálogo sem a chave projects", [files({"docs/project-catalog.json": '{"categories": []}'})]),
    ("catálogo é lista", [files({"docs/project-catalog.json": "[1, 2]"})]),
    ("projects é objeto", [files({"docs/project-catalog.json": '{"projects": {"alpha": {}}}'})]),
    ("projects é número", [files({"docs/project-catalog.json": '{"projects": 3}'})]),
    ("projects é zero", [files({"docs/project-catalog.json": '{"projects": 0}'})]),
    ("projeto que não é objeto", [files({"docs/project-catalog.json": json.dumps({"projects": [ALPHA, "beta"]})})]),
    ("CTAs errados em todas as combinações", [files({"docs/project-catalog.json": catalog(
        ALPHA,
        project("site-github", website="https://s.example.org", website_status="verified"),
        project("sem-site-website", primary_cta="website", secondary_cta="github"),
        project("privado", private=True, website="https://p.example.org", website_declared="https://p.example.org",
                website_status="verified", primary_cta="website", secondary_cta="github"),
        project("http", website="http://h.example.org", website_status="verified", primary_cta="website",
                secondary_cta="github"),
        project("fora", website="https://f.example.org", website_declared="https://f.example.org",
                website_status="unreachable", primary_cta="website", secondary_cta="github"),
        project("privado-declarado", private=1, website_declared="https://d.example.org"),
        project("tipos", website=1, primary_cta=1.5e16, secondary_cta=True, website_status=["x"]),
        project("reprs", primary_cta={"it's": 'a"b'}, secondary_cta="\x85\u200b\t😀", website_status=None),
        {"repository": "o/sem-nome"},
        project(None, primary_cta="github", secondary_cta=False),
        project("vazio", website="", primary_cta="", secondary_cta=""),
    )})]),
    ("código antes do site no README", [readme(base_readme(**{
        "PRODUCT-CARDS": f"| **{LONG}** | [código](https://github.com/o/a) · [▸ Abrir site](https://a.example.org) |",
        "FEATURED-PROJECTS": (f"  [![C]({BADGE.format('C%C3%93DIGO')})](https://github.com/o/a)\u2028"
                              f"[![S]({BADGE.format('ABRIR%20SITE')})](https://a.example.org)\x0b"
                              f"\x1f [código](https://x) [![S]({BADGE.format('ABRIR%20SITE')})](https://a.example.org) \x1c"),
        "WEBSITE-DIRECTORY": "sem link: [código] Abrir site\n[código](http://c) sem site",
        "LIVE-PROJECTS": "",
    }))]),
    ("identidade visual quebrada", [readme(base_readme().replace("skillicons.dev", "icons.example")
                                           .replace("komarev.com/ghpvc", "contador")
                                           .replace("<!-- LANGUAGE-STATS:END -->", "")),
                                    delete("assets/profile-streak.svg"), mkdir("assets/profile-stats.svg"),
                                    files({"assets/lang-stats.svg": ""})]),
    ("README_SITES.json inválido", [files({"docs/README_SITES.json": '{"a": "b"\n "c": 1}'})]),
    ("README_SITES.json não é UTF-8", [raw("docs/README_SITES.json", b'{"\xff": 1}')]),
    ("README_SITES.json ausente", [delete("docs/README_SITES.json")]),
    ("README_SITES.json é lista", [files({"docs/README_SITES.json": '["x"]'})]),
    ("badges: marcadores ausentes", [readme(BASE.replace("<!-- LANGUAGE-BADGES:START -->", ""))]),
    ("badges: nenhum", [readme(base_readme(**{"LANGUAGE-BADGES": "sem badges"}))]),
    ("badges: rótulo repetido", [readme(base_readme(**{"LANGUAGE-BADGES": " ".join(
        f"[![{label}]({BADGE.format(label)})](#)" for label in ["C#", "PL/pgSQL", "C#"])}))]),
    ("badges: rótulo codificado", [readme(base_readme(**{"LANGUAGE-BADGES": " ".join(
        f"[![{label}]({BADGE.format(label)})](#)" for label in ["C#", "PL/pgSQL", "C%23"])}))]),
    ("badges: sem PL/pgSQL", [readme(base_readme(**{"LANGUAGE-BADGES": f"[![C#]({BADGE.format('C')})](#)"}))]),
    ("badges: estilo incompleto", [readme(base_readme(**{"LANGUAGE-BADGES": (
        f"[![C#]({BADGE.format('C')})](#) [![PL/pgSQL](https://img.shields.io/badge/P-x?style=flat-square)](#)")}))]),
    ("badges: quebra de linha dentro da query some no urlparse", [readme(base_readme(**{
        "LANGUAGE-BADGES": (f"[![C#]({BADGE.format('C')})](#) [![PL/pgSQL](https://img.shields.io/badge/P-x"
                            "?style=flat-\nsquare&labelColor=0e0c16#frag)](#) [![Rust]" + f"({BADGE.format('R')})](#)")}))]),
    ("badges: fragmento esconde o estilo", [readme(base_readme(**{"LANGUAGE-BADGES": (
        f"[![C#]({BADGE.format('C')})](#) [![PL/pgSQL](https://img.shields.io/badge/P#x?style=flat-square"
        "&labelColor=0e0c16)](#)")}))]),
    ("badges: categorias ausentes e repetidas", [readme(base_readme(**{
        "ARSENAL-STACK": "### A IA & Conhecimento\n### B IA & Conhecimento\n### Frameworks & Web\n"
                         "### X Hardware & Simulação \n#### Y Frameworks & Web"}))]),
    ("badges: categorias repetidas contam", [readme(base_readme(**{
        "ARSENAL-STACK": "### 🌐 Frameworks & Web\n### ⚙️ Infraestrutura & DevOps\n### 🧠 IA & Conhecimento\n"
                         "### 🔧 Hardware & Simulação\n### 🔁 Hardware & Simulação"}))]),
    ("badges: matriz com linhas a menos", [readme(base_readme(**{
        "LANGUAGE-STATS": "| 1 | **C#** | x |\n| ٢ | **Rust** | y |\n|3| **Z** | w |"}))]),
    ("catálogo: duplicados, excluídos, taxonomia, sites e segredos", [
        files({"docs/project-catalog.json": catalog(
            ALPHA,
            project("alpha", slug="beta"),
            project("beta"),
            project("segredo", category="Jogos"),
            project("o/oculto", repository="o/oculto", slug="oculto", category=5),
            project("gama", website="http://g.example.org", website_status="unreachable", private=True),
            project("", slug="vazio", website="https://r.example.org", website_status="verified"),
            project("repete", slug="repete", website="https://r.example.org", website_status="verified"),
            project("repete2", slug="repete2", website="https://r.example.org", website_status="verified",
                    github_visible=0),
            {"name": "sem-flag", "repository": "o/sem-flag", "slug": "sem-flag", "category": "Web",
             "website_declared": "https://declarado.example.org"},
        )}),
        readme(BASE.replace("Rodapé estático.", "Rodapé [▸ Abrir site](https://desconhecido.example.org) "
                            "[▸ Abrir site](https://declarado.example.org) GHP_x Postgresql://y "
                            "![falta](./assets/nao-existe.svg) segredo"))]),
    ("catálogo: categorias não iteráveis", [files({"docs/project-catalog.json": catalog(categories=None)})]),
    ("catálogo: categorias com lista", [files({"docs/project-catalog.json": catalog(categories=["Web", ["x"]])})]),
    ("catálogo: categorias como texto", [files({"docs/project-catalog.json": catalog(categories="Web")})]),
    ("catálogo: taxonomia vazia não reprova categoria", [
        files({"docs/project-catalog.json": catalog(categories=[])})]),
    ("bloco repetido: só o primeiro é gerado", [readme(
        base_readme(**{"PROFILE-DASHBOARD": "B"}) + block("PROFILE-DASHBOARD", "cópia estática") + "\n",
        before=base_readme(**{"PROFILE-DASHBOARD": "A"}) + block("PROFILE-DASHBOARD", "cópia estática") + "\n")]),
    ("catálogo: manifesto inválido e marcador invertido", [
        files({"docs/README_STACK.json": "[1,]", "docs/README_FEATURED.json": '"texto"'}),
        readme(BASE.replace("<!-- PROJECT-MAP:START -->", "<!-- TMP -->")
               .replace("<!-- PROJECT-MAP:END -->", "<!-- PROJECT-MAP:START -->")
               .replace("<!-- TMP -->", "<!-- PROJECT-MAP:END -->"))]),
]


def materialize(tree: dict[str, Any], target: Path) -> None:
    target.mkdir(parents=True)
    for path, content in tree["files"].items():
        destination = target / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(content, dict):
            destination.write_bytes(bytes.fromhex(content["hex"]))
        else:
            destination.write_bytes(content.encode("utf-8"))
    for path in tree["mkdir"]:
        (target / path).mkdir(parents=True, exist_ok=True)


def outcome(result: subprocess.CompletedProcess[str], root: Path) -> dict[str, Any]:
    clean = lambda text: text.replace(str(root), "<root>")  # noqa: E731
    stderr = clean(result.stderr)
    if "Traceback (most recent call last)" in stderr:
        return {"code": result.returncode, "stdout": clean(result.stdout), "crash": stderr.strip().splitlines()[-1]}
    return {"code": result.returncode, "stdout": clean(result.stdout), "stderr": stderr}


def profile_errors(result: subprocess.CompletedProcess[str], root: Path) -> dict[str, Any]:
    recorded = outcome(result, root)
    if "crash" in recorded:
        return {"crash": recorded["crash"]}
    errors = json.loads(recorded["stdout"])["errors"]
    # Ordenada: o script percorre os excluídos de um set, e a ordem muda a cada processo.
    return {"errors": sorted(e for e in errors
                             if not e.startswith(("workflow missing required construct", "workflow YAML")))}


def run_scenario(tree: dict[str, Any], work: Path) -> dict[str, Any]:
    root = (work / "root").resolve()
    materialize(tree, root)
    scripts = root / "scripts"
    scripts.mkdir(exist_ok=True)
    for script in SCRIPTS:
        shutil.copy2(ROOT / "scripts" / script, scripts / script)

    def run(*command: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run([sys.executable, *command], capture_output=True, text=True, env=CLEAN_ENV,
                              cwd=root, check=False, timeout=60)

    return {
        "readme": outcome(run(str(scripts / "validate_dynamic_sections.py"), "--before",
                              str(root / "README.before.md"), "--after", str(root / "README.md")), root),
        "exclusions": outcome(run(str(scripts / "validate_exclusions.py")), root),
        "links": outcome(run(str(scripts / "validate_project_links.py")), root),
        "visual": outcome(run(str(scripts / "validate_restored_style.py")), root),
        "badges": outcome(run("-c", BADGES_OFFLINE, str(scripts)), root),
        "catalog": profile_errors(run("-c", PROFILE_OFFLINE, str(scripts)), root),
    }


def build() -> dict[str, Any]:
    base = base_tree()
    scenarios = []
    for name, patches in SCENARIOS:
        tree = copy.deepcopy(base)
        for patch in patches:
            patch(tree)
        changes = {
            "write": {p: c for p, c in tree["files"].items() if base["files"].get(p) != c},
            "delete": tree["delete"],
            "mkdir": tree["mkdir"],
        }
        with tempfile.TemporaryDirectory() as tmp:
            expected = run_scenario(tree, Path(tmp))
        scenarios.append({"name": name, "changes": changes, "expected": expected})
    return {
        "schema": "lucas-belucci-bellini/parity-validators@1",
        "note": "Gerado por tests/test_parity_validators.py a partir dos scripts/validate_*.py reais. "
                "Não editar à mão.",
        "python": sys.version.split()[0],
        "base": base["files"],
        "scenarios": scenarios,
    }


def render(data: dict[str, Any]) -> str:
    return json.dumps(data, ensure_ascii=False, indent=1) + "\n"


class ValidatorsParityTests(unittest.TestCase):
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
        self.assertEqual(FIXTURE.read_text(encoding="utf-8"), render(dict(self.data, python=recorded["python"])),
                         "validators.json não bate com os scripts/validate_*.py atuais. Se um validador mudou de "
                         "propósito, regenere com a versão do CI e porte a mudança para o profile-core no mesmo PR.")

    def test_armadilhas_estao_cobertas(self) -> None:
        by_name = {s["name"]: s["expected"] for s in self.data["scenarios"]}
        good = by_name["árvore boa: todos passam"]
        self.assertEqual([0] * 5, [good[v]["code"] for v in ("readme", "exclusions", "links", "visual", "badges")])
        self.assertEqual([], good["catalog"]["errors"])
        self.assertEqual(0, by_name["CRLF no README e só um bloco mudou"]["readme"]["code"])
        self.assertIn("ValueError", by_name["marcador ausente antes da geração"]["readme"]["crash"])
        self.assertIn("UnicodeDecodeError", by_name["README não é UTF-8"]["links"]["crash"])
        self.assertEqual(1, by_name["README ausente"]["badges"]["code"], "OSError tratado pelo main()")
        self.assertIn("JSONDecodeError", by_name["manifesto de exclusões com vírgula final"]["exclusions"]["crash"])
        self.assertIn("1.5e+16", by_name["CTAs errados em todas as combinações"]["links"]["stderr"])
        self.assertIn("\\x85\\u200b\\t😀", by_name["CTAs errados em todas as combinações"]["links"]["stderr"])
        self.assertIn("Expecting ',' delimiter", by_name["README_SITES.json inválido"]["visual"]["stdout"])
        self.assertEqual(0, by_name["badges: quebra de linha dentro da query some no urlparse"]["badges"]["code"])
        self.assertTrue(any("URL repetida" in e for e in
                            by_name["catálogo: duplicados, excluídos, taxonomia, sites e segredos"]["catalog"]["errors"]))


if __name__ == "__main__":
    unittest.main()
