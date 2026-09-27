"""Casos de paridade do monitor de sites: o Python é a referência, o Rust confere.

    OLD PYTHON ──▶ tests/fixtures/parity/site_monitor.json ◀── NEW RUST

Mesma ideia de `test_parity_domain.py`, para `scripts/check_websites.py` e a
verificação HTTP de `scripts/project_catalog.py`. Nada aqui reimplementa o
Python: cada saída esperada vem do código real da biblioteca padrão.

- `preflight`: o que o `urllib`/`http.client` fazem com a URL **antes** da
  rede — `Request`, `HTTPConnection`, `putrequest`/`putheader`. Decide se a
  checagem falha sem conectar e qual é o `final_url` quando não há redirect.
- `redirect`: o `HTTPRedirectHandler.http_error_302` de verdade, com um
  `parent` falso que devolve o pedido seguinte em vez de abri-lo.
- `collect_urls`: o `collect_urls()` do `check_websites.py` sobre uma raiz
  temporária com o catálogo e o manifesto dados (inclusive quando o Python
  quebra com exceção).
- `report`: o `main()` do `check_websites.py` com as checagens trocadas por
  resultados fixos — saída de texto, JSON e código de saída.

O comportamento de rede (status, redirects encadeados, laço, timeout, retry)
fica no teste de ponta a ponta `tests/e2e/check_sites_parity.py`, que roda os
dois contra o mesmo servidor local.

**Versão:** o `urllib` muda dentro da série 3.12 (o `urlunsplit` do 3.12.3
difere do 3.12.14). O fixture é do Python do CI e o teste só compara na mesma
versão menor; regenere com a versão exata do CI:

    UPDATE_PARITY=1 python3.12 -m unittest tests.test_parity_site_monitor
"""
from __future__ import annotations

import contextlib
import http.client
import importlib.util
import io
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.request import HTTPRedirectHandler, Request

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "parity" / "site_monitor.json"
REFERENCE_MINOR = (3, 12)
sys.path.insert(0, str(ROOT / "scripts"))


def _load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


PC = _load("project_catalog", ROOT / "scripts" / "project_catalog.py")
CW = _load("check_websites", ROOT / "scripts" / "check_websites.py")
UA = PC.USER_AGENT
CHECKED_AT = "2026-09-26T00:00:00+00:00"


# ------------------------------------------------------------- preflight

PREFLIGHT_URLS = [
    "https://example.org", "https://example.org/", "http://example.org/a/b?c=d", "https://example.org#frag",
    "https://example.org/p#a#b", "https://example.org/#", "https://example.org\n", "https://example.org?q=1",
    "https://example.org:8443/x", "https://example.org:/x", "https://example.org:abc/x", "https://[::1]:8080/x",
    "https://[::1]/x", "https://user@example.org/", "https://user:pw@example.org/", "https://ex%61mple.org/",
    "https://example.org/\x01", "https://example.org/\x7f", "https://example.org/ção", "https://ção.org/",
    "https://日本.jp/", "https://EXAMPLE.org/Path", "https://example.org/a/../b", "https://example.org//x",
    "https://example.org:443/", "http://example.org:80/", "https://example.org:+443/", "https://example.org:4_43/",
    "https://example.org/%zz", "https://ex ample.org/", "https://example.org/?a=b c",
    "https://" + "a" * 64 + ".org/", "https://xn--9ca.org/", "https://a.b.c.d.e.example.org/~user",
]


def preflight(url: str) -> dict[str, Any]:
    """Tudo o que `urlopen` faz com a URL antes de abrir o socket."""
    try:
        req = Request(url, headers={"User-Agent": UA}, method="GET")
        if req.type not in ("http", "https"):
            raise URLError("unknown url type")
        if not req.host:
            raise URLError("no host given")  # do_request_
        cls = http.client.HTTPSConnection if req.type == "https" else http.client.HTTPConnection
        conn = cls(req.host, timeout=1)  # _get_hostport + _validate_host
        headers = {"Host": req.host, "User-Agent": UA, "Connection": "close"}
        conn.putrequest("GET", req.selector, skip_host=1)  # _validate_path + linha de pedido ASCII
        for name, value in headers.items():
            conn.putheader(name, value)  # valores em latin-1
        if not conn.host.isascii():
            conn.host.encode("idna")  # o que getaddrinfo faz com host não ASCII
        request_line = conn._buffer[0].decode("ascii")
        return {
            "ok": True,
            "full_url": req.full_url,
            "connect_host": conn.host,
            "port": conn.port,
            "request_target": request_line.split(" ")[1],
            "host_header": req.host,
        }
    except Exception as error:  # noqa: BLE001 — registrar o tipo é o objetivo
        caught = isinstance(error, (URLError, TimeoutError, ValueError, OSError))
        return {"ok": False, "error": type(error).__name__, "caught": caught}


