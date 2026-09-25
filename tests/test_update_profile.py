import importlib.util
import unittest
from datetime import timedelta
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "update_profile.py"
SPEC = importlib.util.spec_from_file_location("update_profile", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class LanguageBadgeTests(unittest.TestCase):
    def test_special_language_names_keep_readable_markdown_labels(self) -> None:
        rows = [
            {"language": "C#", "display": "C#", "repositories": 2, "bytes": 200, "share": 60.0},
            {"language": "PLpgSQL", "display": "PL/pgSQL", "repositories": 2, "bytes": 100, "share": 30.0},
            {"language": "Rust", "display": "Rust", "repositories": 1, "bytes": 33, "share": 10.0},
        ]

        rendered = MODULE.render_language_badges(rows)

        self.assertIn("[![C#]", rendered)
        self.assertIn("[![PL/pgSQL]", rendered)
        self.assertNotIn("[![C%23]", rendered)
        self.assertNotIn("[![PL%2FpgSQL]", rendered)
        self.assertIn("/badge/C%23-2%20repos-", rendered)
        self.assertIn("/badge/PL%2FpgSQL-2%20repos-", rendered)
        self.assertIn("language=PLpgSQL", rendered)


class ReplaceBlockTests(unittest.TestCase):
    """O corpo renderizado é texto literal, nunca sintaxe de substituição do `re`.

    Descrições de repositório vêm do GitHub e podem conter `\\`. Antes, o corpo
    ia como string de substituição para `re.sub`, e `C:\\Users` derrubava o
    refresh inteiro com `re.error: bad escape \\U` (auditoria, A5).
    """

    TEMPLATE = "antes\n<!-- X:START -->\nvelho\n<!-- X:END -->\ndepois\n"

    def test_barra_invertida_passa_literal(self) -> None:
        for body in (r"C:\Users\lucas tool", r"regex \d+ e \w", r"grupo \g<0> e \1", "fim com \\"):
            with self.subTest(body=body):
                rendered = MODULE.replace_block(self.TEMPLATE, "X", body)
                self.assertIn(f"<!-- X:START -->\n{body}\n<!-- X:END -->", rendered)

    def test_so_o_bloco_muda(self) -> None:
        rendered = MODULE.replace_block(self.TEMPLATE, "X", "novo")
        self.assertEqual("antes\n<!-- X:START -->\nnovo\n<!-- X:END -->\ndepois\n", rendered)

    def test_marcador_ausente_continua_falhando(self) -> None:
        with self.assertRaises(ValueError):
            MODULE.replace_block(self.TEMPLATE, "Y", "novo")


class SourceTimestampTests(unittest.TestCase):
    """O carimbo "atualizado em" sai dos dados, não do relógio (auditoria, A7)."""

    NOW = MODULE.datetime(2026, 9, 25, 12, 0, tzinfo=MODULE.timezone.utc)

    def test_push_do_proprio_perfil_nao_dita_o_carimbo(self) -> None:
        repos = [
            {"full_name": "Lucas-Belucci-Bellini/projeto", "pushed_at": "2026-09-01T10:00:00Z"},
            # O monitor horário empurra para o perfil o tempo todo.
            {"full_name": "Lucas-Belucci-Bellini/Lucas-Belucci-Bellini", "pushed_at": "2026-09-25T11:17:00Z"},
            {"full_name": "lucas-belucci-bellini/LUCAS-BELUCCI-BELLINI", "pushed_at": "2026-09-25T11:59:00Z"},
        ]
        self.assertEqual("2026-09-01 10:00 UTC", MODULE.source_timestamp(repos, self.NOW).strftime("%Y-%m-%d %H:%M UTC"))

    def test_mesmo_inventario_mesmo_carimbo_em_horas_diferentes(self) -> None:
        repos = [{"full_name": "Lucas-Belucci-Bellini/projeto", "pushed_at": "2026-09-01T10:00:00Z"}]
        manha = MODULE.source_timestamp(repos, self.NOW)
        noite = MODULE.source_timestamp(repos, self.NOW + timedelta(hours=10))
        self.assertEqual(manha, noite)

    def test_sem_data_valida_cai_no_relogio(self) -> None:
        repos = [{"full_name": "Lucas-Belucci-Bellini/projeto", "pushed_at": None}]
        self.assertEqual(self.NOW, MODULE.source_timestamp(repos, self.NOW))

    def test_so_o_perfil_cai_no_relogio(self) -> None:
        repos = [{"full_name": "Lucas-Belucci-Bellini/Lucas-Belucci-Bellini", "pushed_at": "2026-09-25T11:17:00Z"}]
        self.assertEqual(self.NOW, MODULE.source_timestamp(repos, self.NOW))

    def test_updated_at_serve_quando_falta_pushed_at(self) -> None:
        repos = [{"full_name": "Lucas-Belucci-Bellini/projeto", "updated_at": "2026-08-01T00:00:00Z"}]
        self.assertEqual(2026, MODULE.source_timestamp(repos, self.NOW).year)
        self.assertEqual(8, MODULE.source_timestamp(repos, self.NOW).month)


if __name__ == "__main__":
    unittest.main()

