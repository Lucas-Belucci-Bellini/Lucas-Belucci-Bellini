import importlib.util
import unittest
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


if __name__ == "__main__":
    unittest.main()

