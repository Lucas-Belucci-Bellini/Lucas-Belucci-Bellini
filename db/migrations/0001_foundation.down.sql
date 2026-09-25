-- 0001 · Fundação (reversão)
--
-- Só funciona depois de todas as outras migrations revertidas: DROP SCHEMA sem
-- CASCADE falha se sobrar qualquer objeto — de propósito.

SELECT ecosystem.assert_data_loss_allowed('ecosystem.sync_runs');

DROP TABLE ecosystem.sync_run_errors;
DROP TABLE ecosystem.sync_runs;
DROP FUNCTION ecosystem.forbid_history_rewrite();
DROP FUNCTION ecosystem.assert_data_loss_allowed(regclass);
DROP FUNCTION ecosystem.touch_updated_at();
DROP SCHEMA ecosystem;
