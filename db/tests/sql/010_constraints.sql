-- Restrições do schema: cada regra de domínio precisa recusar o dado errado.
-- Roda sobre o seed de desenvolvimento e desfaz tudo no fim (ROLLBACK).

BEGIN;

-- ---------------------------------------------------------------- sync_runs
SELECT ecosystem_test.expect_error('sync_run em andamento não tem fim',
  $$INSERT INTO ecosystem.sync_runs (kind, trigger, source, status, finished_at) VALUES ('full', 'test', 't', 'running', now())$$, '23514');
SELECT ecosystem_test.expect_error('sync_run com falha precisa de mensagem',
  $$INSERT INTO ecosystem.sync_runs (kind, trigger, source, status, finished_at) VALUES ('full', 'test', 't', 'failed', now())$$, '23514');
SELECT ecosystem_test.expect_error('sync_run não termina antes de começar',
  $$INSERT INTO ecosystem.sync_runs (kind, trigger, source, status, started_at, finished_at)
    VALUES ('full', 'test', 't', 'succeeded', now(), now() - interval '1 second')$$, '23514');
SELECT ecosystem_test.expect_error('tipo de sync fora da lista',
  $$INSERT INTO ecosystem.sync_runs (kind, trigger, source) VALUES ('whatever', 'test', 't')$$, '23514');

-- ------------------------------------------------------------- repositórios
SELECT ecosystem_test.expect_error('visibilidade fora da lista',
  $$INSERT INTO ecosystem.repositories (github_id, owner_id, name, full_name, visibility)
    SELECT 900009001, id, 'x', 'demo-owner/x', 'secret' FROM ecosystem.github_owners LIMIT 1$$, '23514');
SELECT ecosystem_test.expect_error('full_name precisa terminar no nome',
  $$INSERT INTO ecosystem.repositories (github_id, owner_id, name, full_name, visibility)
    SELECT 900009002, id, 'x', 'demo-owner/y', 'public' FROM ecosystem.github_owners LIMIT 1$$, '23514');
SELECT ecosystem_test.expect_error('github_id é a identidade',
  $$INSERT INTO ecosystem.repositories (github_id, owner_id, name, full_name, visibility)
    SELECT 900000101, id, 'dup', 'demo-owner/dup', 'public' FROM ecosystem.github_owners LIMIT 1$$, '23505');

SET CONSTRAINTS ecosystem.repositories_active_full_name_key IMMEDIATE;
SELECT ecosystem_test.expect_error('nome ativo é único, sem diferenciar maiúsculas',
  $$INSERT INTO ecosystem.repositories (github_id, owner_id, name, full_name, visibility)
    SELECT 900009003, id, 'AURORA-WEB', 'demo-owner/AURORA-WEB', 'public' FROM ecosystem.github_owners LIMIT 1$$, '23505');
SET CONSTRAINTS ecosystem.repositories_active_full_name_key DEFERRED;

-- Repositório que sumiu libera o nome para um novo.
INSERT INTO ecosystem.repositories (github_id, owner_id, name, full_name, visibility, gone_at)
SELECT 900009004, id, 'aurora-web', 'demo-owner/aurora-web', 'public', now() - interval '1 year'
FROM ecosystem.github_owners LIMIT 1;

-- Troca de nomes na mesma transação (renomeia o antigo e cria um novo com o
-- nome dele): só é possível porque a restrição é DEFERRABLE.
INSERT INTO ecosystem.repositories (github_id, owner_id, name, full_name, visibility)
SELECT 900009005, id, 'nimbus-cli', 'demo-owner/nimbus-cli', 'public' FROM ecosystem.github_owners LIMIT 1;
UPDATE ecosystem.repositories SET name = 'nimbus-cli-legacy', full_name = 'demo-owner/nimbus-cli-legacy'
WHERE github_id = 900000102;
SET CONSTRAINTS ecosystem.repositories_active_full_name_key IMMEDIATE;
SET CONSTRAINTS ecosystem.repositories_active_full_name_key DEFERRED;

SELECT ecosystem_test.expect_error('cor de badge precisa ser hex de 6 dígitos',
  $$INSERT INTO ecosystem.languages (name, display_name, badge_color) VALUES ('Zig', 'Zig', 'orange')$$, '23514');
SELECT ecosystem_test.expect_error('bytes de linguagem não são negativos',
  $$INSERT INTO ecosystem.repository_languages (repository_id, language, bytes, sync_run_id)
    SELECT r.id, 'Python', -1, (SELECT max(id) FROM ecosystem.sync_runs) FROM ecosystem.repositories r WHERE r.github_id = 900000101$$, '23514');
