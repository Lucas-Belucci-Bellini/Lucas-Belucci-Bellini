-- 0005 · Atividade e métricas
--
-- Duas famílias de dado que hoje vivem em JSON versionado:
--
-- * commit_observations — o que .github/scripts/ecosystem_watch.py grava em
--   docs/ECOSYSTEM-COMMIT-STATE.json: último commit do branch padrão de cada
--   repositório e quantos commits entraram desde a observação anterior.
--   Uma linha por MUDANÇA de estado (não por varredura): a varredura em si já
--   está em sync_runs. Mesma regra "sem mudança semântica, nada gravado".
--
-- * metric_samples — séries numéricas com janela e proveniência: as
--   contribuições mensais do GraphQL (update_contribution_timeline.py e
--   profile_cards.py) e os contadores cumulativos do monitor, que NÃO podem
--   ser recalculados a partir do GitHub (partem de LEGACY_BASELINE = 1538) e
--   por isso entram como `legacy_import` na primeira carga.

CREATE TABLE ecosystem.commit_observations (
  id                     bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  repository_id          bigint      NOT NULL REFERENCES ecosystem.repositories (id),
  sync_run_id            bigint      NOT NULL REFERENCES ecosystem.sync_runs (id),
  observed_at            timestamptz NOT NULL DEFAULT now(),
  branch                 text        NOT NULL,
  head_sha               text        CHECK (head_sha ~ '^[0-9a-f]{40}$'),
  head_committed_at      timestamptz,
  head_message           text,       -- primeira linha, até 140 caracteres (paridade)
  head_url               text,
  -- NULL = não determinado (primeira observação, ou a comparação falhou —
  -- "quantidade não determinada" no relatório atual).
  commits_since_previous integer     CHECK (commits_since_previous >= 0),
  is_empty               boolean     NOT NULL DEFAULT false,
  error_stage            text,
  error_message          text,
  -- Exatamente um estado: um commit, repositório vazio, ou erro de consulta.
  CONSTRAINT commit_observations_one_state
    CHECK (num_nonnulls(head_sha, nullif(is_empty, false), error_message) = 1),
  CONSTRAINT commit_observations_error_has_stage
    CHECK ((error_message IS NULL) = (error_stage IS NULL))
);

CREATE INDEX commit_observations_latest_idx
  ON ecosystem.commit_observations (repository_id, observed_at DESC, id DESC);

CREATE TRIGGER commit_observations_append_only BEFORE UPDATE OR DELETE ON ecosystem.commit_observations
  FOR EACH ROW EXECUTE FUNCTION ecosystem.forbid_history_rewrite();
CREATE TRIGGER commit_observations_no_truncate BEFORE TRUNCATE ON ecosystem.commit_observations
  FOR EACH STATEMENT EXECUTE FUNCTION ecosystem.forbid_history_rewrite();

CREATE VIEW ecosystem.repository_head_current AS
SELECT DISTINCT ON (o.repository_id)
  o.repository_id,
  o.id AS observation_id,
  o.observed_at,
  o.branch,
  o.head_sha,
  o.head_committed_at,
  o.head_message,
  o.head_url,
  o.is_empty,
  o.error_stage,
  o.error_message
FROM ecosystem.commit_observations o
ORDER BY o.repository_id, o.observed_at DESC, o.id DESC;

CREATE TABLE ecosystem.metric_definitions (
  key         text PRIMARY KEY CHECK (key ~ '^[a-z0-9_]+(\.[a-z0-9_]+)*$'),
  scope       text NOT NULL CHECK (scope IN ('profile', 'ecosystem', 'repository', 'project')),
  unit        text NOT NULL CHECK (unit IN ('count', 'bytes', 'milliseconds', 'percent')),
  description text NOT NULL,
  source      text NOT NULL
);

