-- 0002 · Repositórios (reversão)

SELECT ecosystem.assert_data_loss_allowed('ecosystem.repositories');
SELECT ecosystem.assert_data_loss_allowed('ecosystem.repository_exclusions');

DROP TABLE ecosystem.repository_exclusions;
DROP TABLE ecosystem.repository_languages;
DROP TABLE ecosystem.repositories;
DROP TABLE ecosystem.languages;
DROP TABLE ecosystem.github_owners;