# -------------------------------------------------------------- redirect

BASE = "https://example.org/dir/page?x=1#top"
REDIRECTS: list[tuple[str, str | None, str | None]] = [
    (BASE, "https://www.example.org/", None),
    (BASE, "https://www.example.org", None),
    (BASE, "HTTPS://WWW.Example.org/A", None),
    (BASE, "/abs/path", None),
    (BASE, "rel", None),
    (BASE, "./rel", None),
    (BASE, "../up", None),
    (BASE, "../../../../too-far", None),
    (BASE, "..", None),
    (BASE, ".", None),
    (BASE, "?only=query", None),
    (BASE, "#only-fragment", None),
    (BASE, "", None),
    (BASE, "//other.example.org/x", None),
    (BASE, "//other.example.org", None),
    (BASE, "/a b/c", None),
    (BASE, "/tab\there", None),
    (BASE, "/ção", None),
    (BASE, "/\xe7\xe3o", None),  # bytes UTF-8 de "ção" lidos como latin-1, como o http.client faz
    (BASE, "/já%20codificado", None),
    (BASE, "/a;params?q#f", None),
    (BASE, "mailto:alguem@example.org", None),
    (BASE, "javascript:alert(1)", None),
    (BASE, "ftp://example.org/file", None),
    (BASE, "file:///etc/passwd", None),
    (BASE, "data:text/plain,oi", None),
    (BASE, "http://[::1/", None),
    (BASE, "http://[::1]:8080/x", None),
    (BASE, "https://example.org:8443", None),
    (BASE, "https://user:pw@example.org/", None),
    (BASE, "/a/./b/../c/", None),
    (BASE, "a//b", None),
    (BASE, "<URL:https://wrapped.example.org/>", None),
    (BASE, "https://example.org/#", None),
    (BASE, "///triple", None),
    (BASE, "////quad", None),
    (BASE, "http:////x", None),
    (BASE, "http:x", None),
    (BASE, "https:rel", None),
    (BASE, "HTTP://Example.org", None),
    (BASE, "\x00/nul", None),
    (BASE, None, "https://uri-header.example.org/"),
    (BASE, None, None),
    ("https://example.org", "next", None),
    ("https://example.org", "/", None),
    ("https://example.org/a/b/", "c/../../d", None),
    ("https://example.org/a/b/", "./", None),
    ("https://example.org/a;p?q", ";x", None),
    ("http://example.org/a", "https://example.org/a", None),
]


class _Parent:
    """`OpenerDirector` falso: devolve o pedido seguinte em vez de abri-lo."""

    def open(self, new, timeout=None):  # noqa: A003
        return new


def redirect(base: str, location: str | None, uri: str | None) -> dict[str, Any]:
    raw = b""
    if location is not None:
        raw += b"Location: " + location.encode("latin-1") + b"\r\n"
    if uri is not None:
        raw += b"URI: " + uri.encode("latin-1") + b"\r\n"
    headers = http.client.parse_headers(io.BytesIO(raw + b"\r\n"))
    handler = HTTPRedirectHandler()
    handler.parent = _Parent()
    req = Request(base, headers={"User-Agent": UA})
    req.timeout = 15.0  # OpenerDirector.open() define antes de chamar os handlers
    try:
        new = handler.http_error_302(req, io.BytesIO(b""), 302, "Found", headers)
    except HTTPError as error:
        return {"kind": "not_allowed", "url": error.url}
    except Exception as error:  # noqa: BLE001
        caught = isinstance(error, (URLError, TimeoutError, ValueError, OSError))
        return {"kind": "error", "error": type(error).__name__, "caught": caught}
    if new is None:
        return {"kind": "no_location"}
    (visited_key,) = req.redirect_dict.keys()
    return {"kind": "follow", "visited_key": visited_key, "full_url": new.full_url}


# ---------------------------------------------------------- collect_urls

def _catalog(*projects: dict[str, Any]) -> str:
    return json.dumps({"projects": list(projects)})


