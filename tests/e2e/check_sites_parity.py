#!/usr/bin/env python3
"""Critério de saída da Fase 2: `profile-core check sites` = `check_websites.py`.

Sobe um servidor HTTP local com cenários (redirects de todo tipo, laço, 404,
timeout, conexão recusada, conexão fechada sem resposta, TLS com certificado
não confiável, falha que passa na segunda tentativa), monta uma raiz com
catálogo e manifesto apontando para ele e roda os dois programas sobre ela:

    python3 scripts/check_websites.py   --root R [--json] [--fail-on-down]
    profile-core check sites --no-db    --root R [--json] [--fail-on-down]

Exige: saída de texto idêntica, JSON idêntico exceto `checked_at` (só o
formato é conferido) e o mesmo código de saída.

    python3 tests/e2e/check_sites_parity.py target/release/profile-core

Use o Python do CI (3.12.14): o comportamento do `urllib` é a referência.
"""
from __future__ import annotations

import json
import re
import shutil
import socket
import ssl
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TIMEOUT = 2
SLOW = TIMEOUT * 2
CHECKED_AT = re.compile(r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\+00:00$")


def free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


class ScenarioServer:
    """HTTP/1.1 mínimo, com as respostas escritas byte a byte."""

    def __init__(self, tls_context: ssl.SSLContext | None = None) -> None:
        self.sock = socket.socket()
        self.sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        self.sock.bind(("127.0.0.1", 0))
        self.sock.listen(64)
        self.port = self.sock.getsockname()[1]
        self.tls = tls_context
        self.hits: dict[str, int] = {}
        self.lock = threading.Lock()
        threading.Thread(target=self._serve, daemon=True).start()

    @property
    def base(self) -> str:
        return f"{'https' if self.tls else 'http'}://127.0.0.1:{self.port}"

    def reset(self) -> None:
        with self.lock:
            self.hits.clear()

    def _serve(self) -> None:
        while True:
            conn, _ = self.sock.accept()
            threading.Thread(target=self._handle, args=(conn,), daemon=True).start()

    def _handle(self, conn: socket.socket) -> None:
        try:
            conn.settimeout(15)
            if self.tls:
                try:
                    conn = self.tls.wrap_socket(conn, server_side=True)
                except (ssl.SSLError, OSError):
                    return  # o cliente recusou o certificado: é o cenário
            data = b""
            while b"\r\n\r\n" not in data:
                chunk = conn.recv(4096)
                if not chunk:
                    return
                data += chunk
            target = data.split(b"\r\n", 1)[0].split(b" ")[1].decode("latin-1")
            response = self.respond(target)
            if response is not None:
                conn.sendall(response)
        except (OSError, IndexError):
            pass
        finally:
            try:
                conn.close()
            except OSError:
                pass

    def respond(self, target: str) -> bytes | None:
        path = target.split("?", 1)[0]
        with self.lock:
            self.hits[path] = self.hits.get(path, 0) + 1
            hits = self.hits[path]
        base = f"http://127.0.0.1:{self.port}"

        def reply(status: int, headers: list[tuple[str, bytes]] = (), body: bytes = b"ok") -> bytes:
            head = f"HTTP/1.1 {status} X\r\nContent-Length: {len(body)}\r\nConnection: close\r\n".encode()
            for name, value in headers:
                head += name.encode() + b": " + value + b"\r\n"
            return head + b"\r\n" + body

        def to(location: str | bytes, status: int = 302) -> bytes:
            value = location if isinstance(location, bytes) else location.encode("latin-1")
            return reply(status, [("Location", value)], b"")

        if path == "/" or path.startswith("/ok"):
            return reply(200)
        fixed = {
            "/created": lambda: reply(201),
            "/no-content": lambda: reply(204, body=b""),
            "/404": lambda: reply(404),
            "/500": lambda: reply(500),
            "/503": lambda: reply(503),
            "/300": lambda: reply(300, [("Location", b"/ok")]),
            "/304": lambda: reply(304, body=b""),
            "/r/abs": lambda: to(f"{base}/ok", 301),
            "/r/rel": lambda: to("/ok"),
            "/r/dotdot": lambda: to("../ok"),
            "/r/chain3": lambda: to("/r/chain2"),
            "/r/chain2": lambda: to("/r/chain1", 307),
            "/r/chain1": lambda: to("/ok", 308),
            "/r/303": lambda: to("/ok", 303),
            "/r/to-404": lambda: to("/404", 301),
            "/r/loop": lambda: to("/r/loop"),
            "/r/no-location-302": lambda: reply(302, body=b""),
            "/r/no-location-303": lambda: reply(303, body=b""),
            "/r/mailto": lambda: to("mailto:alguem@example.org", 301),
            "/r/space": lambda: to("/ok?a b"),
            "/r/latin1": lambda: to(b"/ok/\xc3\xa7\xc3\xa3o"),
            "/r/empty": lambda: to(""),
            "/r/refused": lambda: to(f"http://127.0.0.1:{DEAD_PORT}/"),
            "/r/ftp": lambda: to(f"ftp://127.0.0.1:{self.port}/x"),
            "/r/no-path": lambda: to(base, 301),
            "/r/scheme-relative": lambda: to(f"//127.0.0.1:{self.port}/ok"),
            "/r/query-only": lambda: to("?x=1"),
            "/r/upper": lambda: to(f"HTTP://127.0.0.1:{self.port}/ok"),
            "/r/dots": lambda: to("/a/./b/../../ok"),
            "/r/uri-header": lambda: reply(302, [("URI", b"/ok")], b""),
            "/r/fragment": lambda: to("/ok#frag"),
            "/r/tls": lambda: to(f"{TLS_BASE}/ok") if TLS_BASE else reply(404),
            "/flaky": lambda: reply(500) if hits == 1 else reply(200),
            "/close": lambda: None,
        }
        if path == "/r/hub":
            # A mesma URL volta a cada dois saltos enquanto as outras são novas:
            # o ponto em que o urllib desiste (4 repetições) decide o final_url.
            return to(f"/r/spoke/{hits}")
        if path.startswith("/r/spoke/"):
            return to("/r/hub")
        if path.startswith("/r/distinct/"):
            return to(f"/r/distinct/{int(path.rsplit('/', 1)[1]) + 1}")
        if path == "/slow":
            time.sleep(SLOW)
            return reply(200)
        return fixed.get(path, lambda: reply(404))()


def tls_context(workdir: Path) -> ssl.SSLContext | None:
    """Certificado autoassinado: os dois clientes têm de recusar."""
    if not shutil.which("openssl"):
        return None
    cert, key = workdir / "cert.pem", workdir / "key.pem"
    subprocess.run(
        ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1", "-subj", "/CN=127.0.0.1",
         "-keyout", str(key), "-out", str(cert)],
        check=True, capture_output=True,
    )
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(cert, key)
    return context


