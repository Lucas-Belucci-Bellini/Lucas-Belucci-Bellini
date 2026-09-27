"""Semântica de texto do CPython que os validadores expõem, caso a caso.

    OLD PYTHON ──▶ tests/fixtures/parity/pytext.json ◀── NEW RUST

Os validadores (`scripts/validate_*.py`) imprimem o que a biblioteca padrão
diz: a mensagem do `json.JSONDecodeError` com linha, coluna e posição em code
points; o `str()` de um `OSError` e de um `UnicodeDecodeError`; e o `repr()` de
valores lidos de JSON (aspas escolhidas pelo conteúdo, escapes pelo
`str.isprintable()` do Unicode do próprio Python). O `ecosystem-domain`
reproduz os quatro (`pyjson::loads`, `pyio`, `pyrepr`), e este fixture é a
prova:

    cargo test -p ecosystem-domain --test pytext_parity

O que o Python aceita e um `serde_json::Value` não representa — `NaN`,
`Infinity`, inteiro fora de 64 bits, float que estoura para `inf`, surrogate
isolado — fica marcado como `unsupported`: o Rust recusa em vez de inventar
um valor (D-037).

**Versão:** mensagens e tabela Unicode mudam entre versões (o 3.13 trocou a
mensagem da vírgula final); o teste só compara no CPython do CI.

    UPDATE_PARITY=1 python3.12 -m unittest tests.test_parity_pytext
"""
from __future__ import annotations

import json
import math
import os
import sys
import unittest
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests" / "fixtures" / "parity" / "pytext.json"
TABLE = ROOT / "crates" / "ecosystem-domain" / "src" / "printable.rs"
REFERENCE_MINOR = (3, 12)

VALID_DOCS = [
    '{"a": [1, -2.5e3, "x\\n\\u00e9\\ud83d\\ude00", true, null], "b": {"c": false}}',
    '[0, -0, 0.5, 1E+2, "ç\\"\\\\\\/\\b\\f\\r\\t", {}, [], ""]',
]

JSON_TEXTS = [
    "", " ", "\t\n\r ", "{", '{"a"', '{"a":', '{"a":}', '{"a":1,}', "[1,]", "[1 2]", '{"a" 1}', "{1:2}",
    '"abc', '"a\\x"', '"\\u12"', '"\\u12', '"\\u123"', '"\x01"', '"\x1f"', '"\x7f"', '"\\u0000"', "nul", "null",
    "true", "false", "tru", "fals", "NaN", "Infinity", "-Infinity", "-Infinit", "Na", "-", "-a", "01", "-01", "1.",
    "1.e5", "1e", "1e+", "1e5", "1E-2", "-0", "-0.0", "0.1", "1.5", "2.5e-3", "[1e5x]", "\ufeff{}", "{} x", "[] ]",
    '"\\ud800"', '"\\udc00"', '"\\ud83d\\ude00"', '"\\ud83d\\u0041"', '"\\ud83d\\u004"', '"\\ud83d\\uZZZZ"',
    '"\\ud83d\\ude00', '"\\ud83dab"', '"\\u00e9\\u12G4"', '"a\\', '"a\\"', "[", "[,", '{"a":[}', "{,}", "[,]",
    "1" * 30, "123456789012345678", "9223372036854775807", "9223372036854775808", "18446744073709551615",
    "18446744073709551616", "-9223372036854775808", "-9223372036854775809", "1e999", "-1e999", "1e308",
    "4.9e-324", "1e-400", '{"a":1,"a":2,"b":3}', '{"b":1,"a":2,"b":3}', "\n\n  [\n x]", '{\n"ç": ,}',
    '["ção", "中文", "😀"] x', '\u00a0 1', '{"a" : 1 , "b" : [ ] }', '{"":{}}', "1 \n", "[" * 40 + "]" * 40,
    "[" * 5 + "]" * 4, '{"x": "tab\there"}', '"\\/"', '"\\a"', '"\\U0041"', "'a'", '{"a": 01}', "[.5]", "[+1]",
    "[1,,2]", '{"a":1 "b":2}', '{"a":1}}', "[true false]", '"\u2028\u2029"', '"\\u2028"',
]

