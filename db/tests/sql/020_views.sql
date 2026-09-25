-- A projeção pública é o contrato de privacidade e de vitrine. Estes testes
-- conferem, sobre o seed de desenvolvimento, as mesmas regras que
-- tests/test_project_catalog.py confere no Python.

-- 1. Só entra o que é público, não excluído e ainda existe.
SELECT ecosystem_test.assert_eq('projetos públicos',
  (SELECT string_agg(slug, ',' ORDER BY slug) FROM ecosystem.public_projects),
  'aurora-web,nimbus-cli,old-landing');
SELECT ecosystem_test.assert_true('repositório excluído some da base pública',
  NOT EXISTS (SELECT 1 FROM ecosystem.public_repositories WHERE name = 'secret-client'));
SELECT ecosystem_test.assert_true('repositório privado some da base pública',
  NOT EXISTS (SELECT 1 FROM ecosystem.public_repositories WHERE name = 'vault-notes'));

-- 2. Site no ar vira CTA primário; o código vira secundário.
SELECT ecosystem_test.assert_eq('aurora: site publicado',
  (SELECT website FROM ecosystem.public_projects WHERE slug = 'aurora-web'), 'https://aurora-web.example.org');
SELECT ecosystem_test.assert_eq('aurora: CTA primário',
  (SELECT primary_cta || '/' || secondary_cta FROM ecosystem.public_projects WHERE slug = 'aurora-web'), 'website/github');
SELECT ecosystem_test.assert_eq('aurora: taxonomia canônica e domínio',
  (SELECT category || ' | ' || domain || ' | ' || category_label FROM ecosystem.public_projects WHERE slug = 'aurora-web'),
  'Web & SaaS | WEB & SAAS | Web');
SELECT ecosystem_test.assert_true('aurora: curadoria',
  (SELECT featured AND featured_position = 1 AND featured_priority = 90 FROM ecosystem.public_projects WHERE slug = 'aurora-web'));

-- 3. Sem site: só o código.
SELECT ecosystem_test.assert_eq('nimbus: sem site',
  (SELECT coalesce(website, '∅') || '/' || primary_cta || '/' || coalesce(secondary_cta, '∅') || '/' || website_status
   FROM ecosystem.public_projects WHERE slug = 'nimbus-cli'),
  '∅/github/∅/none');

-- 4. Site que caiu: continua declarado (para ser cobrado), mas não vira link —
--    mesmo tendo respondido 200 no passado.
SELECT ecosystem_test.assert_eq('old-landing: site caído não é publicado',
  (SELECT coalesce(website, '∅') || '/' || website_declared || '/' || website_status || '/' || website_http_status || '/' || primary_cta
   FROM ecosystem.public_projects WHERE slug = 'old-landing'),
  '∅/https://old-landing.example.org/unreachable/404/github');

-- 5. Privado aparece só na listagem privada autorizada, e sem site.
SELECT ecosystem_test.assert_eq('listagem privada',
  (SELECT string_agg(name, ',') FROM ecosystem.private_repository_listing), 'vault-notes');
SELECT ecosystem_test.assert_true('listagem privada não expõe coluna de site',
  NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'ecosystem' AND table_name = 'private_repository_listing'
      AND column_name IN ('website', 'website_declared', 'homepage', 'url')));

-- 6. Matriz de linguagens: só bytes públicos e não excluídos.
SELECT ecosystem_test.assert_eq('linguagens públicas',
  (SELECT string_agg(language || '=' || bytes || '/' || repositories, ',' ORDER BY language) FROM ecosystem.public_language_totals),
  'CSS=20000/2,HTML=10000/1,Rust=25000/1,TypeScript=60000/1');
SELECT ecosystem_test.assert_true('participações somam 100%',
  (SELECT abs(sum(share_percent) - 100) < 0.01 FROM ecosystem.public_language_totals));

-- 7. Histórico de sites: o estado atual é a última checagem; a disponibilidade vem do histórico.
SELECT ecosystem_test.assert_eq('estado atual por site',
  (SELECT string_agg(url || '=' || is_online, ',' ORDER BY url) FROM ecosystem.website_status_current),
  'https://aurora-web.example.org=true,https://old-landing.example.org=false,https://secret.example.com=true,https://vault-notes.example.org=true');
SELECT ecosystem_test.assert_eq('uptime de 30 dias da aurora',
  (SELECT checks || '/' || verified_checks || '/' || uptime_percent FROM ecosystem.website_uptime_30d
   WHERE url = 'https://aurora-web.example.org'),
  '2/1/50.00');
SELECT ecosystem_test.assert_eq('checagem de 40 dias atrás fica fora da janela',
  (SELECT checks || '/' || uptime_percent FROM ecosystem.website_uptime_30d WHERE url = 'https://old-landing.example.org'),
  '1/0.00');

-- 8. Monitor de commits e métricas: vale sempre a observação mais recente.
SELECT ecosystem_test.assert_eq('cabeça atual da aurora',
  (SELECT h.head_sha FROM ecosystem.repository_head_current h
   JOIN ecosystem.repositories r ON r.id = h.repository_id WHERE r.name = 'aurora-web'),
  repeat('b', 40));
SELECT ecosystem_test.assert_eq('erro de consulta vira estado, não exceção',
  (SELECT h.error_stage FROM ecosystem.repository_head_current h
   JOIN ecosystem.repositories r ON r.id = h.repository_id WHERE r.name = 'old-landing'),
  'latest_commit');
SELECT ecosystem_test.assert_eq('métrica: coleta mais recente vence',
  (SELECT value::text FROM ecosystem.metric_latest WHERE metric_key = 'profile.contributions.total'), '42');
SELECT ecosystem_test.assert_eq('contadores legados importados',
  (SELECT string_agg(metric_key || '=' || value, ',' ORDER BY metric_key) FROM ecosystem.metric_latest
   WHERE provenance = 'legacy_import'),
  'ecosystem.commits.monitor_total=10,ecosystem.commits.project_total=100,ecosystem.commits.tracked_total=110');

-- 9. Todo rótulo editorial cai num domínio e numa categoria (antes: teste em
--    tests/test_profile_showcase.py; agora também restrição do banco).
SELECT ecosystem_test.assert_eq('rótulos cobertos',
  (SELECT count(*)::text FROM ecosystem.classification_labels l
   JOIN ecosystem.categories c ON c.slug = l.category_slug
   JOIN ecosystem.showcase_domains d ON d.slug = l.domain_slug),
  '9');