COLLECT: list[tuple[str | None, str | None]] = [
    (None, None),
    (_catalog({"repository": "o/a", "website_declared": "https://a.example.org", "website": "https://a2.example.org"},
              {"repository": "o/b", "website": "https://b.example.org"},
              {"repository": "o/c"}),
     json.dumps({"o/b": "https://b-manifest.example.org", "o/d": {"website": "https://d.example.org"}})),
    ("{ não é json", json.dumps({"o/m": "https://m.example.org"})),
    (_catalog({"repository": "o/a", "website": "https://a.example.org"}), "[ quebrado"),
    (_catalog({"repository": "o/dup", "website": "https://1.example.org"},
              {"repository": "o/dup", "website": "https://2.example.org"}), None),
    (_catalog({"repository": None, "website": "https://sem-repo.example.org"},
              {"website": "https://sem-chave.example.org"},
              {"repository": 42, "website": 7},
              {"repository": "o/vazio", "website_declared": "", "website": "https://fallback.example.org"},
              {"repository": "o/falso", "website_declared": False}), None),
    (None, json.dumps({"o/s": "", "o/z": {"website": 0}, "o/t": {"website": True}, "o/n": {"website": 42},
                       "o/l": ["https://lista.example.org"], "o/x": {"note": "sem site"}, "o/nul": None})),
    (json.dumps({"projects": {}}), None),
    (json.dumps({"projects": ""}), None),
    (json.dumps({"projects": []}), json.dumps([])),
    (json.dumps({"other": 1}), json.dumps("texto")),
    ("﻿" + _catalog({"repository": "o/bom", "website": "https://bom.example.org"}), None),
    (json.dumps({"o/dup": "https://1.example.org", "o/dup2": "x"})[:-1] + ', "o/dup": "https://2.example.org"}', None),
    (None, '{"o/dup": "https://1.example.org", "o/dup": "https://2.example.org"}'),
    # O Python quebra com exceção nestes; o Rust tem de recusar também.
    (json.dumps([]), None),
    (json.dumps(None), None),
    (json.dumps({"projects": "abc"}), None),
    (json.dumps({"projects": [1, 2]}), None),
    (json.dumps({"projects": None}), None),
    (json.dumps({"projects": {"o/a": {}}}), None),
]


def collect(catalog: str | None, manifest: str | None) -> dict[str, Any]:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "docs").mkdir()
        if catalog is not None:
            (root / "docs" / "project-catalog.json").write_text(catalog, encoding="utf-8")
        if manifest is not None:
            (root / "docs" / "README_SITES.json").write_text(manifest, encoding="utf-8")
        try:
            urls = CW.collect_urls(root)
        except Exception as error:  # noqa: BLE001
            return {"crash": type(error).__name__}
    return {"urls": sorted([key, value] for key, value in urls.items())}


# ---------------------------------------------------------------- report

V, U, I = "verified", "unreachable", "invalid"
SCENARIO = {
    "Lucas/Zeta": ("https://zeta.example.org", (V, 200, "https://zeta.example.org")),
    "Lucas/alfa": ("https://alfa.example.org", (V, 200, "https://www.alfa.example.org/")),
    "Lucas/beta": ("https://beta.example.org/x", (U, 404, "https://beta.example.org/x")),
    "Lucas/gama": ("https://gama.example.org", (U, 0, "https://gama.example.org")),
    "Lucas/délta": ("ftp://delta.example.org", (I, 0, "")),
    "Lucas/épsilon": ("", None),
    "Lucas/ζeta": ("https://grego.example.org/ ", (V, 302, "mailto:x@example.org")),
    "Lucas/ctrl": ("https://ctrl.example.org/\x1f\"\\", (V, 1000, "https://ctrl.example.org/\x1f\"\\")),
    "Lucas/mesmo-site": ("https://zeta.example.org", (V, 200, "https://zeta.example.org")),
}
ALL_UP = {repo: value for repo, value in SCENARIO.items() if value[1] and value[1][0] == V}
REPORTS: list[tuple[dict[str, Any], list[str]]] = [
    (SCENARIO, []),
    (SCENARIO, ["--json"]),
    (SCENARIO, ["--fail-on-down"]),
    (SCENARIO, ["--json", "--fail-on-down"]),
    (ALL_UP, ["--fail-on-down"]),
    (ALL_UP, ["--json", "--fail-on-down"]),
    ({}, ["--json", "--fail-on-down"]),
    ({}, []),
]


