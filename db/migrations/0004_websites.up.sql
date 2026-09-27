-- 0004 · Sites e histórico de verificação
--
-- O que muda em relação ao sistema atual: a verificação deixa de sobrescrever
-- o estado anterior. Cada checagem é uma linha append-only em website_checks;
-- "está no ar agora" é uma view sobre a checagem mais recente.
--
-- Descobrir ≠ publicar (docs/WEBSITE-DISCOVERY.md): uma URL existe em
-- `websites` assim que é declarada (homepage do GitHub ou manifesto), mas só
-- vira link público quando a checagem mais recente for `verified`.

CREATE TABLE ecosystem.websites (
  id              bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  project_id      bigint      NOT NULL REFERENCES ecosystem.projects (id),
  -- Mesma expressão de _looks_like_http_url() em project_catalog.py.
  url             text        NOT NULL CHECK (url ~ '^https?://[^\s/$.?#].[^\s]*$'),
  source          text        NOT NULL CHECK (source IN ('github_homepage', 'manifest', 'editorial')),
  is_primary      boolean     NOT NULL DEFAULT true,
  expected_status smallint    CHECK (expected_status BETWEEN 100 AND 599),  -- NULL = qualquer 2xx/3xx após redirecionamento
  declared_at     timestamptz NOT NULL DEFAULT now(),
  retired_at      timestamptz,  -- deixou de ser declarada; o histórico continua
  created_at      timestamptz NOT NULL DEFAULT now(),
  updated_at      timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT websites_project_url_key UNIQUE (project_id, url),
  CONSTRAINT websites_retired_after_declared CHECK (retired_at IS NULL OR retired_at >= declared_at)
);

CREATE UNIQUE INDEX websites_one_active_primary
  ON ecosystem.websites (project_id) WHERE is_primary AND retired_at IS NULL;

CREATE TRIGGER websites_touch BEFORE UPDATE ON ecosystem.websites
  FOR EACH ROW EXECUTE FUNCTION ecosystem.touch_updated_at();

CREATE TABLE ecosystem.website_checks (
  id               bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  website_id       bigint      NOT NULL REFERENCES ecosystem.websites (id),
  sync_run_id      bigint      NOT NULL REFERENCES ecosystem.sync_runs (id),
  checked_at       timestamptz NOT NULL DEFAULT now(),
  outcome          text        NOT NULL CHECK (outcome IN ('verified', 'unreachable', 'invalid')),
  http_status      smallint    CHECK (http_status BETWEEN 100 AND 599),  -- NULL = não houve resposta HTTP (o Python grava 0)
  final_url        text,        -- destino após redirecionamentos
  redirect_count   smallint    NOT NULL DEFAULT 0 CHECK (redirect_count >= 0),
  response_time_ms integer     CHECK (response_time_ms >= 0),
  attempts         smallint    NOT NULL DEFAULT 1 CHECK (attempts >= 1),
  error_kind       text        CHECK (error_kind IN (
                     'timeout', 'dns', 'connect', 'tls', 'http_status', 'invalid_url', 'other')),
  error_message    text,
  CONSTRAINT website_checks_verified_has_http  CHECK (outcome <> 'verified' OR (http_status IS NOT NULL AND error_kind IS NULL)),
  CONSTRAINT website_checks_failure_classified CHECK (outcome = 'verified' OR error_kind IS NOT NULL)
);

COMMENT ON TABLE ecosystem.website_checks IS 'Histórico append-only de verificações HTTP. Nunca sobrescrito.';

CREATE INDEX website_checks_latest_idx ON ecosystem.website_checks (website_id, checked_at DESC, id DESC);
CREATE INDEX website_checks_run_idx ON ecosystem.website_checks (sync_run_id);

CREATE TRIGGER website_checks_append_only BEFORE UPDATE OR DELETE ON ecosystem.website_checks
  FOR EACH ROW EXECUTE FUNCTION ecosystem.forbid_history_rewrite();
CREATE TRIGGER website_checks_no_truncate BEFORE TRUNCATE ON ecosystem.website_checks
  FOR EACH STATEMENT EXECUTE FUNCTION ecosystem.forbid_history_rewrite();

-- Estado atual = checagem mais recente de cada site ativo. Regra de paridade
-- com o Python: no ar se, e somente se, a última checagem foi `verified`.
CREATE VIEW ecosystem.website_status_current AS
SELECT DISTINCT ON (w.id)
  w.id                               AS website_id,
  w.project_id,
  w.url,
  w.source,
  w.is_primary,
  c.id                               AS check_id,
  c.checked_at                       AS last_checked_at,
  c.outcome,
  c.http_status,
  c.final_url,
  c.redirect_count,
  c.response_time_ms,
  coalesce(c.outcome = 'verified', false) AS is_online
FROM ecosystem.websites w
LEFT JOIN ecosystem.website_checks c ON c.website_id = w.id
WHERE w.retired_at IS NULL
ORDER BY w.id, c.checked_at DESC NULLS LAST, c.id DESC;

-- Disponibilidade dos últimos 30 dias: o dado que o sistema atual não tem,
-- porque cada execução apaga a anterior.
CREATE VIEW ecosystem.website_uptime_30d AS
SELECT
  w.id                                             AS website_id,
  w.project_id,
  w.url,
  count(c.id)                                      AS checks,
  count(c.id) FILTER (WHERE c.outcome = 'verified') AS verified_checks,
  round(100.0 * count(c.id) FILTER (WHERE c.outcome = 'verified') / nullif(count(c.id), 0), 2) AS uptime_percent,
  round(avg(c.response_time_ms) FILTER (WHERE c.outcome = 'verified'))                           AS avg_response_time_ms,
  max(c.checked_at) FILTER (WHERE c.outcome = 'verified')                                         AS last_verified_at
FROM ecosystem.websites w
LEFT JOIN ecosystem.website_checks c
  ON c.website_id = w.id AND c.checked_at >= now() - interval '30 days'
WHERE w.retired_at IS NULL
GROUP BY w.id, w.project_id, w.url;
