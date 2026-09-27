-- 0004 · Sites e histórico de verificação (reversão)

SELECT ecosystem.assert_data_loss_allowed('ecosystem.website_checks');
SELECT ecosystem.assert_data_loss_allowed('ecosystem.websites');

DROP VIEW ecosystem.website_uptime_30d;
DROP VIEW ecosystem.website_status_current;
DROP TABLE ecosystem.website_checks;
DROP TABLE ecosystem.websites;
