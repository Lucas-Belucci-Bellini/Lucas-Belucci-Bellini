"""Teste golden do gerador do README (Fase 0.11).

Entrada fixa e sintética (tests/fixtures/profile/input) → saída byte a byte
(tests/fixtures/profile/expected). É o contrato que o núcleo em Rust vai ter
de reproduzir antes de substituir o Python (docs/migration/PYTHON-TO-RUST.md
§6): mesmas entradas, mesmos bytes.

Roda offline: inventário e linguagens vêm de arquivo, as checagens de site vêm
de `--site-checks-fixture` e o relógio é fixo (`--now`).

A saída esperada registra o comportamento ATUAL, inclusive os defeitos já
conhecidos e ainda não corrigidos de propósito (D-006):
  * "demo-daily-notes" cai em "IA & Automação" porque "daily" contém "ai"
    (classificação por substring, auditoria A6);
  * PROJECT-MAP mostra as linguagens de "demo-private" (auditoria A20).
Quando uma correção dessas entrar, o diff deste teste é a revisão dela.

Mudou o gerador de propósito? Regenere e revise o diff:

    UPDATE_GOLDEN=1 python3 -m unittest tests.test_golden_profile
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "update_profile.py"
FIXTURE = ROOT / "tests" / "fixtures" / "profile"
INPUT = FIXTURE / "input"
EXPECTED = FIXTURE / "expected"
NOW = "2026-09-25T12:00:00Z"

# arquivo gerado (relativo à raiz) → nome no diretório expected/
OUTPUTS = {
    "README.md": "README.md",
    "docs/project-catalog.json": "project-catalog.json",
    "assets/profile-snapshot.svg": "profile-snapshot.svg",
}


def run_generator(root: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable, str(SCRIPT),
            "--root", str(root),
            "--input-repos", str(root / "repos.json"),
            "--languages-dir", str(root / "languages"),
            "--site-checks-fixture", str(root / "site-checks.json"),
            "--now", NOW,
            "--write",
        ],
        capture_output=True,
        text=True,
        check=False,
        # Sem token nenhum: prova que o modo fixture não depende de rede.
        env={key: value for key, value in os.environ.items() if "TOKEN" not in key},
    )


class GoldenProfileTests(unittest.TestCase):
    tmp: tempfile.TemporaryDirectory[str]
    root: Path
    first: subprocess.CompletedProcess[str]

    @classmethod
    def setUpClass(cls) -> None:
        cls.tmp = tempfile.TemporaryDirectory()
        cls.root = Path(cls.tmp.name) / "profile"
        shutil.copytree(INPUT, cls.root)
        cls.first = run_generator(cls.root)
        if os.environ.get("UPDATE_GOLDEN") == "1" and cls.first.returncode == 0:
            EXPECTED.mkdir(parents=True, exist_ok=True)
            for source, name in OUTPUTS.items():
                shutil.copyfile(cls.root / source, EXPECTED / name)
            (EXPECTED / "summary.json").write_text(cls.first.stdout, encoding="utf-8")

    @classmethod
    def tearDownClass(cls) -> None:
        cls.tmp.cleanup()

    def test_gerador_roda_offline_sem_erro(self) -> None:
        self.assertEqual(0, self.first.returncode, self.first.stderr)

    def test_saidas_iguais_ao_golden(self) -> None:
        for source, name in OUTPUTS.items():
            with self.subTest(arquivo=source):
                expected = (EXPECTED / name).read_text(encoding="utf-8")
                actual = (self.root / source).read_text(encoding="utf-8")
                self.assertEqual(expected, actual, f"{source} difere do golden; se foi de propósito, rode UPDATE_GOLDEN=1")

    def test_resumo_igual_ao_golden(self) -> None:
        self.assertEqual(
            json.loads((EXPECTED / "summary.json").read_text(encoding="utf-8")),
            json.loads(self.first.stdout),
        )

    def test_segunda_execucao_nao_muda_nada(self) -> None:
        # Dado igual → arquivo igual → nenhum commit.
        before = {source: (self.root / source).read_bytes() for source in OUTPUTS}
        second = run_generator(self.root)
        self.assertEqual(0, second.returncode, second.stderr)
        self.assertFalse(json.loads(second.stdout)["catalog_written"])
        for source, content in before.items():
            with self.subTest(arquivo=source):
                self.assertEqual(content, (self.root / source).read_bytes())

    def test_so_os_blocos_gerados_mudaram(self) -> None:
        sys.path.insert(0, str(ROOT / "scripts"))
        try:
            import validate_dynamic_sections as vds  # noqa: PLC0415
        finally:
            sys.path.pop(0)
        before = vds.canonicalize((INPUT / "README.md").read_text(encoding="utf-8"))
        after = vds.canonicalize((self.root / "README.md").read_text(encoding="utf-8"))
        self.assertEqual(before, after)

    def test_regras_de_privacidade_e_exclusao(self) -> None:
        readme = (self.root / "README.md").read_text(encoding="utf-8")
        catalog = json.loads((self.root / "docs" / "project-catalog.json").read_text(encoding="utf-8"))
        self.assertNotIn("demo-excluded", readme)
        self.assertNotIn("excluded.example.org", readme)
        self.assertNotIn("private.example.org", readme)
        self.assertNotIn("demo-excluded", {project["name"] for project in catalog["projects"]})
        privado = next(project for project in catalog["projects"] if project["name"] == "demo-private")
        self.assertNotIn("website", privado)
        self.assertNotIn("website_declared", privado)

    def test_carimbo_nao_vem_do_proprio_perfil(self) -> None:
        # O perfil tem o push mais recente (11:17). O carimbo é o do push mais
        # recente entre os demais — demo-private, 2026-09-22 10:00: privados
        # sempre entraram nesse cálculo (comportamento anterior, mantido).
        readme = (self.root / "README.md").read_text(encoding="utf-8")
        self.assertIn("atualizado em `2026-09-22 10:00 UTC`", readme)
        self.assertNotIn("2026-09-25 11:17", readme)
        svg = (self.root / "assets" / "profile-snapshot.svg").read_text(encoding="utf-8")
        self.assertIn("generated 2026-09-22 10:00 UTC", svg)

    def test_barra_invertida_da_descricao_chega_literal(self) -> None:
        catalog = json.loads((self.root / "docs" / "project-catalog.json").read_text(encoding="utf-8"))
        projeto = next(project for project in catalog["projects"] if project["name"] == "demo-backslash")
        self.assertIn(r"C:\Users\demo", projeto["description"])
        self.assertIn(r"\g<0>", projeto["description"])

    def test_url_sem_resultado_no_fixture_e_erro(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "profile"
            shutil.copytree(INPUT, root)
            checks = json.loads((root / "site-checks.json").read_text(encoding="utf-8"))
            checks.pop("https://down.example.org")
            (root / "site-checks.json").write_text(json.dumps(checks), encoding="utf-8")
            result = run_generator(root)
        self.assertEqual(2, result.returncode)
        self.assertIn("https://down.example.org", result.stderr)


if __name__ == "__main__":
    unittest.main()