DEAD_PORT = free_port()  # nada escuta aqui
TLS_BASE = ""


def build_root(root: Path, http: str, tls: str) -> None:
    paths = [
        "", "/", "/ok", "/ok#frag", "/created", "/no-content", "/404", "/500", "/503", "/300", "/304",
        "/r/abs", "/r/rel", "/r/dotdot", "/r/chain3", "/r/303", "/r/to-404", "/r/loop", "/r/distinct/0",
        "/r/no-location-302", "/r/no-location-303", "/r/mailto", "/r/space", "/r/latin1", "/r/empty",
        "/r/refused", "/r/ftp", "/r/no-path", "/r/scheme-relative", "/r/query-only", "/r/upper", "/r/dots",
        "/r/uri-header", "/r/fragment", "/r/tls", "/flaky", "/close", "/slow", "/r/spoke/0",
    ]
    projects = [{"repository": f"e2e/site-{i:02d}", "website_declared": http + p} for i, p in enumerate(paths)]
    projects += [
        {"repository": "e2e/mesma-url", "website": http + "/ok"},
        {"repository": "e2e/invalida", "website": f"ftp://127.0.0.1:{DEAD_PORT}/x"},
        {"repository": "e2e/recusada", "website": f"http://127.0.0.1:{DEAD_PORT}/"},
    ]
    if tls:
        projects.append({"repository": "e2e/tls-autoassinado", "website": tls + "/ok"})
    manifest = {
        "e2e/site-00": "http://nunca.example.org",  # já está no catálogo: o manifesto não sobrescreve
        "e2e/manifesto-vazio": "",
        "e2e/manifesto-quebra-linha": {"website": http + "/ok\n"},
    }
    (root / "docs").mkdir(parents=True)
    (root / "docs" / "project-catalog.json").write_text(json.dumps({"projects": projects}), encoding="utf-8")
    (root / "docs" / "README_SITES.json").write_text(json.dumps(manifest), encoding="utf-8")


