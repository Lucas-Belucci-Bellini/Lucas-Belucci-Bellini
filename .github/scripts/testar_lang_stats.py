#!/usr/bin/env python3
"""Prova o bot de análise do perfil contra respostas de API FABRICADAS.

    python3 .github/scripts/testar_lang_stats.py

## Por que existe

Este bot só roda de verdade dentro do Actions, com um PAT que enxerga os
repositórios privados. Não dá para conferir o resultado antes de publicar — e o
que ele publica é o README público do perfil. Um erro aqui não quebra nada:
imprime um número errado com a mesma confiança de um certo.

O teste central não é "a tabela sai bonita". É **cada número diz o que o rótulo
promete** — porque foi exatamente aí que a versão anterior errava: ela ordenava
tipos de arquivo por CONTAGEM e chamava aquilo de retrato do acervo, enquanto
por PESO o pódio era outro. Medido nos repositórios reais:

    por contagem   .webp 80,0%   .p3d 10,9%
    por peso       .p3d  84,4%   .json 6,4%

As duas leituras são quase invertidas. Nenhuma é falsa; publicar só uma
escolhia a história por conta própria.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lang_stats as ls                                          # noqa: E402

falhas: list[str] = []


def checar(cond, titulo, detalhe=""):
    if cond:
        print(f"  ✓ {titulo}")
    else:
        print(f"  ✗ {titulo}")
        if detalhe:
            print(f"      {detalhe}")
        falhas.append(titulo)


# ── um perfil sintético com a mesma DISTORÇÃO do real ──────────────────────
# Muitos arquivos minúsculos de um lado, poucos e enormes do outro. É o caso
# em que contagem e peso discordam, que é o caso que importa.
REPOS = [
    {"name": "com-assets", "owner": {"login": "u"}, "private": False,
     "fork": False, "default_branch": "main"},
    {"name": "so-codigo", "owner": {"login": "u"}, "private": True,
     "fork": False, "default_branch": "main"},
    {"name": "um-fork", "owner": {"login": "u"}, "private": False,
     "fork": True, "default_branch": "main"},
]

ARVORES = {
    "com-assets": {"tree": [
        *[{"type": "blob", "path": f"icones/i{i}.webp", "size": 1024}
          for i in range(1000)],                       # 1000 arquivos, ~1 MB
        {"type": "blob", "path": "modelos/tanque.p3d", "size": 500 * 1024 * 1024},
        {"type": "blob", "path": "LICENSE", "size": 1000},        # sem extensão
        {"type": "blob", "path": ".gitignore", "size": 40},       # ponto-nome
        {"type": "tree", "path": "icones", "size": 0},            # não é blob
    ]},
    "so-codigo": {"tree": [
        {"type": "blob", "path": "src/app.js", "size": 40 * 1024},
        {"type": "blob", "path": "src/util.py", "size": 20 * 1024},
        {"type": "blob", "path": "Main.class", "size": 8 * 1024},
    ]},
}

LINGUAGENS = {
    "com-assets": {"JavaScript": 300_000},
    "so-codigo": {"JavaScript": 40_000, "Python": 20_000},
}

GITATTRIBUTES = {"com-assets": "src/data/*.js linguist-generated=true\n"}


def instalar_falso():
    """Troca `api` por uma que responde do dicionário acima."""
    import base64

    def falso(path, params=None, retry_denied=True):
        if path == "/user/repos":
            return REPOS if (params or {}).get("page", "1") == "1" else []
        if path.startswith("/users/"):
            return REPOS if (params or {}).get("page", "1") == "1" else []
        for nome in ("com-assets", "so-codigo", "um-fork"):
            if path == f"/repos/u/{nome}/languages":
                return LINGUAGENS.get(nome, {})
            if path.startswith(f"/repos/u/{nome}/git/trees/"):
                return ARVORES.get(nome)
            if path == f"/repos/u/{nome}/contents/.gitattributes":
                txt = GITATTRIBUTES.get(nome)
                if txt is None:
                    return None
                return {"content": base64.b64encode(txt.encode()).decode()}
        return None

    ls.api = falso
    ls.TOKEN = "falso"


def teste_extensao():
    checar(ls.extensao("a/b/x.WEBP") == "webp", "extensão vem em minúscula")
    checar(ls.extensao("LICENSE") is None, "arquivo sem extensão não vira tipo")
    checar(ls.extensao(".gitignore") is None, "ponto-nome não vira tipo `gitignore`")
    checar(ls.extensao("x.umaextensaoabsurdamentelonga") is None,
           "extensão absurda é nome com ponto, não tipo")
    checar(ls.extensao("v1.2/arquivo") is None, "ponto no DIRETÓRIO não conta")


def teste_peso_e_contagem_sao_medidas_DIFERENTES():
    """O coração do teste: o mesmo acervo, dois pódios."""
    d = ls.collect()

    checar(d["repo_count"] == 2, "fork fica de fora por padrão", f"{d['repo_count']}")
    checar(d["privados"] == 1, "o privado é contado")

    por_contagem = max(d["per_ext"], key=lambda e: d["per_ext"][e])
    por_peso = max(d["per_ext_bytes"], key=lambda e: d["per_ext_bytes"][e])
    checar(por_contagem == "webp" and por_peso == "p3d",
           "contagem e peso apontam tipos DIFERENTES — é por isso que os dois "
           "vão para a tabela", f"contagem={por_contagem} peso={por_peso}")

    checar(d["per_ext"]["webp"] == 1000, "contagem de .webp bate")
    checar(d["per_ext_bytes"]["p3d"] == 500 * 1024 * 1024, "peso de .p3d bate")

    # os dois totais NÃO podem ser o mesmo número
    checar(d["bytes_arquivos"] > d["total_bytes"] * 100,
           "o total em disco e o total de código são grandezas distintas",
           f"disco={d['bytes_arquivos']} codigo={d['total_bytes']}")

    checar(d["arquivos_total"] == 1004,
           "LICENSE e .gitignore não entram; a árvore (não-blob) também não",
           f"{d['arquivos_total']}")
    return d


def teste_deteccao_de_gerado(d):
    checar(d["declaram_gerado"] == 1,
           "detecta o repo que declara linguist-generated")
    nomes = [n for n, _p in d["sem_declaracao"]]
    checar(nomes == ["so-codigo"],
           "e nomeia exatamente quem falta", f"{nomes}")


def teste_markdown(d):
    md = ls.build_markdown(d)

    checar(md.startswith(ls.START) and md.rstrip().endswith(ls.END),
           "o bloco sai entre os marcadores — patch_readme depende disso")

    i_peso, i_arq = md.find("| Peso |"), md.find("| Arquivos |")
    checar(i_peso > 0 and i_arq > i_peso,
           "a tabela de tipos traz PESO antes de contagem (ordenada por peso)")

    # o .p3d tem de vir ANTES do .webp na tabela, apesar de serem 1000 contra 1
    checar(md.find("`.p3d`") < md.find("`.webp`"),
           "o tipo mais PESADO encabeça, mesmo sendo 1 arquivo contra 1000")

    checar("de código" in md and "em disco" in md,
           "o resumo rotula as duas grandezas em vez de dizer só 'total'")
    checar("linguist-generated" in md,
           "o README explica COMO a linguagem é medida")
    checar("so-codigo" in md, "e diz qual repositório ainda não declara")

    checar("compilado" in md,
           "`.class` cai em 'compilado', não em 'código' — é bytecode do "
           "`.java` que já foi contado, e somar os dois conta o mesmo "
           "trabalho duas vezes")

    # nenhum percentual pode passar de 100 nem sair negativo
    import re
    ruins = [p for p in re.findall(r"`(\d+\.\d+)%`", md) if not 0 <= float(p) <= 100]
    checar(not ruins, "todo percentual fica entre 0 e 100", f"{ruins[:5]}")


def teste_svg_nao_quebra(d):
    svg = ls.build_svg(d)
    checar(svg.startswith("<svg") and svg.rstrip().endswith("</svg>"),
           "o SVG fecha")
    checar("&" not in svg.replace("&amp;", "").replace("&#", "").replace("&x", ""),
           "sem & solto — SVG mal formado não renderiza no GitHub")


def teste_acervo_vazio_nao_estoura():
    """Sem nenhum dado o bot tem de recusar, não publicar zeros."""
    vazio = {**ls.collect(), "per_lang": {}, "per_ext": {}, "per_ext_bytes": {},
             "arquivos_total": 0, "bytes_arquivos": 0, "total_bytes": 0}
    try:
        ls.build_markdown(vazio)
        checar(True, "acervo vazio não estoura na formatação")
    except ZeroDivisionError as err:
        checar(False, "acervo vazio não estoura na formatação", f"{err!r}")


def main():
    print("provando o bot de análise do perfil\n")
    instalar_falso()
    print("1. extensão → tipo")
    teste_extensao()
    print("\n2. peso e contagem são medidas diferentes")
    d = teste_peso_e_contagem_sao_medidas_DIFERENTES()
    print("\n3. o que o Linguist conta como código")
    teste_deteccao_de_gerado(d)
    print("\n4. o markdown publicado")
    teste_markdown(d)
    print("\n5. o SVG")
    teste_svg_nao_quebra(d)
    print("\n6. bordas")
    teste_acervo_vazio_nao_estoura()

    print()
    if falhas:
        print(f"✗ {len(falhas)} falha(s): {', '.join(falhas)}")
        return 1
    print("✓ cada número publicado diz o que o rótulo promete")
    return 0


if __name__ == "__main__":
    sys.exit(main())
