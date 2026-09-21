"""Testes do catálogo de projetos: descoberta, verificação e regra de CTA.

A verificação de site é testada contra um servidor HTTP local, não contra a
internet: os casos de redirecionamento, 404 e host inexistente precisam ser
determinísticos, e um teste que depende da vercel.app estar no ar não prova
nada sobre este código.
"""
from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location(
    "project_catalog", ROOT / "scripts" / "project_catalog.py"
)
assert SPEC and SPEC.loader
PC = importlib.util.module_from_spec(SPEC)
# @dataclass resolve as anotações via sys.modules[cls.__module__]; um módulo
# carregado por importlib sem registro ali falha na criação da classe.
sys.modules["project_catalog"] = PC
SPEC.loader.exec_module(PC)


class _Handler(BaseHTTPRequestHandler):
    """Rotas fixas: uma OK, uma que redireciona, uma 404, uma 500."""

    def do_GET(self) -> None:  # noqa: N802 - assinatura da stdlib
        if self.path == "/ok":
            body = b"<html><body>online</body></html>"
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif self.path == "/redirect":
            self.send_response(302)
            self.send_header("Location", "/ok")
            self.end_headers()
        elif self.path == "/gone":
            self.send_response(404)
            self.send_header("Content-Length", "0")
            self.end_headers()
        else:
            self.send_response(500)
            self.send_header("Content-Length", "0")
            self.end_headers()

    def log_message(self, *args: object) -> None:  # silencia o servidor de teste
        return


class WebsiteCheckTests(unittest.TestCase):
    server: ThreadingHTTPServer
    base: str

    @classmethod
    def setUpClass(cls) -> None:
        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), _Handler)
        threading.Thread(target=cls.server.serve_forever, daemon=True).start()
        host, port = cls.server.server_address[:2]
        cls.base = f"http://{host}:{port}"

    @classmethod
    def tearDownClass(cls) -> None:
        cls.server.shutdown()
        cls.server.server_close()

    def test_site_no_ar_e_verificado(self) -> None:
        check = PC.check_website(f"{self.base}/ok", timeout=5, retries=0)
        self.assertTrue(check.is_live)
        self.assertEqual("verified", check.status)
        self.assertEqual(200, check.http_status)

    def test_redirecionamento_conta_como_no_ar_e_registra_o_destino(self) -> None:
        check = PC.check_website(f"{self.base}/redirect", timeout=5, retries=0)
        self.assertTrue(check.is_live)
        # urllib segue o 302; o destino final é o que responde.
        self.assertEqual(f"{self.base}/ok", check.final_url)
        self.assertNotEqual(check.url, check.final_url)

    def test_site_inexistente_nao_e_publicado_como_online(self) -> None:
        check = PC.check_website(f"{self.base}/gone", timeout=5, retries=0)
        self.assertFalse(check.is_live)
        self.assertEqual("unreachable", check.status)
        self.assertEqual(404, check.http_status)

    def test_host_que_recusa_conexao_falha_sem_explodir(self) -> None:
        # Porta fechada em loopback: falha de transporte determinística, sem DNS
        # e sem sair da máquina — o mesmo ramo de código de um host que sumiu.
        check = PC.check_website("http://127.0.0.1:9/", timeout=3, retries=0)
        self.assertFalse(check.is_live)
        self.assertEqual("unreachable", check.status)
        self.assertEqual(0, check.http_status)

    def test_url_malformada_e_recusada_antes_da_rede(self) -> None:
        for bad in ("", "nao-e-url", "ftp://exemplo.com", "javascript:alert(1)"):
            with self.subTest(bad=bad):
                self.assertEqual("invalid", PC.check_website(bad, timeout=1, retries=0).status)

    def test_url_repetida_e_verificada_uma_vez_so(self) -> None:
        url = f"{self.base}/ok"
        results = PC.check_websites([url, url, url], timeout=5, retries=0)
        self.assertEqual(1, len(results))
        self.assertIn(url, results)


