"""Testes dos renderizadores da vitrine: cards, mapa do ecossistema e curadoria.

Estes testes não tocam a rede: montam `Presentation` diretamente e conferem o
que o README vai receber. O que está em jogo é a regra de hierarquia — o site
na frente do código — e o fato de o selo de estado sair sempre da verificação.
"""
from __future__ import annotations

import importlib.util
import sys
import unittest
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _carregar(nome: str, caminho: Path):
    spec = importlib.util.spec_from_file_location(nome, caminho)
    assert spec and spec.loader
    modulo = importlib.util.module_from_spec(spec)
    sys.modules[nome] = modulo
    spec.loader.exec_module(modulo)
    return modulo


PC = _carregar("project_catalog", ROOT / "scripts" / "project_catalog.py")
UP = _carregar("update_profile", ROOT / "scripts" / "update_profile.py")

NO_AR = PC.WebsiteCheck("https://x.example", "verified", 200, "https://x.example", "t")
FORA = PC.WebsiteCheck("https://y.example", "unreachable", 404, "https://y.example", "t")


def repo(nome: str, **extra):
    base = {
        "name": nome,
        "full_name": f"dono/{nome}",
        "description": f"Plataforma de teste chamada {nome}, com descrição suficiente.",
        "private": False,
        "fork": False,
        "archived": False,
        "homepage": "",
        "size": 1000,
        "pushed_at": "2026-09-01T00:00:00Z",
    }
    base.update(extra)
    return base


def montar(repos, sites, checks, manifest=None):
    return UP.build_presentations(
        repos,
        sites=sites,
        checks=checks,
        featured_manifest=manifest or {"projects": []},
        now=datetime(2026, 9, 21, tzinfo=timezone.utc),
    )


class ProductCardTests(unittest.TestCase):
    def setUp(self) -> None:
        self.repos = [repo("ComSite"), repo("SemSite"), repo("Privado", private=True)]
        self.sites = {"dono/ComSite": ("https://x.example", "github_homepage"),
                      "dono/SemSite": (None, "none")}
        self.pres = montar(self.repos, self.sites, {"https://x.example": NO_AR})

    def test_card_poe_o_botao_do_site_antes_do_codigo(self) -> None:
        html = UP.render_product_cards(self.repos, self.pres)
        self.assertLess(html.index("ABRIR%20SITE"), html.index("C%C3%93DIGO"))
        self.assertIn("](https://x.example)", html)

    def test_projeto_sem_site_mostra_so_o_codigo(self) -> None:
        html = UP.render_product_cards([self.repos[1]], self.pres)
        self.assertNotIn("ABRIR%20SITE", html)
        self.assertIn("C%C3%93DIGO", html)

    def test_repositorio_privado_nao_vira_card(self) -> None:
        html = UP.render_product_cards(self.repos, self.pres)
        self.assertNotIn("Privado", html)

    def test_projeto_com_site_no_ar_vem_antes(self) -> None:
        html = UP.render_product_cards(self.repos, self.pres)
        self.assertLess(html.index("ComSite"), html.index("SemSite"))

    def test_grade_fecha_com_celula_vazia_quando_a_linha_e_impar(self) -> None:
        html = UP.render_product_cards([self.repos[0]], self.pres, columns=2)
        self.assertEqual(html.count("<tr>"), html.count("</tr>"))
        self.assertEqual(2, html.count('<td width="50%"'))

    def test_selo_de_estado_vem_da_verificacao(self) -> None:
        repos = [repo("Caido", homepage="https://y.example")]
        pres = montar(repos, {"dono/Caido": ("https://y.example", "github_homepage")},
                      {"https://y.example": FORA})
        html = UP.render_product_cards(repos, pres)
        self.assertIn("SITE%20FORA%20DO%20AR", html)
        self.assertNotIn("SITE%20VERIFICADO", html)
        self.assertNotIn("https://y.example", html)

    def test_descricao_longa_e_cortada_sem_quebrar_palavra(self) -> None:
        item = list(self.pres.values())[0]
        item.description = "palavra " * 60
        resumo = UP.card_summary(item, limit=40)
        self.assertTrue(resumo.endswith("…"))
        self.assertLessEqual(len(resumo), 41)
        self.assertNotIn(" …", resumo)


class EcosystemMapTests(unittest.TestCase):
    def test_mapa_conta_projetos_e_sites_por_ramo(self) -> None:
        repos = [repo("A"), repo("B"), repo("Privado", private=True)]
        sites = {"dono/A": ("https://x.example", "manifest"), "dono/B": (None, "none")}
        pres = montar(repos, sites, {"https://x.example": NO_AR})
        mapa = UP.render_ecosystem_map(repos, pres)
        self.assertIn("```text", mapa)
        self.assertIn("ECOSSISTEMA", mapa)
        self.assertIn("com site", mapa)
        self.assertNotIn("Privado", mapa)
        self.assertIn("A ●", mapa)

    def test_singular_quando_o_ramo_tem_um_projeto(self) -> None:
        repos = [repo("Unico")]
        pres = montar(repos, {"dono/Unico": (None, "none")}, {})
        mapa = UP.render_ecosystem_map(repos, pres)
        self.assertIn(" 1 projeto ", mapa)
        self.assertNotIn(" 1 projetos", mapa)

    def test_mapa_vazio_nao_explode(self) -> None:
        self.assertIn("Nenhum projeto", UP.render_ecosystem_map([], {}))


class CuratedManifestTests(unittest.TestCase):
    MANIFEST = {
        "projects": [
            {"name": "Caido", "label": "L", "focus": "F", "order": 1, "website_required": True},
            {"name": "Firme", "label": "L", "focus": "F", "order": 2, "priority": 60},
        ]
    }

    def test_website_required_retira_a_entrada_sem_site_no_ar(self) -> None:
        repos = [repo("Caido", homepage="https://y.example"), repo("Firme")]
        sites = {"dono/Caido": ("https://y.example", "github_homepage"),
                 "dono/Firme": (None, "none")}
        pres = montar(repos, sites, {"https://y.example": FORA}, self.MANIFEST)
        tabela = UP.render_curated_featured(
            repos, pres, self.MANIFEST, datetime(2026, 9, 21, tzinfo=timezone.utc)
        )
        self.assertNotIn("Caido", tabela)
        self.assertIn("Firme", tabela)

    def test_priority_explicita_substitui_a_derivada_da_ordem(self) -> None:
        r = repo("Firme")
        base = PC.marketing_priority(r, None, featured_order=2)
        alta = PC.marketing_priority(r, None, featured_order=2, featured_priority=100)
        baixa = PC.marketing_priority(r, None, featured_order=2, featured_priority=0)
        self.assertGreater(alta, base)
        self.assertLess(baixa, base)

    def test_manifesto_versionado_do_repositorio_continua_legivel(self) -> None:
        manifest = UP.load_json_object(UP.FEATURED_FILE)
        for entry in manifest["projects"]:
            self.assertIn("order", entry)
            self.assertIn("priority", entry)
            self.assertIsInstance(entry["website_required"], bool)
            self.assertTrue(entry["reason"].strip())


class SlugTests(unittest.TestCase):
    def test_slug_e_estavel_e_seguro_para_rota(self) -> None:
        pres = montar([repo("Meu Projeto -- 2!")], {"dono/Meu Projeto -- 2!": (None, "none")}, {})
        item = list(pres.values())[0]
        self.assertEqual("meu-projeto-2", item.slug)


if __name__ == "__main__":
    unittest.main()
