import importlib.util
import unittest
from urllib.error import HTTPError
from datetime import timedelta
from pathlib import Path

from fake_github import FakeGitHub


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


class GitHubFetchTests(unittest.TestCase):
    """Coleta do inventário contra um GitHub simulado (GITHUB_API_URL).

    É o comportamento que o `profile-core` reproduz: página cheia (100) pede
    a próxima; com token, `/user/repos` (privados inclusive); sem token, só os
    públicos do dono; linguagem que falha vira mapa vazio, sem derrubar nada.
    """

    def setUp(self) -> None:
        self.saved = MODULE.API
        self.fake = FakeGitHub()

    def tearDown(self) -> None:
        MODULE.API = self.saved

    def test_pagina_cheia_pede_a_proxima_e_token_muda_o_endpoint(self) -> None:
        owner = MODULE.OWNER
        page1 = [{"name": f"r{i}", "full_name": f"{owner}/r{i}"} for i in range(100)]
        self.fake.get(f"/users/{owner}/repos?type=owner&per_page=100&page=1", page1)
        self.fake.get(f"/users/{owner}/repos?type=owner&per_page=100&page=2", [{"name": "last"}])
        self.fake.get("/user/repos?affiliation=owner,collaborator,organization_member&per_page=100&page=1", [])
        with self.fake:
            MODULE.API = self.fake.url
            public = MODULE.fetch_repositories(None)
            private_aware = MODULE.fetch_repositories("t0ken")
        self.assertEqual(101, len(public))
        self.assertEqual([], private_aware)
        self.assertEqual([None, None, "Bearer t0ken"], [auth for _, _, auth in self.fake.requests])

    def test_erro_no_inventario_propaga_e_na_linguagem_vira_vazio(self) -> None:
        self.fake.get("/repos/o/ok/languages", {"Rust": 10, "Python": "7"})
        self.fake.get("/repos/o/gone/languages", {"message": "Not Found"}, status=404)
        self.fake.get("/repos/o/bad/languages", raw=b"<html>")
        with self.fake:
            MODULE.API = self.fake.url
            self.assertEqual({"Rust": 10, "Python": 7}, MODULE.fetch_languages("o/ok", None))
            self.assertEqual({}, MODULE.fetch_languages("o/gone", None))
            self.assertEqual({}, MODULE.fetch_languages("o/bad", None))
            with self.assertRaises(HTTPError):
                MODULE.fetch_repositories(None)


if __name__ == "__main__":
    unittest.main()

