-- Seed de desenvolvimento — dados SINTÉTICOS.
--
-- Nenhum nome, URL ou número real do perfil: domínios `example.org`/`example.com`
-- são reservados (RFC 2606) e os ids do GitHub estão numa faixa fictícia.
-- Cada linha existe para exercitar uma regra que os testes em db/tests/sql
-- conferem:
--
--   aurora-web      público, site no ar hoje (caiu ontem, voltou)     → vitrine com site
--   nimbus-cli      público, sem site                                   → CTA = código
--   old-landing     público e arquivado, site que caiu                  → já não publica site
--   vault-notes     PRIVADO com homepage verificada                     → nunca publica nada além da listagem privada
--   secret-client   público mas EXCLUÍDO                                → some de todas as views
--
-- Uso: psql -v ON_ERROR_STOP=1 -1 -f db/seeds/dev/0001_demo_ecosystem.sql

INSERT INTO ecosystem.sync_runs (kind, trigger, source, status, started_at, finished_at, items_seen, items_changed)
VALUES ('full', 'test', 'seed:dev', 'succeeded', now() - interval '1 minute', now(), 5, 5);

INSERT INTO ecosystem.github_owners (github_id, login, kind) VALUES (900000001, 'demo-owner', 'User');

INSERT INTO ecosystem.repository_exclusions (match_name, reason, source)
VALUES ('secret-client', 'Exemplo de exclusão editorial.', 'seed:dev');

INSERT INTO ecosystem.repositories
  (github_id, owner_id, name, full_name, description, visibility, is_archived, default_branch,
   primary_language, homepage, github_pushed_at, last_synced_at)
SELECT v.github_id, o.id, v.name, 'demo-owner/' || v.name, v.description, v.visibility, v.is_archived, 'main',
       v.primary_language, v.homepage, now() - v.pushed_ago, now()
FROM ecosystem.github_owners o
CROSS JOIN (VALUES
  (900000101, 'aurora-web',    'Painel web de demonstração com site publicado.', 'public',  false, 'TypeScript', 'https://aurora-web.example.org',  interval '3 days'),
  (900000102, 'nimbus-cli',    'Ferramenta de linha de comando sem site.',       'public',  false, 'Rust',       NULL,                              interval '20 days'),
  (900000103, 'old-landing',   'Landing page antiga, arquivada.',                'public',  true,  'HTML',       'https://old-landing.example.org', interval '400 days'),
  (900000104, 'vault-notes',   'Notas privadas.',                                'private', false, 'Python',     'https://vault-notes.example.org', interval '1 day'),
  (900000105, 'secret-client', 'Projeto excluído por decisão editorial.',        'public',  false, 'JavaScript', 'https://secret.example.com',      interval '5 days')
) AS v(github_id, name, description, visibility, is_archived, primary_language, homepage, pushed_ago)
WHERE o.login = 'demo-owner';

INSERT INTO ecosystem.repository_languages (repository_id, language, bytes, sync_run_id)
SELECT r.id, v.language, v.bytes, (SELECT max(id) FROM ecosystem.sync_runs)
FROM ecosystem.repositories r
JOIN (VALUES
  ('aurora-web',    'TypeScript', 60000),
  ('aurora-web',    'CSS',        15000),
  ('nimbus-cli',    'Rust',       25000),
  ('old-landing',   'HTML',       10000),
  ('old-landing',   'CSS',        5000),
  ('vault-notes',   'Python',     999999),
  ('secret-client', 'JavaScript', 888888)
) AS v(repo, language, bytes) ON v.repo = r.name;

INSERT INTO ecosystem.projects (slug, name, summary, label_slug, label_source, classifier_version, publication)
VALUES
  ('aurora-web',    'aurora-web',    'Painel web de demonstração.', 'web',                  'heuristic', 'py-classify@1', 'public'),
  ('nimbus-cli',    'nimbus-cli',    NULL,                          'software-ferramentas', 'heuristic', 'py-classify@1', 'public'),
  ('old-landing',   'old-landing',   NULL,                          'web',                  'editorial', NULL,            'public'),
  ('vault-notes',   'vault-notes',   NULL,                          'ia-automacao',         'heuristic', 'py-classify@1', 'public'),
  ('secret-client', 'secret-client', NULL,                          'web',                  'heuristic', 'py-classify@1', 'public');

INSERT INTO ecosystem.project_repositories (project_id, repository_id, role)
SELECT p.id, r.id, 'primary'
FROM ecosystem.projects p
JOIN ecosystem.repositories r ON r.name = p.slug;

