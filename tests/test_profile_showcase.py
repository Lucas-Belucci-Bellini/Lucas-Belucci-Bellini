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
        tabela = UP.render_featured_projects(
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


class FeaturedTableTests(unittest.TestCase):
    MANIFEST = {
        "intro": "intro editorial",
        "projects": [{"name": "Curado", "label": "ROTULO", "focus": "foco editorial", "order": 1}],
    }

    def setUp(self) -> None:
        self.repos = [repo("Curado"), repo("Heuristico"), repo("Privado", private=True)]
        self.sites = {f"dono/{n}": (None, "none") for n in ("Curado", "Heuristico")}
        self.pres = montar(self.repos, self.sites, {}, self.MANIFEST)

    def test_curadoria_vem_antes_da_heuristica(self) -> None:
        tabela = UP.render_featured_projects(
            self.repos, self.pres, self.MANIFEST, datetime(2026, 9, 21, tzinfo=timezone.utc)
        )
        self.assertLess(tabela.index("Curado"), tabela.index("Heuristico"))
        self.assertIn("**ROTULO** · Curado", tabela)
        self.assertIn("foco editorial", tabela)

    def test_nenhum_projeto_aparece_duas_vezes(self) -> None:
        tabela = UP.render_featured_projects(
            self.repos, self.pres, self.MANIFEST, datetime(2026, 9, 21, tzinfo=timezone.utc)
        )
        self.assertEqual(1, tabela.count("· Curado |"))

    def test_privado_fica_fora_da_tabela(self) -> None:
        tabela = UP.render_featured_projects(
            self.repos, self.pres, self.MANIFEST, datetime(2026, 9, 21, tzinfo=timezone.utc)
        )
        self.assertNotIn("Privado", tabela)

    def test_limite_e_respeitado(self) -> None:
        muitos = [repo(f"P{i}") for i in range(20)]
        pres = montar(muitos, {f"dono/P{i}": (None, "none") for i in range(20)}, {})
        tabela = UP.render_featured_projects(
            muitos, pres, {"projects": []}, datetime(2026, 9, 21, tzinfo=timezone.utc), maximum=4
        )
        self.assertEqual(4, sum(1 for l in tabela.splitlines() if l.startswith("| ") and "· P" in l))


class WebsiteDirectoryTests(unittest.TestCase):
    def test_diretorio_lista_so_o_que_esta_no_ar(self) -> None:
        repos = [repo("NoAr"), repo("Caido", homepage="https://y.example"), repo("Sem")]
        sites = {"dono/NoAr": ("https://x.example", "manifest"),
                 "dono/Caido": ("https://y.example", "github_homepage"),
                 "dono/Sem": (None, "none")}
        pres = montar(repos, sites, {"https://x.example": NO_AR, "https://y.example": FORA})
        d = UP.render_website_directory(repos, pres)
        self.assertIn("](https://x.example)", d)
        self.assertNotIn("y.example", d)
        self.assertNotIn("Sem", d)
        self.assertIn("**1 sites no ar**", d)

    def test_sem_site_nenhum_o_diretorio_diz_isso(self) -> None:
        repos = [repo("Sem")]
        pres = montar(repos, {"dono/Sem": (None, "none")}, {})
        self.assertIn("Nenhum site respondeu", UP.render_website_directory(repos, pres))


class WhatIBuildTests(unittest.TestCase):
    def test_dominios_mostram_contagem_real(self) -> None:
        repos = [repo("Jogo", description="Um game de teste"), repo("NoAr")]
        sites = {"dono/Jogo": (None, "none"), "dono/NoAr": ("https://x.example", "manifest")}
        pres = montar(repos, sites, {"https://x.example": NO_AR})
        html = UP.render_what_i_build(repos, pres)
        self.assertRegex(html, r"`\d+ projetos?` · `\d+ com site`")
        self.assertEqual(html.count("<tr>"), html.count("</tr>"))

    def test_dominio_vazio_nao_aparece_zerado(self) -> None:
        repos = [repo("SoWeb", homepage="https://x.example")]
        pres = montar(repos, {"dono/SoWeb": ("https://x.example", "github_homepage")},
                      {"https://x.example": NO_AR})
        html = UP.render_what_i_build(repos, pres)
        self.assertNotIn("`0 projetos`", html)

    def test_sem_projeto_publico_nao_explode(self) -> None:
        self.assertIn("Nenhum projeto", UP.render_what_i_build([], {}))

    def test_todo_rotulo_editorial_cai_em_algum_dominio(self) -> None:
        # Uma categoria de classify() sem domínio sumiria da vitrine em silêncio.
        rotulos = set(PC.CATEGORY_ALIASES)
        self.assertEqual(set(), rotulos - UP.DOMAIN_LABELS, "rótulo sem domínio")

    def test_grade_de_dominios_fecha_as_linhas(self) -> None:
        repos = [repo("Jogo"), repo("Outro")]
        pres = montar(repos, {f"dono/{n}": (None, "none") for n in ("Jogo", "Outro")}, {})
        html = UP.render_what_i_build(repos, pres, columns=3)
        self.assertEqual(html.count("<tr>"), html.count("</tr>"))
        self.assertEqual(3, html.count('<td width="33%"'))


class SlugTests(unittest.TestCase):
    def test_slug_e_estavel_e_seguro_para_rota(self) -> None:
        pres = montar([repo("Meu Projeto -- 2!")], {"dono/Meu Projeto -- 2!": (None, "none")}, {})
        item = list(pres.values())[0]
        self.assertEqual("meu-projeto-2", item.slug)


if __name__ == "__main__":
    unittest.main()
