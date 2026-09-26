"""Testes do bot de linguagens (.github/scripts/lang_stats.py).

Cobrem as regras que a Fase 0 fixou: exclusões editoriais valem aqui também,
o bot não toca o README, e o carimbo de hora não gera commit sozinho.
"""
from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / ".github" / "scripts" / "lang_stats.py"
SPEC = importlib.util.spec_from_file_location("lang_stats", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class ExclusionTests(unittest.TestCase):
    REPOS = [
        {"name": "publico", "full_name": "dono/publico"},
        {"name": "excluido-por-nome", "full_name": "dono/excluido-por-nome"},
        {"name": "excluido-por-full-name", "full_name": "outro/excluido-por-full-name"},
    ]

    def test_remove_por_nome_curto_ou_full_name(self) -> None:
        excluded = {"excluido-por-nome", "outro/excluido-por-full-name"}
        restantes = MODULE.without_excluded(self.REPOS, excluded)
        self.assertEqual(["publico"], [repo["name"] for repo in restantes])

    def test_sem_exclusoes_nada_muda(self) -> None:
        self.assertEqual(self.REPOS, MODULE.without_excluded(self.REPOS, set()))

    def test_le_o_manifesto_versionado(self) -> None:
        manifesto = json.loads((ROOT / "docs" / "README_EXCLUDED.json").read_text(encoding="utf-8"))
        self.assertEqual(set(manifesto["repositories"]), MODULE.load_excluded_names())

    def test_manifesto_ausente_nao_exclui_nada(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            self.assertEqual(set(), MODULE.load_excluded_names(Path(tmp) / "nao-existe.json"))


class OwnershipTests(unittest.TestCase):
    def test_bot_nao_escreve_no_readme(self) -> None:
        # O bloco LANG-STATS não existe no README desde agosto (auditoria, A9);
        # o bot escreve só os dois SVGs dele.
        self.assertFalse(hasattr(MODULE, "patch_readme"))
        self.assertFalse(hasattr(MODULE, "build_markdown"))
        self.assertNotIn("LANG-STATS:START", (ROOT / "README.md").read_text(encoding="utf-8"))
        self.assertEqual({"lang-stats.svg", "profile-top-langs.svg"},
                         {MODULE.SVG_OUT.name, MODULE.PROFILE_TOP_LANGS_OUT.name})

    def test_update_profile_nao_escreve_lang_stats_svg(self) -> None:
        # Um escritor por arquivo (auditoria, A4).
        fonte = (ROOT / "scripts" / "update_profile.py").read_text(encoding="utf-8")
        self.assertNotIn("def render_svg(", fonte)
        self.assertNotIn('/ "lang-stats.svg"', fonte)


class TimestampTests(unittest.TestCase):
    def test_so_o_carimbo_mudou_nao_e_mudanca(self) -> None:
        antigo = '<text>atualizado automaticamente · 2026-09-20 08:35 UTC</text><text>19</text>'
        novo = '<text>atualizado automaticamente · 2026-09-27 08:35 UTC</text><text>19</text>'
        self.assertTrue(MODULE.mesmo_conteudo(novo, antigo))

    def test_numero_mudou_e_mudanca(self) -> None:
        antigo = '<text>2026-09-20 08:35 UTC</text><text>19</text>'
        novo = '<text>2026-09-27 08:35 UTC</text><text>20</text>'
        self.assertFalse(MODULE.mesmo_conteudo(novo, antigo))


if __name__ == "__main__":
    unittest.main()
