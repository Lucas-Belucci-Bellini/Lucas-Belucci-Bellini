from __future__ import annotations

import html
import json
import os
import sys
import urllib.error
import urllib.request
from calendar import monthrange
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

LOGIN = os.environ.get("PROFILE_LOGIN", "Lucas-Belucci-Bellini")
ROOT = Path(__file__).resolve().parents[1]
DATA_PATH = ROOT / "docs/assets/contributions-timeline-data.json"
HTML_PATH = ROOT / "docs/assets/contributions-timeline.html"
GRAPHQL_URL = "https://api.github.com/graphql"
QUERY = """
query($login:String!, $from:DateTime!, $to:DateTime!) {
  user(login:$login) {
    contributionsCollection(from:$from, to:$to) {
      contributionCalendar { totalContributions }
      totalCommitContributions
      totalIssueContributions
      totalPullRequestContributions
      totalPullRequestReviewContributions
      totalRepositoryContributions
      restrictedContributionsCount
    }
  }
}
"""


def period_ranges(start: date, end: date) -> list[tuple[date, date]]:
    ranges: list[tuple[date, date]] = []
    cursor = start.replace(day=1)
    while cursor <= end:
        last = date(cursor.year, cursor.month, monthrange(cursor.year, cursor.month)[1])
        ranges.append((max(cursor, start), min(last, end)))
        cursor = last + timedelta(days=1)
    return ranges


