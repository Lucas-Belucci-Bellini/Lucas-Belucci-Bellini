"""Catálogo de projetos: descoberta de site, verificação e resolução editorial.

Este módulo é a fonte única sobre **como um projeto é apresentado**. Ele existe
porque o perfil deixou de ser uma lista de repositórios e virou uma vitrine: o
site do projeto é a experiência, o GitHub é a prova técnica.

Regra central, aplicada em `resolve_presentation`:

    projeto COM site   →  CTA primário = site,   secundário = código
    projeto SEM site   →  CTA primário = código, sem CTA secundário

Sem dependências externas, como o resto do repositório.
"""

from __future__ import annotations

import json
import re
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

# HTTP considerados "no ar". Redirecionamento conta quando o destino final
# responde — é o caso normal de apex → www e de http → https.
LIVE_STATUSES = frozenset({200, 301, 302, 307, 308})

# Taxonomia fechada. Categoria nova exige evidência e entra aqui, não no dado.
CATEGORIES = (
    "AI & Intelligence",
    "Web & SaaS",
    "Games",
    "Infrastructure",
    "Automation",
    "Education",
    "Productivity",
    "Research",
    "Security",
    "Hardware & Simulation",
    "Software & Tools",
    "Experimental",
)

# O README exibe os rótulos em português que o perfil já usava; o catálogo
# legível por máquina usa a taxonomia canônica acima. A ponte fica aqui para
# que exista um lugar só onde as duas listas se encontram.
CATEGORY_ALIASES = {
    "Digital Logic / Hardware": "Hardware & Simulation",
    "Ecossistema Baluarte": "Web & SaaS",
    "Academia": "Education",
    "Games": "Games",
    "IA & Automação": "AI & Intelligence",
    "Infraestrutura / Backend / Dados": "Infrastructure",
    "Web": "Web & SaaS",
    "Experimentos": "Experimental",
    "Software & Ferramentas": "Software & Tools",
}


def canonical_category(label: str) -> str:
    """Traduz o rótulo exibido no README para a taxonomia fechada.

    Rótulo desconhecido cai em `Experimental` em vez de virar categoria nova:
    a taxonomia só cresce por edição deliberada de `CATEGORIES`, nunca por
    um dado de entrada inesperado.
    """
    return CATEGORY_ALIASES.get(label, "Experimental")

USER_AGENT = "profile-readme-website-check/2.0"


# --------------------------------------------------------------- verificação


@dataclass(frozen=True)
class WebsiteCheck:
    """Resultado de uma verificação de site.

    `checked_at` existe no objeto em memória mas **não** é gravado no catálogo
    versionado — ver `catalog_entry`. Um carimbo que muda a cada execução
    criaria commit a cada execução, e a filosofia deste repositório é
    "dado igual → README igual → nenhum commit".
    """

    url: str
    status: str  # "verified" | "unreachable" | "invalid"
    http_status: int
    final_url: str
    checked_at: str

    @property
    def is_live(self) -> bool:
        return self.status == "verified"


def _looks_like_http_url(url: str) -> bool:
    return bool(url) and bool(re.match(r"^https?://[^\s/$.?#].[^\s]*$", url))


def check_website(url: str, *, timeout: float = 15.0, retries: int = 1) -> WebsiteCheck:
    """Verifica uma URL, seguindo redirecionamento.

    `retries` é o número de tentativas **extras** após a primeira. O backoff é
    linear e curto de propósito: o objetivo é tolerar um soluço de rede, não
    insistir num site que está fora.
    """
    now = datetime.now(timezone.utc).isoformat(timespec="seconds")

    if not _looks_like_http_url(url):
        return WebsiteCheck(url, "invalid", 0, "", now)

    last: WebsiteCheck | None = None
    for attempt in range(retries + 1):
        request = Request(url, headers={"User-Agent": USER_AGENT}, method="GET")
        try:
            with urlopen(request, timeout=timeout) as response:
                status = int(response.status)
                final = response.geturl() or url
                ok = status in LIVE_STATUSES or 200 <= status < 400
                return WebsiteCheck(url, "verified" if ok else "unreachable", status, final, now)
        except HTTPError as error:
            # Alguns deploys recusam o método mas ainda revelam um destino útil.
            code = int(error.code)
            final = getattr(error, "url", url) or url
            ok = code in LIVE_STATUSES
            last = WebsiteCheck(url, "verified" if ok else "unreachable", code, final, now)
            if ok:
                return last
        except (URLError, TimeoutError, ValueError, OSError):
            last = WebsiteCheck(url, "unreachable", 0, url, now)
        if attempt < retries:
            # Espera curta entre tentativas; não sobrecarregar o site alheio.
            time.sleep(0.5 * (attempt + 1))

    return last or WebsiteCheck(url, "unreachable", 0, url, now)