def report(scenario: dict[str, Any], argv: list[str]) -> dict[str, Any]:
    urls = {repo: url for repo, (url, _) in scenario.items()}
    results = {url: check for url, check in scenario.values() if check is not None}
    original = (CW.collect_urls, CW.check_websites)
    CW.collect_urls = lambda root: dict(urls)
    CW.check_websites = lambda wanted, **_: {
        url: PC.WebsiteCheck(url, *results[url], CHECKED_AT) for url in dict.fromkeys(wanted) if url
    }
    buffer = io.StringIO()
    try:
        with contextlib.redirect_stdout(buffer):
            code = CW.main(argv)
    finally:
        CW.collect_urls, CW.check_websites = original
    return {"stdout": buffer.getvalue(), "exit": code}


# ---------------------------------------------------------------- fixture

def build() -> dict[str, Any]:
    cases: dict[str, list[dict[str, Any]]] = {
        "preflight": [{"input": url, "expected": preflight(url)} for url in PREFLIGHT_URLS],
        "redirect": [
            {"input": {"base": base, "location": location, "uri": uri}, "expected": redirect(base, location, uri)}
            for base, location, uri in REDIRECTS
        ],
        "collect_urls": [
            {"input": {"catalog": catalog, "manifest": manifest}, "expected": collect(catalog, manifest)}
            for catalog, manifest in COLLECT
        ],
        "report": [
            {
                "input": {
                    "argv": argv,
                    "sites": [
                        {"repository": repo, "website": url, "check": None if check is None else
                         {"status": check[0], "http_status": check[1], "final_url": check[2]}}
                        for repo, (url, check) in scenario.items()
                    ],
                    "checked_at": CHECKED_AT,
                },
                "expected": report(scenario, argv),
            }
            for scenario, argv in REPORTS
        ],
    }
    return {
        "schema": "lucas-belucci-bellini/parity-site-monitor@1",
        "note": "Gerado por tests/test_parity_site_monitor.py a partir da biblioteca padrão. Não editar à mão.",
        "python": ".".join(map(str, sys.version_info[:3])),
        "user_agent": UA,
        "cases": cases,
    }


def render(data: dict[str, Any]) -> str:
    return json.dumps(data, ensure_ascii=False, indent=2) + "\n"


class SiteMonitorParityTests(unittest.TestCase):
    def setUp(self) -> None:
        if sys.version_info[:2] != REFERENCE_MINOR:
            self.skipTest(f"fixture do CPython {REFERENCE_MINOR[0]}.{REFERENCE_MINOR[1]} (o do CI)")

    def test_fixture_reflete_o_python_atual(self) -> None:
        data = build()
        if os.environ.get("UPDATE_PARITY") == "1":
            FIXTURE.write_text(render(data), encoding="utf-8")
        recorded = json.loads(FIXTURE.read_text(encoding="utf-8"))
        data["python"] = recorded["python"]  # a versão é informativa; o que conta são os casos
        self.assertEqual(
            FIXTURE.read_text(encoding="utf-8"), render(data),
            f"site_monitor.json não bate com este Python ({sys.version.split()[0]}; o fixture é do "
            f"{recorded['python']}). Se a biblioteca padrão mudou de comportamento, regenere com a versão "
            "do CI e porte a mudança para o Rust no mesmo PR.",
        )

    def test_armadilhas_estao_cobertas(self) -> None:
        cases = build()["cases"]
        # AttributeError/TypeError aqui seria defeito do gerador, não comportamento do Python.
        harness = [c for group in cases.values() for c in group
                   if c["expected"].get("error") in ("AttributeError", "TypeError", "KeyError")]
        self.assertEqual([], harness)
        by_location = {c["input"]["location"]: c["expected"] for c in cases["redirect"]}
        self.assertEqual("not_allowed", by_location["mailto:alguem@example.org"]["kind"])
        self.assertEqual("https://www.example.org/", by_location["https://www.example.org"]["full_url"])
        self.assertTrue(any("crash" in c["expected"] for c in cases["collect_urls"]))
        by_url = {c["input"]: c["expected"] for c in cases["preflight"]}
        self.assertEqual("https://example.org", by_url["https://example.org\n"]["full_url"])
        self.assertFalse(by_url["https://example.org/ção"]["ok"])


if __name__ == "__main__":
    unittest.main()