INSERT INTO ecosystem.featured_entries (project_id, position, priority, label, focus, reason)
SELECT id, 1, 90, 'WEB / DEMO', 'Painel web de demonstração.', 'Exemplo de curadoria.'
FROM ecosystem.projects WHERE slug = 'aurora-web';

INSERT INTO ecosystem.websites (project_id, url, source)
SELECT p.id, r.homepage, 'github_homepage'
FROM ecosystem.projects p
JOIN ecosystem.project_repositories pr ON pr.project_id = p.id
JOIN ecosystem.repositories r ON r.id = pr.repository_id
WHERE r.homepage IS NOT NULL;

-- Histórico: aurora caiu ontem e voltou; old-landing estava no ar e caiu;
-- vault-notes responde, mas é privado; secret-client responde, mas é excluído.
INSERT INTO ecosystem.website_checks
  (website_id, sync_run_id, checked_at, outcome, http_status, final_url, response_time_ms, error_kind, error_message)
SELECT w.id, (SELECT max(id) FROM ecosystem.sync_runs), now() - v.ago, v.outcome, v.http_status,
       CASE WHEN v.http_status IS NOT NULL THEN w.url END, v.ms, v.error_kind, v.error_message
FROM ecosystem.websites w
JOIN (VALUES
  ('https://aurora-web.example.org',  interval '1 day',   'unreachable', 503::smallint, 900,  'http_status', 'HTTP 503'),
  ('https://aurora-web.example.org',  interval '1 hour',  'verified',    200::smallint, 180,  NULL,          NULL),
  ('https://old-landing.example.org', interval '40 days', 'verified',    200::smallint, 250,  NULL,          NULL),
  ('https://old-landing.example.org', interval '2 hours', 'unreachable', 404::smallint, 120,  'http_status', 'HTTP 404'),
  ('https://vault-notes.example.org', interval '1 hour',  'verified',    200::smallint, 90,   NULL,          NULL),
  ('https://secret.example.com',      interval '1 hour',  'verified',    200::smallint, 95,   NULL,          NULL)
) AS v(url, ago, outcome, http_status, ms, error_kind, error_message) ON v.url = w.url;

INSERT INTO ecosystem.commit_observations
  (repository_id, sync_run_id, observed_at, branch, head_sha, head_committed_at, head_message, commits_since_previous)
SELECT r.id, (SELECT max(id) FROM ecosystem.sync_runs), now() - v.ago, 'main', v.sha, now() - v.ago, v.message, v.n
FROM ecosystem.repositories r
JOIN (VALUES
  ('aurora-web', interval '2 days', repeat('a', 40), 'feat: primeira versão',      NULL::integer),
  ('aurora-web', interval '3 hours', repeat('b', 40), 'fix: corrige o cabeçalho',  3),
  ('nimbus-cli', interval '20 days', repeat('c', 40), 'chore: versão inicial',     NULL::integer)
) AS v(repo, ago, sha, message, n) ON v.repo = r.name;

INSERT INTO ecosystem.commit_observations (repository_id, sync_run_id, branch, error_stage, error_message)
SELECT r.id, (SELECT max(id) FROM ecosystem.sync_runs), 'main', 'latest_commit', 'HTTP Error 409: Conflict'
FROM ecosystem.repositories r WHERE r.name = 'old-landing';

-- Contribuições mensais (duas coletas do mesmo mês: vale a mais recente) e a
-- carga única dos contadores legados do monitor.
INSERT INTO ecosystem.metric_samples (metric_key, value, window_start, window_end, collected_at, sync_run_id, provenance)
SELECT v.metric_key, v.value, date_trunc('month', now()), now(), now() - v.ago, (SELECT max(id) FROM ecosystem.sync_runs), v.provenance
FROM (VALUES
  ('profile.contributions.total',   40, interval '1 day',  'collected'),
  ('profile.contributions.total',   42, interval '1 hour', 'collected'),
  ('profile.contributions.commits', 30, interval '1 hour', 'collected')
) AS v(metric_key, value, ago, provenance);

INSERT INTO ecosystem.metric_samples (metric_key, value, sync_run_id, provenance)
SELECT v.metric_key, v.value, (SELECT max(id) FROM ecosystem.sync_runs), 'legacy_import'
FROM (VALUES
  ('ecosystem.commits.project_total', 100),
  ('ecosystem.commits.monitor_total', 10),
  ('ecosystem.commits.tracked_total', 110)
) AS v(metric_key, value);

INSERT INTO ecosystem.stack_tools (name, category_slug, family, evidence, position)
VALUES ('Vite', 'frameworks-web', 'build e dev server', 'aurora-web (exemplo)', 1);