def check_websites(
    urls: Iterable[str],
    *,
    max_workers: int = 6,
    timeout: float = 15.0,
    retries: int = 1,
) -> dict[str, WebsiteCheck]:
    """Verifica várias URLs com concorrência limitada.

    O limite existe para não disparar dezenas de requisições simultâneas contra
    hosts que podem ser o mesmo provedor.
    """
    unique = [u for u in dict.fromkeys(urls) if u]
    if not unique:
        return {}
    workers = max(1, min(max_workers, len(unique)))
    with ThreadPoolExecutor(max_workers=workers) as pool:
        results = pool.map(lambda u: check_website(u, timeout=timeout, retries=retries), unique)
        return {check.url: check for check in results}


# ----------------------------------------------------------------- descoberta


def normalize_site_overrides(raw: Any) -> dict[str, dict[str, Any]]:
    """Lê `README_SITES.json` nos dois formatos.

    Formato antigo, ainda aceito:      {"owner/repo": "https://..."}
    Formato novo:                      {"owner/repo": {"website": "https://...", ...}}

    A migração é compatível de propósito: o manifesto é editado à mão pelo
    proprietário do perfil, e quebrá-lo obrigaria a reescrever tudo de uma vez.
    """
    if not isinstance(raw, dict):
        return {}
    out: dict[str, dict[str, Any]] = {}
    for key, value in raw.items():
        if isinstance(value, str):
            out[str(key)] = {"website": value}
        elif isinstance(value, dict) and value.get("website"):
            out[str(key)] = {**value, "website": str(value["website"])}
    return out


def discover_project_website(
    repo: dict[str, Any],
    overrides: dict[str, dict[str, Any]],
) -> tuple[str | None, str]:
    """Descobre a URL do site de um projeto.

    Ordem de precedência, e o motivo de cada degrau:

    1. **homepage declarada no GitHub** — é o próprio dono do repositório
       afirmando qual é o site. Ganha de tudo.
    2. **`docs/README_SITES.json`** — manifesto editorial, para quando a
       homepage não está preenchida no GitHub.
    3. **nada.** Devolve `None`.

    Nunca infere URL a partir do nome do repositório. `meu-projeto` não vira
    `meu-projeto.vercel.app` por dedução: um site que não existe, anunciado
    como se existisse, é pior do que nenhum site.

    Retorna `(url, origem)`, onde origem é `github_homepage`, `manifest` ou
    `none` — a origem é registrada no catálogo para auditoria.
    """
    homepage = str(repo.get("homepage") or "").strip()
    if _looks_like_http_url(homepage):
        return homepage, "github_homepage"

    full_name = str(repo.get("full_name") or "")
    entry = overrides.get(full_name)
    if entry:
        url = str(entry.get("website") or "").strip()
        if _looks_like_http_url(url):
            return url, "manifest"

    return None, "none"


# ------------------------------------------------------------- apresentação


@dataclass
class Presentation:
    """O que a vitrine precisa saber para renderizar um projeto."""

    repository: str
    name: str
    description: str
    category: str  # taxonomia canônica (CATEGORIES)
    category_label: str  # rótulo exibido no README, em português
    website: str | None  # publicado só quando verificado
    website_declared: str | None  # o que foi descoberto, no ar ou não
    website_status: str
    website_http_status: int
    website_source: str
    website_final_url: str  # destino após redirecionamento, para auditoria
    github: str
    primary_cta: str  # "website" | "github"
    secondary_cta: str | None
    status: str
    private: bool
    featured: bool
    marketing_priority: int

    @property
    def has_live_website(self) -> bool:
        return bool(self.website) and self.website_status == "verified"


def marketing_priority(
    repo: dict[str, Any],
    check: WebsiteCheck | None,
    *,
    featured_order: int | None = None,
    description_ok: bool = False,
) -> int:
    """Prioridade **de exibição**, de 0 a 100.

    Isto não é uma nota de qualidade técnica e nunca é mostrado ao visitante.
    É só o critério de ordenação da vitrine, e é explícito para que a ordem
    possa ser discutida em vez de parecer arbitrária.
    """
    score = 0

    # O site é o que o visitante vem ver: pesa mais que qualquer outro fator.
    if check and check.is_live:
        score += 45
    elif check:
        score += 5  # tem site declarado, mas fora do ar

    if featured_order is not None:
        # Curadoria manual vence heurística. Ordem 1 vale mais que ordem 7.
        score += max(0, 25 - (featured_order - 1) * 3)

    if description_ok:
        score += 10

    if not repo.get("private"):
        score += 10

    if not repo.get("fork"):
        score += 5

    if not repo.get("archived"):
        score += 5

    return max(0, min(100, score))