class DiscoveryTests(unittest.TestCase):
    def test_homepage_do_github_ganha_do_manifesto(self) -> None:
        repo = {
            "full_name": "dono/projeto",
            "homepage": "https://do-github.example",
        }
        overrides = PC.normalize_site_overrides(
            {"dono/projeto": "https://do-manifesto.example"}
        )
        url, source = PC.discover_project_website(repo, overrides)
        self.assertEqual("https://do-github.example", url)
        self.assertEqual("github_homepage", source)

    def test_manifesto_cobre_a_homepage_vazia(self) -> None:
        repo = {"full_name": "dono/projeto", "homepage": ""}
        overrides = PC.normalize_site_overrides(
            {"dono/projeto": "https://do-manifesto.example"}
        )
        url, source = PC.discover_project_website(repo, overrides)
        self.assertEqual("https://do-manifesto.example", url)
        self.assertEqual("manifest", source)

    def test_nunca_inventa_url_a_partir_do_nome(self) -> None:
        repo = {"full_name": "dono/meu-projeto", "name": "meu-projeto", "homepage": None}
        url, source = PC.discover_project_website(repo, {})
        self.assertIsNone(url)
        self.assertEqual("none", source)

    def test_manifesto_aceita_os_dois_formatos(self) -> None:
        legado = PC.normalize_site_overrides({"a/b": "https://x.example"})
        novo = PC.normalize_site_overrides({"a/b": {"website": "https://x.example", "nota": "n"}})
        self.assertEqual("https://x.example", legado["a/b"]["website"])
        self.assertEqual("https://x.example", novo["a/b"]["website"])
        self.assertEqual("n", novo["a/b"]["nota"])

    def test_entrada_invalida_no_manifesto_e_ignorada(self) -> None:
        self.assertEqual({}, PC.normalize_site_overrides({"a/b": {"sem_website": 1}}))
        self.assertEqual({}, PC.normalize_site_overrides(["nao", "e", "objeto"]))


def _presentation(**kwargs):
    repo = {
        "full_name": "dono/projeto",
        "name": "projeto",
        "private": kwargs.pop("private", False),
        "fork": False,
        "archived": False,
    }
    return PC.resolve_presentation(
        repo,
        check=kwargs.pop("check", None),
        website=kwargs.pop("website", None),
        website_source=kwargs.pop("website_source", "none"),
        category=kwargs.pop("category", "Web"),
        status=kwargs.pop("status", "🟢 Active"),
        description=kwargs.pop("description", "Uma descrição pública suficientemente longa."),
        featured_order=kwargs.pop("featured_order", None),
    )