INSERT INTO ecosystem.metric_definitions (key, scope, unit, description, source) VALUES
  ('profile.contributions.total',         'profile',   'count', 'Total do calendário de contribuições na janela.',                  'github_graphql:contributionsCollection.contributionCalendar'),
  ('profile.contributions.commits',       'profile',   'count', 'Commits atribuídos ao perfil na janela.',                          'github_graphql:contributionsCollection.totalCommitContributions'),
  ('profile.contributions.pull_requests', 'profile',   'count', 'Pull requests atribuídos ao perfil na janela.',                    'github_graphql:contributionsCollection.totalPullRequestContributions'),
  ('profile.contributions.issues',        'profile',   'count', 'Issues atribuídas ao perfil na janela.',                           'github_graphql:contributionsCollection.totalIssueContributions'),
  ('profile.contributions.reviews',       'profile',   'count', 'Reviews de pull request na janela.',                               'github_graphql:contributionsCollection.totalPullRequestReviewContributions'),
  ('profile.contributions.repositories',  'profile',   'count', 'Contribuições de criação de repositório na janela.',               'github_graphql:contributionsCollection.totalRepositoryContributions'),
  ('profile.contributions.restricted',    'profile',   'count', 'Contribuições privadas contadas sem detalhe.',                     'github_graphql:contributionsCollection.restrictedContributionsCount'),
  ('ecosystem.commits.project_total',     'ecosystem', 'count', 'Contador cumulativo de commits dos projetos (monitor do ecossistema).', 'ecosystem_watch'),
  ('ecosystem.commits.monitor_total',     'ecosystem', 'count', 'Snapshots publicados pelo próprio monitor.',                       'ecosystem_watch'),
  ('ecosystem.commits.tracked_total',     'ecosystem', 'count', 'project_total + monitor_total. Não é GitHub Contributions.',        'ecosystem_watch'),
  ('ecosystem.languages.public_bytes',    'ecosystem', 'bytes', 'Bytes por linguagem nos repositórios públicos (dimensão = linguagem).', 'github_rest:/repos/{repo}/languages');

CREATE TABLE ecosystem.metric_samples (
  id            bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  metric_key    text        NOT NULL REFERENCES ecosystem.metric_definitions (key),
  repository_id bigint      REFERENCES ecosystem.repositories (id),
  project_id    bigint      REFERENCES ecosystem.projects (id),
  dimension     text        NOT NULL DEFAULT '',  -- ex.: nome da linguagem; '' = sem dimensão
  value         numeric     NOT NULL,
  window_start  timestamptz,
  window_end    timestamptz,
  collected_at  timestamptz NOT NULL DEFAULT now(),
  sync_run_id   bigint      NOT NULL REFERENCES ecosystem.sync_runs (id),
  provenance    text        NOT NULL DEFAULT 'collected'
                  CHECK (provenance IN ('collected', 'derived', 'legacy_import')),
  CONSTRAINT metric_samples_window_order CHECK (window_end IS NULL OR window_start IS NULL OR window_end >= window_start),
  CONSTRAINT metric_samples_single_subject CHECK (num_nonnulls(repository_id, project_id) <= 1)
);

CREATE INDEX metric_samples_series_idx
  ON ecosystem.metric_samples (metric_key, repository_id, project_id, dimension, window_start, collected_at DESC);

-- O escopo da definição manda no sujeito da amostra: métrica de repositório
-- exige repository_id, de projeto exige project_id, e as demais não têm sujeito.
CREATE FUNCTION ecosystem.check_metric_sample_scope() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
  metric_scope text;
BEGIN
  SELECT scope INTO metric_scope FROM ecosystem.metric_definitions WHERE key = NEW.metric_key;
  IF metric_scope = 'repository' AND (NEW.repository_id IS NULL OR NEW.project_id IS NOT NULL)
     OR metric_scope = 'project' AND (NEW.project_id IS NULL OR NEW.repository_id IS NOT NULL)
     OR metric_scope IN ('profile', 'ecosystem') AND num_nonnulls(NEW.repository_id, NEW.project_id) > 0 THEN
    RAISE EXCEPTION USING
      ERRCODE = 'check_violation',
      MESSAGE = format('metric %s has scope %s; subject columns do not match', NEW.metric_key, metric_scope);
  END IF;
  RETURN NEW;
END
$$;

CREATE TRIGGER metric_samples_scope BEFORE INSERT ON ecosystem.metric_samples
  FOR EACH ROW EXECUTE FUNCTION ecosystem.check_metric_sample_scope();
CREATE TRIGGER metric_samples_append_only BEFORE UPDATE OR DELETE ON ecosystem.metric_samples
  FOR EACH ROW EXECUTE FUNCTION ecosystem.forbid_history_rewrite();
CREATE TRIGGER metric_samples_no_truncate BEFORE TRUNCATE ON ecosystem.metric_samples
  FOR EACH STATEMENT EXECUTE FUNCTION ecosystem.forbid_history_rewrite();

-- Valor mais recente de cada série (métrica × sujeito × dimensão × janela).
CREATE VIEW ecosystem.metric_latest AS
SELECT DISTINCT ON (s.metric_key, s.repository_id, s.project_id, s.dimension, s.window_start, s.window_end)
  s.metric_key,
  s.repository_id,
  s.project_id,
  s.dimension,
  s.window_start,
  s.window_end,
  s.value,
  s.collected_at,
  s.provenance,
  s.sync_run_id
FROM ecosystem.metric_samples s
ORDER BY s.metric_key, s.repository_id, s.project_id, s.dimension, s.window_start, s.window_end,
         s.collected_at DESC, s.id DESC;
