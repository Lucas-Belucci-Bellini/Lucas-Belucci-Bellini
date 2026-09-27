-- 0003 · Projetos, taxonomia e curadoria (reversão)

SELECT ecosystem.assert_data_loss_allowed('ecosystem.projects');

DROP TABLE ecosystem.featured_entries;
DROP TABLE ecosystem.project_repositories;
DROP TABLE ecosystem.projects;
DROP TABLE ecosystem.classification_labels;
DROP TABLE ecosystem.showcase_domains;
DROP TABLE ecosystem.categories;