REPR_TEXTS = [
    '"it\'s"', '"a\\"b"', '"a\'b\\"c"', '"\\\\"', '"\\u007f\\u0085\\u00a0\\u200b😀\\ue000\\u0378"', '"tab\\tnl\\nr\\r"',
    '"\\u0000\\u0001\\u001f"', '"ção 中 ﷽"', '"\\u2028\\u2029\\u3000\\u1680"', '"\\udb40\\udc01 \\u00ad \\ufeff"',
    '"e\\u0301 \\u0903"', '"\\ud83c\\udff4\\udb40\\udc67"', '"\\uffff\\ufffe\\ufffd"', '"\\udbff\\udffd"', '""', '"\'"',
    '"\\""', '"\'\\""', "null", "true", "false", "0", "-7", "123456789012345678",
    "[0.0, -0.0, 1.0, 1e16, 1e15, 1e-5, 1e-4, 0.1, 1.5e300, 5e-324, 1.7976931348623157e308, 123456789.123]",
    "[2.5, 1e22, 1e21, 9007199254740993.0, 100.0, 1e17, 12345678901234567.0, 0.000123, 3.14159]",
    "[]", "{}", '[1, "a", [2, {"k": null}], {"x": [true]}]', '{"b": 1, "a": {"c": "d"}}',
    '{"it\'s": "a\\"b", "n": [1.0]}',
]

UTF8_BYTES = [
    b"", b"abc", "ção".encode(), b"\xff", b"ab\xe2\x82x", b"\xe2x", b"\xed\xa0\x80", b"abc\xe2\x82", b"\xc3",
    b"\xe0\x80\x80", b"\xf4\x90\x80\x80", b"\xf0\x9f\x98", b"\x80", b"\xc0\xaf", b"\xc1\xbf", b"\xf5\x80",
    b"a\xf0\x9f\x98\x80b\xfe", b"\xe2\x82\xac\xe2", b"\xf0\x80\x80\x80", b"\xf0\x9f", b"\xef\xbb\xbfx",
]

OS_ERRORS = [
    (2, "/raiz/docs/README_SITES.json"), (21, "/raiz/docs"), (13, "/raiz/README.md"), (20, "/raiz/README.md/x"),
    (2, "/raiz/it's"), (2, "/raiz/a\"b'c"), (2, "/raiz/ção\n"),
]


def mutations(doc: str) -> list[str]:
    """Todos os prefixos e uma troca de caractere por posição: erros em todo lugar."""
    out = [doc[:n] for n in range(len(doc))]
    swaps = ",:]}\"x "
    out += [doc[:n] + swaps[n % len(swaps)] + doc[n + 1:] for n in range(len(doc))]
    return out


def unsupported(value: Any) -> bool:
    """O que um serde_json::Value não representa."""
    if isinstance(value, float):
        return not math.isfinite(value)
    if isinstance(value, bool) or value is None:
        return False
    if isinstance(value, int):
        return not (-(2 ** 63) <= value < 2 ** 64)
    if isinstance(value, str):
        return any(0xD800 <= ord(c) <= 0xDFFF for c in value)
    if isinstance(value, list):
        return any(unsupported(v) for v in value)
    if isinstance(value, dict):
        return any(unsupported(k) or unsupported(v) for k, v in value.items())
    raise TypeError(type(value))


def loads_case(text: str) -> dict[str, Any]:
    try:
        value = json.loads(text)
    except json.JSONDecodeError as error:
        return {"input": text, "error": str(error)}
    except RecursionError:
        return {"input": text, "unsupported": True}
    if unsupported(value):
        return {"input": text, "unsupported": True}
    return {"input": text, "repr": repr(value)}


def utf8_case(data: bytes) -> dict[str, Any]:
    try:
        return {"input": data.hex(), "text": data.decode("utf-8")}
    except UnicodeDecodeError as error:
        return {"input": data.hex(), "error": str(error)}


def os_case(errno: int, path: str) -> dict[str, Any]:
    error = OSError(errno, os.strerror(errno), path)
    return {"errno": errno, "path": path, "kind": type(error).__name__, "message": str(error)}


