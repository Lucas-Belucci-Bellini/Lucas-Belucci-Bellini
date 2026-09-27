"""GitHub simulado para testes: rotas fixas, sem rede.

Usado pelos testes Python dos coletores e pelo teste de ponta a ponta
(tests/e2e/github_parity.py), que roda o Python e o núcleo em Rust contra as
**mesmas** respostas. O GitHub de verdade não entra em nenhum teste.

    fake = FakeGitHub()
    fake.get("/users/o/repos?type=owner&per_page=100&page=1", [{"name": "a"}])
    fake.get("/repos/o/a/commits?sha=main&per_page=1", status=409, body={"message": "Git Repository is empty."})
    fake.graphql(lambda variables: {"data": ...})
    with fake:
        os.environ["GITHUB_API_URL"] = fake.url
        ...
    fake.requests  # [("GET", "/users/o/repos?...", "Bearer x"), ...]

Uma rota pode receber uma lista de respostas (`responses=[...]`): cada
requisição consome a próxima, e a última se repete. É o que simula uma falha
que passa na segunda tentativa.
"""
from __future__ import annotations

import json
import threading
from dataclasses import dataclass, field
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Callable


@dataclass
class Response:
    status: int = 200
    body: Any = None
    headers: dict[str, str] = field(default_factory=dict)
    raw: bytes | None = None  # corpo cru, quando o teste quer JSON inválido

    def payload(self) -> bytes:
        if self.raw is not None:
            return self.raw
        return json.dumps(self.body, ensure_ascii=False).encode("utf-8")


class FakeGitHub:
    def __init__(self) -> None:
        self.routes: dict[str, list[Response]] = {}
        self.graphql_handler: Callable[[dict[str, Any]], Response | dict[str, Any]] | None = None
        self.requests: list[tuple[str, str, str | None]] = []
        self._lock = threading.Lock()
        self._server: ThreadingHTTPServer | None = None
        self._thread: threading.Thread | None = None

    # ------------------------------------------------------------ rotas

    def get(self, path: str, body: Any = None, *, status: int = 200, headers: dict[str, str] | None = None,
            raw: bytes | None = None, responses: list[Response] | None = None) -> None:
        """Registra a resposta de um GET. `path` inclui a query, exatamente como o cliente a envia."""
        self.routes[path] = responses or [Response(status, body, headers or {}, raw)]

    def graphql(self, handler: Callable[[dict[str, Any]], Response | dict[str, Any]]) -> None:
        """Responde POST /graphql chamando `handler(variables)`."""
        self.graphql_handler = handler

    def _next(self, path: str) -> Response:
        with self._lock:
            queue = self.routes.get(path)
            if not queue:
                return Response(404, {"message": "Not Found", "documentation_url": "https://docs.github.com/rest"})
            return queue.pop(0) if len(queue) > 1 else queue[0]

    # ------------------------------------------------------------ servidor

    @property
    def url(self) -> str:
        assert self._server is not None, "servidor não iniciado"
        host, port = self._server.server_address[:2]
        return f"http://{host}:{port}"

    def __enter__(self) -> "FakeGitHub":
        fake = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args: Any) -> None:  # silencioso
                pass

            def _send(self, response: Response) -> None:
                payload = response.payload()
                self.send_response(response.status)
                self.send_header("Content-Type", "application/json; charset=utf-8")
                self.send_header("Content-Length", str(len(payload)))
                for key, value in response.headers.items():
                    self.send_header(key, value)
                self.end_headers()
                self.wfile.write(payload)

            def do_GET(self) -> None:
                with fake._lock:
                    fake.requests.append(("GET", self.path, self.headers.get("Authorization")))
                self._send(fake._next(self.path))

            def do_POST(self) -> None:
                length = int(self.headers.get("Content-Length") or 0)
                request = json.loads(self.rfile.read(length) or b"{}")
                with fake._lock:
                    fake.requests.append(("POST", self.path, self.headers.get("Authorization")))
                if self.path != "/graphql" or fake.graphql_handler is None:
                    self._send(Response(404, {"message": "Not Found"}))
                    return
                result = fake.graphql_handler(request.get("variables") or {})
                self._send(result if isinstance(result, Response) else Response(200, result))

        self._server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)
        self._thread.start()
        return self

    def __exit__(self, *exc: Any) -> None:
        if self._server is not None:
            self._server.shutdown()
            self._server.server_close()
        self._server = None