SELECT ecosystem_test.expect_error('exclusão é única sem diferenciar maiúsculas',
  $$INSERT INTO ecosystem.repository_exclusions (match_name, reason) VALUES ('SECRET-CLIENT', 'dup')$$, '23505');

-- ---------------------------------------------------------------- projetos
SELECT ecosystem_test.expect_error('slug seguro para rota',
  $$INSERT INTO ecosystem.projects (slug, name) VALUES ('Bad Slug', 'x')$$, '23514');
SELECT ecosystem_test.expect_error('slug único',
  $$INSERT INTO ecosystem.projects (slug, name) VALUES ('aurora-web', 'x')$$, '23505');
SELECT ecosystem_test.expect_error('rótulo precisa existir na taxonomia',
  $$INSERT INTO ecosystem.projects (slug, name, label_slug, label_source) VALUES ('x', 'x', 'nao-existe', 'editorial')$$, '23503');
SELECT ecosystem_test.expect_error('rótulo heurístico registra a versão do classificador',
  $$INSERT INTO ecosystem.projects (slug, name, label_slug, label_source) VALUES ('x', 'x', 'web', 'heuristic')$$, '23514');
SELECT ecosystem_test.expect_error('rótulo editorial não pode ser vazio',
  $$INSERT INTO ecosystem.projects (slug, name, label_source) VALUES ('x', 'x', 'editorial')$$, '23514');
SELECT ecosystem_test.expect_error('publicação fora da lista',
  $$INSERT INTO ecosystem.projects (slug, name, publication) VALUES ('x', 'x', 'secret')$$, '23514');
SELECT ecosystem_test.expect_error('repositório pertence a um projeto só',
  $$INSERT INTO ecosystem.project_repositories (project_id, repository_id, role)
    SELECT (SELECT id FROM ecosystem.projects WHERE slug = 'nimbus-cli'), id, 'component'
    FROM ecosystem.repositories WHERE github_id = 900000101$$, '23505');

INSERT INTO ecosystem.projects (slug, name) VALUES ('multi-repo', 'multi-repo');
INSERT INTO ecosystem.project_repositories (project_id, repository_id, role)
SELECT (SELECT id FROM ecosystem.projects WHERE slug = 'multi-repo'), id, 'primary'
FROM ecosystem.repositories WHERE github_id = 900009004;
SELECT ecosystem_test.expect_error('um primário por projeto',
  $$INSERT INTO ecosystem.project_repositories (project_id, repository_id, role)
    SELECT (SELECT id FROM ecosystem.projects WHERE slug = 'multi-repo'), id, 'primary'
    FROM ecosystem.repositories WHERE github_id = 900009005$$, '23505');

SELECT ecosystem_test.expect_error('prioridade da curadoria vai de 0 a 100',
  $$INSERT INTO ecosystem.featured_entries (project_id, position, priority, label)
    SELECT id, 2, 101, 'X' FROM ecosystem.projects WHERE slug = 'nimbus-cli'$$, '23514');

-- Reordenar a vitrine troca posições dentro da transação.
INSERT INTO ecosystem.featured_entries (project_id, position, label)
SELECT id, 2, 'CLI' FROM ecosystem.projects WHERE slug = 'nimbus-cli';
UPDATE ecosystem.featured_entries SET position = CASE position WHEN 1 THEN 2 ELSE 1 END;
SET CONSTRAINTS ecosystem.featured_entries_position_key IMMEDIATE;
SET CONSTRAINTS ecosystem.featured_entries_position_key DEFERRED;

-- updated_at é do banco, não da aplicação.
UPDATE ecosystem.projects SET name = 'nimbus', updated_at = '2000-01-01' WHERE slug = 'nimbus-cli';
SELECT ecosystem_test.assert_true('touch_updated_at sobrescreve updated_at',
  (SELECT updated_at = now() FROM ecosystem.projects WHERE slug = 'nimbus-cli'));

-- ------------------------------------------------------------------- sites
SELECT ecosystem_test.expect_error('URL precisa ser http(s)',
  $$INSERT INTO ecosystem.websites (project_id, url, source)
    SELECT id, 'ftp://files.example.org', 'editorial' FROM ecosystem.projects WHERE slug = 'nimbus-cli'$$, '23514');
SELECT ecosystem_test.expect_error('origem da URL fora da lista',
  $$INSERT INTO ecosystem.websites (project_id, url, source)
    SELECT id, 'https://nimbus.example.org', 'guessed_from_name' FROM ecosystem.projects WHERE slug = 'nimbus-cli'$$, '23514');