def nonprintable_ranges() -> list[list[int]]:
    ranges: list[list[int]] = []
    for code in range(0x110000):
        if chr(code).isprintable():
            continue
        if ranges and ranges[-1][1] == code - 1:
            ranges[-1][1] = code
        else:
            ranges.append([code, code])
    return ranges


def rust_table(ranges: list[list[int]]) -> str:
    rows = "\n".join(f"    (0x{a:04X}, 0x{b:04X})," for a, b in ranges)
    return (
        "//! Gerado por `tests/test_parity_pytext.py` a partir do `str.isprintable()` do\n"
        f"//! CPython {sys.version.split()[0]}. Não editar à mão: `UPDATE_PARITY=1` regenera.\n\n"
        "/// Intervalos fechados de code points que o Python **não** considera\n"
        "/// imprimíveis (Cc, Cf, Cs, Co, Cn, Zl, Zp e Zs exceto o espaço).\n"
        f"pub(crate) const NONPRINTABLE: [(u32, u32); {len(ranges)}] = [\n{rows}\n];\n"
    )


def build() -> dict[str, Any]:
    texts = list(dict.fromkeys(JSON_TEXTS + [t for doc in VALID_DOCS for t in [doc, *mutations(doc)]]))
    return {
        "schema": "lucas-belucci-bellini/parity-pytext@1",
        "note": "Gerado por tests/test_parity_pytext.py a partir da biblioteca padrão do CPython. Não editar à mão.",
        "python": sys.version.split()[0],
        "json_loads": [loads_case(text) for text in texts],
        "repr": [{"input": text, "repr": repr(json.loads(text)), "str": str(json.loads(text))} for text in REPR_TEXTS],
        "utf8": [utf8_case(data) for data in UTF8_BYTES],
        "os_error": [os_case(errno, path) for errno, path in OS_ERRORS],
        "nonprintable": nonprintable_ranges(),
    }


def render(data: dict[str, Any]) -> str:
    return json.dumps(data, ensure_ascii=False, indent=1) + "\n"


class PyTextParityTests(unittest.TestCase):
    data: dict[str, Any]

    @classmethod
    def setUpClass(cls) -> None:
        if sys.version_info[:2] != REFERENCE_MINOR:
            raise unittest.SkipTest(f"fixture do CPython {REFERENCE_MINOR[0]}.{REFERENCE_MINOR[1]} (o do CI)")
        cls.data = build()
        if os.environ.get("UPDATE_PARITY") == "1":
            FIXTURE.write_text(render(cls.data), encoding="utf-8")
            TABLE.write_text(rust_table(cls.data["nonprintable"]), encoding="utf-8")

    def test_fixture_reflete_o_python_atual(self) -> None:
        recorded = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(FIXTURE.read_text(encoding="utf-8"), render(dict(self.data, python=recorded["python"])),
                         "pytext.json não bate com o CPython atual; regenere com a versão do CI")

    def test_tabela_do_rust_e_a_do_fixture(self) -> None:
        table = TABLE.read_text(encoding="utf-8")
        expected = rust_table(self.data["nonprintable"])
        self.assertEqual(expected.split("\n", 2)[2], table.split("\n", 2)[2],
                         "crates/ecosystem-domain/src/printable.rs não é a tabela deste Python")

    def test_armadilhas_estao_cobertas(self) -> None:
        errors = {case["input"]: case.get("error") for case in self.data["json_loads"]}
        self.assertEqual("Expecting property name enclosed in double quotes: line 1 column 8 (char 7)",
                         errors['{"a":1,}'], "vírgula final (o 3.13 mudou esta mensagem)")
        self.assertEqual("Invalid \\uXXXX escape: line 1 column 9 (char 8)", errors['"\\ud83d\\ude00'],
                         "par de surrogates sem aspas de fecho não vira 'Unterminated'")
        self.assertEqual("Expecting value: line 4 column 2 (char 7)", errors["\n\n  [\n x]"])
        self.assertTrue(any(case.get("unsupported") for case in self.data["json_loads"]))
        reprs = {case["input"]: case["repr"] for case in self.data["repr"]}
        self.assertEqual('"it\'s"', reprs['"it\'s"'])
        self.assertIn("1e+16", reprs[REPR_TEXTS[24]])


if __name__ == "__main__":
    unittest.main()
