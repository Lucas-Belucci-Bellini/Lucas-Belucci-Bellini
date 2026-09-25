-- 0005 · Atividade e métricas (reversão)

SELECT ecosystem.assert_data_loss_allowed('ecosystem.commit_observations');
SELECT ecosystem.assert_data_loss_allowed('ecosystem.metric_samples');

DROP VIEW ecosystem.metric_latest;
DROP TABLE ecosystem.metric_samples;
DROP FUNCTION ecosystem.check_metric_sample_scope();
DROP TABLE ecosystem.metric_definitions;
DROP VIEW ecosystem.repository_head_current;
DROP TABLE ecosystem.commit_observations;