SELECT ecosystem_test.expect_error('um site primário ativo por projeto',
  $$INSERT INTO ecosystem.websites (project_id, url, source)
    SELECT id, 'https://aurora-mirror.example.org', 'editorial' FROM ecosystem.projects WHERE slug = 'aurora-web'$$, '23505');
SELECT ecosystem_test.expect_error('verified exige código HTTP',
  $$INSERT INTO ecosystem.website_checks (website_id, sync_run_id, outcome)
    SELECT id, (SELECT max(id) FROM ecosystem.sync_runs), 'verified' FROM ecosystem.websites LIMIT 1$$, '23514');
SELECT ecosystem_test.expect_error('falha precisa ser classificada',
  $$INSERT INTO ecosystem.website_checks (website_id, sync_run_id, outcome, http_status)
    SELECT id, (SELECT max(id) FROM ecosystem.sync_runs), 'unreachable', 404 FROM ecosystem.websites LIMIT 1$$, '23514');
SELECT ecosystem_test.expect_error('código HTTP plausível',
  $$INSERT INTO ecosystem.website_checks (website_id, sync_run_id, outcome, http_status, error_kind)
    SELECT id, (SELECT max(id) FROM ecosystem.sync_runs), 'unreachable', 0, 'connect' FROM ecosystem.websites LIMIT 1$$, '23514');

-- ---------------------------------------------------------------- atividade
SELECT ecosystem_test.expect_error('observação tem exatamente um estado (commit E erro)',
  $$INSERT INTO ecosystem.commit_observations (repository_id, sync_run_id, branch, head_sha, error_stage, error_message)
    SELECT id, (SELECT max(id) FROM ecosystem.sync_runs), 'main', repeat('d', 40), 'x', 'y'
    FROM ecosystem.repositories WHERE github_id = 900000101$$, '23514');
SELECT ecosystem_test.expect_error('observação tem exatamente um estado (nenhum)',
  $$INSERT INTO ecosystem.commit_observations (repository_id, sync_run_id, branch)
    SELECT id, (SELECT max(id) FROM ecosystem.sync_runs), 'main' FROM ecosystem.repositories WHERE github_id = 900000101$$, '23514');
SELECT ecosystem_test.expect_error('SHA completo em hexadecimal',
  $$INSERT INTO ecosystem.commit_observations (repository_id, sync_run_id, branch, head_sha)
    SELECT id, (SELECT max(id) FROM ecosystem.sync_runs), 'main', 'abc123' FROM ecosystem.repositories WHERE github_id = 900000101$$, '23514');
SELECT ecosystem_test.expect_error('erro de consulta registra a etapa',
  $$INSERT INTO ecosystem.commit_observations (repository_id, sync_run_id, branch, error_message)
    SELECT id, (SELECT max(id) FROM ecosystem.sync_runs), 'main', 'HTTP 409' FROM ecosystem.repositories WHERE github_id = 900000101$$, '23514');
SELECT ecosystem_test.expect_error('métrica de perfil não tem repositório',
  $$INSERT INTO ecosystem.metric_samples (metric_key, repository_id, value, sync_run_id)
    SELECT 'profile.contributions.total', id, 1, (SELECT max(id) FROM ecosystem.sync_runs)
    FROM ecosystem.repositories WHERE github_id = 900000101$$, '23514');
SELECT ecosystem_test.expect_error('métrica desconhecida',
  $$INSERT INTO ecosystem.metric_samples (metric_key, value, sync_run_id)
    VALUES ('profile.stars', 1, (SELECT max(id) FROM ecosystem.sync_runs))$$, '23503');
SELECT ecosystem_test.expect_error('janela da métrica em ordem',
  $$INSERT INTO ecosystem.metric_samples (metric_key, value, window_start, window_end, sync_run_id)
    VALUES ('profile.contributions.total', 1, now(), now() - interval '1 day', (SELECT max(id) FROM ecosystem.sync_runs))$$, '23514');

-- ---------------------------------------------------------------- arsenal
SELECT ecosystem_test.expect_error('ferramenta sem evidência não entra',
  $$INSERT INTO ecosystem.stack_tools (name, category_slug, family, evidence, position)
    VALUES ('Nada', 'frameworks-web', 'x', '   ', 2)$$, '23514');
SELECT ecosystem_test.expect_error('categoria do arsenal precisa existir',
  $$INSERT INTO ecosystem.stack_tools (name, category_slug, family, evidence, position)
    VALUES ('Nada', 'nao-existe', 'x', 'y', 2)$$, '23503');

ROLLBACK;
