-- Histórico é append-only: nenhum caminho sobrescreve uma checagem, uma
-- observação de commit ou uma amostra de métrica sem opt-in explícito.

BEGIN;

SELECT ecosystem_test.expect_error('checagem de site não é editada',
  $$UPDATE ecosystem.website_checks SET outcome = 'verified', http_status = 200, error_kind = NULL$$, 'P0001');
SELECT ecosystem_test.expect_error('checagem de site não é apagada',
  $$DELETE FROM ecosystem.website_checks$$, 'P0001');
SELECT ecosystem_test.expect_error('histórico de sites não é truncado',
  $$TRUNCATE ecosystem.website_checks$$, 'P0001');
SELECT ecosystem_test.expect_error('observação de commit não é editada',
  $$UPDATE ecosystem.commit_observations SET branch = 'x'$$, 'P0001');
SELECT ecosystem_test.expect_error('observação de commit não é apagada',
  $$DELETE FROM ecosystem.commit_observations$$, 'P0001');
SELECT ecosystem_test.expect_error('amostra de métrica não é editada',
  $$UPDATE ecosystem.metric_samples SET value = value + 1$$, 'P0001');
SELECT ecosystem_test.expect_error('amostra de métrica não é truncada',
  $$TRUNCATE ecosystem.metric_samples$$, 'P0001');

-- Com o opt-in (um job de retenção, por exemplo), apagar é possível.
SELECT set_config('ecosystem.allow_data_loss', 'on', true);
DELETE FROM ecosystem.website_checks WHERE checked_at < now() - interval '30 days';
SELECT ecosystem_test.assert_eq('retenção explícita apaga só o que pediu',
  (SELECT count(*)::text FROM ecosystem.website_checks), '5');
SELECT set_config('ecosystem.allow_data_loss', 'off', true);

SELECT ecosystem_test.expect_error('opt-in desligado volta a proteger',
  $$DELETE FROM ecosystem.website_checks$$, 'P0001');

ROLLBACK;