def query_period(token: str, start: date, end: date) -> dict[str, int]:
    variables = {
        "login": LOGIN,
        "from": f"{start.isoformat()}T00:00:00Z",
        "to": f"{end.isoformat()}T23:59:59Z",
    }
    payload = json.dumps({"query": QUERY, "variables": variables}).encode("utf-8")
    request = urllib.request.Request(
        GRAPHQL_URL,
        data=payload,
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
            "User-Agent": "Lucas-Belucci-Bellini-contributions-timeline",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            result = json.load(response)
    except urllib.error.HTTPError as error:
        raise RuntimeError(f"GitHub GraphQL HTTP {error.code}") from error
    errors = result.get("errors") or []
    if errors:
        raise RuntimeError("; ".join(str(error.get("message", "GraphQL error")) for error in errors))
    collection = result["data"]["user"]["contributionsCollection"]
    return {
        "total": collection["contributionCalendar"]["totalContributions"],
        "commits": collection["totalCommitContributions"],
        "pull_requests": collection["totalPullRequestContributions"],
        "issues": collection["totalIssueContributions"],
        "reviews": collection["totalPullRequestReviewContributions"],
        "repositories": collection["totalRepositoryContributions"],
        "restricted": collection["restrictedContributionsCount"],
    }


def collect(token: str) -> dict[str, object]:
    today = datetime.now(timezone.utc).date()
    start = today - timedelta(days=365)
    rows: list[dict[str, object]] = []
    for period_start, period_end in period_ranges(start, today):
        values = query_period(token, period_start, period_end)
        values.update({"period_start": period_start.isoformat(), "period_end": period_end.isoformat()})
        rows.append(values)
    return {
        "login": LOGIN,
        "window_start": start.isoformat(),
        "window_end": today.isoformat(),
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "source": "GitHub GraphQL API / contributionsCollection",
        "rows": rows,
    }


def render_html(payload: dict[str, object]) -> str:
    data_json = json.dumps(payload, ensure_ascii=False, separators=(",", ":"))
    login = html.escape(str(payload["login"]))
    generated = html.escape(str(payload["generated_at"]))
    return f'''<!DOCTYPE html>
<html lang="pt-BR">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Baluarte // GitHub Contributions Timeline</title>
  <script src="https://cdn.plot.ly/plotly-2.35.2.min.js"></script>
  <style>
    :root {{ color-scheme: dark; }}
    html, body {{ margin: 0; min-height: 100%; background: #0e0c16; }}
    body {{ color: #f4ecdd; font-family: "DejaVu Sans Mono", "Courier New", monospace; }}
    main {{ max-width: 1440px; margin: 0 auto; padding: 28px 22px 40px; }}
    h1 {{ margin: 0; color: #e8c07a; font-size: clamp(22px, 3vw, 34px); letter-spacing: 1px; }}
    .meta {{ color: #a89a80; font-size: 12px; line-height: 1.6; margin-top: 8px; }}
    #chart {{ width: 100%; min-height: 760px; margin-top: 22px; }}
    .fallback {{ color: #a89a80; font-family: Arial, sans-serif; line-height: 1.5; margin-top: 22px; }}
    code {{ color: #3ddc84; }}
  </style>
</head>
<body>
  <main>
    <h1>BALUARTE // GITHUB CONTRIBUTIONS TIMELINE</h1>
    <div class="meta">Perfil: <code>{login}</code> · Janela móvel de 365 dias · Snapshot: <code>{generated}</code></div>
    <div id="chart" aria-label="Timeline interativa de contribuições do GitHub"></div>
    <noscript><p class="fallback">Ative JavaScript para visualizar o gráfico interativo. O snapshot bruto está disponível em <code>contributions-timeline-data.json</code>.</p></noscript>
  </main>
  <script>
    const payload = {data_json};
    const months = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];
    const rows = payload.rows;
    const labels = rows.map(row => {{ const d = row.period_start.split("-"); return `${{months[Number(d[1]) - 1]}}/${{d[0]}}`; }});
    const totals = rows.map(row => row.total);
    const commits = rows.map(row => row.commits);
    const prs = rows.map(row => row.pull_requests);
    const issues = rows.map(row => row.issues);
    const reviews = rows.map(row => row.reviews);
    const repos = rows.map(row => row.repositories);
    const others = totals.map((total, index) => Math.max(0, total - commits[index] - prs[index] - issues[index]));
    const colors = {{ commits: "#d4a24e", prs: "#3ddc84", issues: "#a68dad", reviews: "#e8c07a", repos: "#8fa6c4", total: "#f4ecdd" }};
    const trace = (values, name, color) => ({{ type: "bar", x: labels, y: values, name, marker: {{ color }}, hovertemplate: `<b>${{name}}</b>: %{{y}}<extra></extra>` }});
    const pct = (values) => values.map((value, index) => totals[index] ? 100 * value / totals[index] : 0);
    const traces = [trace(commits, "Commits", colors.commits), trace(prs, "Pull requests", colors.prs), trace(issues, "Issues", colors.issues), trace(reviews, "Reviews", colors.reviews), trace(repos, "Repositórios", colors.repos), {{ type: "scatter", x: labels, y: totals, name: "Contribuições totais", mode: "lines+markers", line: {{ color: colors.total, width: 3 }}, marker: {{ color: colors.total, size: 8 }}, hovertemplate: "<b>Total</b>: %{{y}}<extra></extra>" }}];
    const shareTraces = [{{ type: "scatter", x: labels, y: pct(commits), name: "% Commits", mode: "lines+markers", line: {{ color: colors.commits, width: 2 }} }}, {{ type: "scatter", x: labels, y: pct(prs), name: "% PRs", mode: "lines+markers", line: {{ color: colors.prs, width: 2 }} }}, {{ type: "scatter", x: labels, y: pct(issues), name: "% Issues", mode: "lines+markers", line: {{ color: colors.issues, width: 2 }} }}, {{ type: "scatter", x: labels, y: pct(others), name: "% Outras contribuições", mode: "lines+markers", line: {{ color: colors.reviews, width: 2 }} }}];
    const layout = {{ template: "plotly_dark", paper_bgcolor: "#0e0c16", plot_bgcolor: "#0e0c16", font: {{ family: "DejaVu Sans Mono, monospace", color: "#f4ecdd" }}, barmode: "stack", hovermode: "x unified", height: 820, margin: {{ l: 68, r: 24, t: 70, b: 90 }}, legend: {{ orientation: "h", y: 1.04, x: 0 }}, xaxis: {{ showgrid: false, rangeslider: {{ visible: true, thickness: 0.08 }}, title: "Passe o cursor para detalhar; use o slider para ampliar a janela" }}, yaxis: {{ title: "Quantidade", gridcolor: "#3b3047" }}, annotations: [{{ text: "Volume mensal e composição", x: 0.5, y: 1.11, xref: "paper", yref: "paper", showarrow: false, font: {{ color: "#e8c07a", size: 15 }} }}] }};
    Plotly.newPlot("chart", traces, layout, {{ responsive: true, displaylogo: false }});
    const shareStart = traces.length;
    const shareLayout = {{ ...layout, barmode: "overlay", yaxis: {{ title: "Participação (%)", range: [0, 100], ticksuffix: "%", gridcolor: "#3b3047" }}, annotations: [{{ text: "Proporção por tipo", x: 0.5, y: 1.11, xref: "paper", yref: "paper", showarrow: false, font: {{ color: "#e8c07a", size: 15 }} }}] }};
    const divider = document.createElement("div"); divider.id = "share"; divider.style.width = "100%"; divider.style.minHeight = "500px"; divider.style.marginTop = "24px"; document.querySelector("main").appendChild(divider);
    Plotly.newPlot("share", shareTraces, shareLayout, {{ responsive: true, displaylogo: false }});
  </script>
</body>
</html>
'''


def main() -> int:
    token = os.environ.get("PROFILE_README_TOKEN") or os.environ.get("GITHUB_TOKEN")
    if not token:
        print("PROFILE_README_TOKEN ou GITHUB_TOKEN ausente", file=sys.stderr)
        return 2
    try:
        payload = collect(token)
    except Exception as error:  # noqa: BLE001
        print(f"timeline generation failed: {error}", file=sys.stderr)
        return 1
    DATA_PATH.parent.mkdir(parents=True, exist_ok=True)
    DATA_PATH.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    HTML_PATH.write_text(render_html(payload), encoding="utf-8")
    print(f"timeline updated: rows={len(payload['rows'])} html={HTML_PATH.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