class PresentationTests(unittest.TestCase):
    LIVE = PC.WebsiteCheck("https://x.example", "verified", 200, "https://x.example", "t")
    DOWN = PC.WebsiteCheck("https://x.example", "unreachable", 404, "https://x.example", "t")

    def test_site_no_ar_vira_cta_primario_e_github_secundario(self) -> None:
        item = _presentation(check=self.LIVE, website="https://x.example", website_source="manifest")
        self.assertEqual("website", item.primary_cta)
        self.assertEqual("github", item.secondary_cta)
        self.assertTrue(item.has_live_website)

    def test_sem_site_o_github_assume_como_cta_primario(self) -> None:
        item = _presentation()
        self.assertEqual("github", item.primary_cta)
        self.assertIsNone(item.secondary_cta)
        self.assertIsNone(item.website)

    def test_site_fora_do_ar_nao_e_publicado(self) -> None:
        item = _presentation(check=self.DOWN, website="https://x.example", website_source="manifest")
        self.assertIsNone(item.website)
        self.assertEqual("github", item.primary_cta)

    def test_repositorio_privado_nunca_anuncia_site(self) -> None:
        item = _presentation(private=True, check=self.LIVE, website="https://x.example")
        self.assertIsNone(item.website)
        self.assertEqual("github", item.primary_cta)
        self.assertTrue(item.private)

    def test_celula_de_acesso_poe_o_site_na_frente(self) -> None:
        com_site = PC.cta_cell(_presentation(check=self.LIVE, website="https://x.example"))
        sem_site = PC.cta_cell(_presentation())
        self.assertLess(com_site.index("Abrir site"), com_site.index("código"))
        self.assertIn("**[▸ Abrir site](https://x.example)**", com_site)
        self.assertNotIn("Abrir site", sem_site)
        self.assertIn("[código](https://github.com/dono/projeto)", sem_site)

    def test_site_no_ar_pesa_mais_que_qualquer_outro_fator(self) -> None:
        com = _presentation(check=self.LIVE, website="https://x.example")
        sem = _presentation()
        self.assertGreater(com.marketing_priority, sem.marketing_priority)
        self.assertLessEqual(com.marketing_priority, 100)

    def test_curadoria_manual_respeita_a_ordem_do_manifesto(self) -> None:
        primeiro = _presentation(featured_order=1)
        setimo = _presentation(featured_order=7)
        self.assertGreater(primeiro.marketing_priority, setimo.marketing_priority)
        self.assertTrue(primeiro.featured)
        self.assertFalse(_presentation().featured)

    def test_categoria_do_readme_vira_taxonomia_canonica(self) -> None:
        item = _presentation(category="IA & Automação")
        self.assertEqual("AI & Intelligence", item.category)
        self.assertEqual("IA & Automação", item.category_label)
        self.assertIn(item.category, PC.CATEGORIES)

    def test_categoria_desconhecida_nao_cria_categoria_nova(self) -> None:
        item = _presentation(category="Categoria Inventada")
        self.assertEqual("Experimental", item.category)
        self.assertIn(item.category, PC.CATEGORIES)


class CatalogTests(unittest.TestCase):
    LIVE = PC.WebsiteCheck("https://x.example", "verified", 200, "https://x.example", "t")

    def test_catalogo_nao_guarda_carimbo_de_tempo(self) -> None:
        entry = PC.catalog_entry(_presentation(check=self.LIVE, website="https://x.example"))
        self.assertNotIn("checked_at", entry)
        self.assertNotIn("generated_at", json.dumps(PC.build_catalog([_presentation()])))

    def test_mesma_entrada_produz_exatamente_o_mesmo_arquivo(self) -> None:
        um = PC.build_catalog([_presentation(check=self.LIVE, website="https://x.example")])
        dois = PC.build_catalog([_presentation(check=self.LIVE, website="https://x.example")])
        self.assertEqual(json.dumps(um, sort_keys=True), json.dumps(dois, sort_keys=True))

    def test_nao_grava_quando_o_conteudo_nao_muda(self) -> None:
        catalog = PC.build_catalog([_presentation()])
        with tempfile.TemporaryDirectory() as tmp:
            destination = Path(tmp) / "project-catalog.json"
            self.assertTrue(PC.write_catalog_if_changed(catalog, destination))
            self.assertFalse(PC.write_catalog_if_changed(catalog, destination))
            catalog["counts"]["projects"] = 99
            self.assertTrue(PC.write_catalog_if_changed(catalog, destination))

    def test_ordenacao_e_por_prioridade_depois_nome(self) -> None:
        com_site = _presentation(check=self.LIVE, website="https://x.example")
        sem_site = _presentation()
        catalog = PC.build_catalog([sem_site, com_site])
        self.assertEqual("website", catalog["projects"][0]["primary_cta"])
        self.assertEqual(1, catalog["counts"]["with_live_website"])

    def test_entrada_privada_nao_carrega_website(self) -> None:
        entry = PC.catalog_entry(_presentation(private=True, check=self.LIVE, website="https://x.example"))
        self.assertNotIn("website", entry)
        self.assertTrue(entry["private"])
        self.assertEqual("github", entry["primary_cta"])


if __name__ == "__main__":
    unittest.main()