def resolve_presentation(
    repo: dict[str, Any],
    *,
    check: WebsiteCheck | None,
    website: str | None,
    website_source: str,
    category: str,
    status: str,
    description: str,
    featured_order: int | None = None,
) -> Presentation:
    """O `ProjectPresentationResolver`: decide como o projeto aparece.

    A decisão de CTA é a regra central do perfil, e mora só aqui:
    site verificado vira CTA primário; sem ele, o código assume.
    """
    private = bool(repo.get("private"))
    live = bool(check and check.is_live) and bool(website)

    # Projeto privado nunca anuncia site público, mesmo que haja homepage.
    if private:
        live = False
        website = None

    description_ok = len(description.strip()) >= 25 and description.strip() != "—"

    return Presentation(
        repository=str(repo.get("full_name") or ""),
        name=str(repo.get("name") or ""),
        description=description,
        category=canonical_category(category),
        category_label=category,
        website=website if live else None,
        website_declared=(None if private else website),
        website_status=(check.status if check else "none"),
        website_http_status=(check.http_status if check else 0),
        website_source=website_source,
        website_final_url=(check.final_url if check else ""),
        github=f"https://github.com/{repo.get('full_name')}",
        primary_cta="website" if live else "github",
        secondary_cta="github" if live else None,
        status=status,
        private=private,
        featured=featured_order is not None,
        marketing_priority=marketing_priority(
            repo, check, featured_order=featured_order, description_ok=description_ok
        ),
    )


# ---------------------------------------------------------------- catálogo


def catalog_entry(presentation: Presentation) -> dict[str, Any]:
    """Uma linha do `docs/project-catalog.json`.

    **Sem carimbo de tempo, de propósito.** O catálogo é versionado, e um campo
    que muda a cada execução geraria commit a cada execução. A frescura da
    verificação é reportada por `scripts/check_websites.py`, que imprime
    `checked_at` sem gravá-lo no arquivo versionado.
    """
    entry: dict[str, Any] = {
        "repository": presentation.repository,
        "name": presentation.name,
        "description": presentation.description,
        "category": presentation.category,
        "category_label": presentation.category_label,
        "status": presentation.status,
        "github": presentation.github,
        "github_visible": True,
        "github_role": "source" if presentation.website else "primary",
        "primary_cta": presentation.primary_cta,
        "secondary_cta": presentation.secondary_cta,
        "featured": presentation.featured,
        "marketing_priority": presentation.marketing_priority,
        "private": presentation.private,
    }
    # A URL declarada fica registrada mesmo quando está fora do ar: é o que
    # permite a `scripts/check_websites.py` cobrar um deployment caído em vez
    # de ele sumir do radar. `website` (o link que o README publica) continua
    # existindo só quando a verificação passou.
    if presentation.website_declared:
        entry["website_declared"] = presentation.website_declared
        entry["website_status"] = presentation.website_status
        entry["website_http_status"] = presentation.website_http_status
        entry["website_source"] = presentation.website_source
    if presentation.website:
        entry["website"] = presentation.website
        # O link publicado é sempre a URL declarada. O destino do
        # redirecionamento fica registrado só quando difere, para auditoria.
        if presentation.website_final_url and presentation.website_final_url != presentation.website:
            entry["website_final_url"] = presentation.website_final_url
    return entry


def build_catalog(presentations: list[Presentation]) -> dict[str, Any]:
    """Monta o catálogo completo, ordenado de forma determinística.

    A ordenação é por prioridade de exibição decrescente e, em empate, por
    nome — nunca por data. Duas execuções sobre os mesmos dados produzem
    exatamente o mesmo arquivo.
    """
    ordered = sorted(
        presentations,
        key=lambda p: (-p.marketing_priority, p.name.lower()),
    )
    live = [p for p in ordered if p.has_live_website]
    return {
        "schema": "lucas-belucci-bellini/project-catalog@1",
        "note": (
            "Gerado por scripts/update_profile.py. Não editar à mão: ajuste "
            "docs/README_SITES.json, docs/README_FEATURED.json ou a lógica do gerador."
        ),
        "counts": {
            "projects": len(ordered),
            "public": sum(1 for p in ordered if not p.private),
            "private": sum(1 for p in ordered if p.private),
            "with_live_website": len(live),
        },
        "categories": list(CATEGORIES),
        "projects": [catalog_entry(p) for p in ordered],
    }


def write_catalog_if_changed(catalog: dict[str, Any], destination: Path) -> bool:
    """Grava o catálogo só quando o conteúdo muda. Devolve True se gravou."""
    rendered = json.dumps(catalog, ensure_ascii=False, indent=2) + "\n"
    if destination.exists() and destination.read_text(encoding="utf-8") == rendered:
        return False
    destination.write_text(rendered, encoding="utf-8")
    return True


# ------------------------------------------------------------------ render


def cta_cell(presentation: Presentation) -> str:
    """Renderiza a célula de acesso com o site na frente.

    É a tradução visual da regra do perfil. Um projeto com site mostra
    **Abrir site** primeiro e o código depois; sem site, mostra só o código.
    """
    if presentation.website:
        return f"**[▸ Abrir site]({presentation.website})** · [código]({presentation.github})"
    return f"[código]({presentation.github})"