def run(command: list[str]) -> tuple[str, int, float]:
    started = time.monotonic()
    done = subprocess.run(command, capture_output=True, text=True, timeout=300)
    if done.returncode not in (0, 1):
        raise SystemExit(f"FALHOU: {' '.join(command)} saiu com {done.returncode}\n{done.stderr}")
    return done.stdout, done.returncode, time.monotonic() - started


def without_checked_at(stdout: str) -> dict:
    report = json.loads(stdout)
    for site in report["sites"]:
        stamp = site.pop("checked_at")
        if stamp and not CHECKED_AT.match(stamp):
            raise SystemExit(f"FALHOU: checked_at fora do formato: {stamp!r}")
    return report


def main() -> int:
    global TLS_BASE
    binary = sys.argv[1] if len(sys.argv) > 1 else str(ROOT / "target" / "release" / "profile-core")
    with tempfile.TemporaryDirectory() as tmp:
        workdir = Path(tmp)
        context = tls_context(workdir)
        tls_server = ScenarioServer(context) if context else None
        TLS_BASE = tls_server.base if tls_server else ""
        server = ScenarioServer()
        build_root(workdir / "root", server.base, TLS_BASE)
        common = ["--root", str(workdir / "root"), "--timeout", str(TIMEOUT), "--retries", "1"]
        python = [sys.executable, str(ROOT / "scripts" / "check_websites.py"), *common]
        rust = [binary, "check", "sites", "--no-db", *common]

        failures = []
        for flags in ([], ["--json"], ["--fail-on-down"], ["--json", "--fail-on-down"]):
            server.reset()
            py_out, py_code, py_time = run(python + flags)
            server.reset()
            rs_out, rs_code, rs_time = run(rust + flags)
            label = " ".join(flags) or "(texto)"
            print(f"  {label:<28} Python {py_time:5.1f}s · Rust {rs_time:5.1f}s · saída {py_code}/{rs_code}")
            if py_code != rs_code:
                failures.append(f"{label}: código de saída {py_code} (Python) × {rs_code} (Rust)")
            if "--json" in flags:
                py_report, rs_report = without_checked_at(py_out), without_checked_at(rs_out)
                if py_report != rs_report:
                    py_sites = {s["repository"]: s for s in py_report["sites"]}
                    for site in rs_report["sites"]:
                        if py_sites.get(site["repository"]) != site:
                            failures.append(f"{label}: {site['repository']}\n    Python: "
                                            f"{py_sites.get(site['repository'])}\n    Rust:   {site}")
                    if py_report.keys() != rs_report.keys() or py_report["checked"] != rs_report["checked"]:
                        failures.append(f"{label}: cabeçalho do relatório difere")
            elif py_out != rs_out:
                failures.append(f"{label}: texto difere\n--- Python\n{py_out}\n--- Rust\n{rs_out}")

        report = without_checked_at(run(python + ["--json"])[0])
        verified = sum(s["status"] == "verified" for s in report["sites"])
        print(f"\n  {report['checked']} sites · {verified} verified · {report['down']} fora · "
              f"TLS {'incluído' if TLS_BASE else 'pulado (sem openssl)'}")
        if failures:
            print("\nFALHOU:\n" + "\n".join(failures), file=sys.stderr)
            return 1
    print("\ncheck sites: paridade Python × Rust ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
