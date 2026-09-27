-- A fronteira de privacidade (D-010) é de privilégio, não de boa vontade:
-- um papel que recebeu SELECT só nas views públicas lê a vitrine e nada mais.
-- CREATE ROLE é transacional no PostgreSQL; o ROLLBACK final apaga o papel.

BEGIN;

CREATE ROLE ecosystem_probe_public_reader NOLOGIN;
GRANT USAGE ON SCHEMA ecosystem, ecosystem_test TO ecosystem_probe_public_reader;
GRANT SELECT ON ecosystem.public_projects, ecosystem.public_repositories,
                ecosystem.public_language_totals
  TO ecosystem_probe_public_reader;

SET LOCAL ROLE ecosystem_probe_public_reader;

SELECT ecosystem_test.assert_eq('leitor público lê a vitrine',
  (SELECT string_agg(slug, ',' ORDER BY slug) FROM ecosystem.public_projects),
  'aurora-web,nimbus-cli,old-landing');
SELECT ecosystem_test.assert_eq('leitor público lê a matriz de linguagens',
  (SELECT count(*)::text FROM ecosystem.public_language_totals), '4');
SELECT ecosystem_test.expect_error('leitor público não lê a tabela de repositórios',
  $$SELECT count(*) FROM ecosystem.repositories$$, '42501');
SELECT ecosystem_test.expect_error('leitor público não lê exclusões',
  $$SELECT count(*) FROM ecosystem.repository_exclusions$$, '42501');
SELECT ecosystem_test.expect_error('leitor público não lê a listagem privada',
  $$SELECT count(*) FROM ecosystem.private_repository_listing$$, '42501');
SELECT ecosystem_test.expect_error('leitor público não lê o histórico bruto',
  $$SELECT count(*) FROM ecosystem.website_checks$$, '42501');
SELECT ecosystem_test.expect_error('leitor público não escreve',
  $$INSERT INTO ecosystem.repository_exclusions (match_name, reason) VALUES ('x', 'y')$$, '42501');

RESET ROLE;
ROLLBACK;
