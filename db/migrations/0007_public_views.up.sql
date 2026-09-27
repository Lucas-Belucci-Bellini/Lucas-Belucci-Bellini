-- 0007 · Projeção pública
--
-- As regras de privacidade e de vitrine que hoje estão só no Python
-- (resolve_presentation, filtros `if repo.get("private")`, exclusões) passam a
-- existir também no banco. Qualquer consumidor — gerador do README, API,
-- consulta manual — lê por estas views e herda as regras, em vez de cada um
-- reimplementar o filtro.
--
-- Regras (paridade com scripts/project_catalog.py e docs/PORTFOLIO-MARKETING.md):
--   1. Repositório privado, que sumiu do GitHub (gone_at) ou excluído não aparece.
--   2. Site só é publicado se a checagem mais recente foi `verified`.
--   3. Com site: CTA primário = website, secundário = github. Sem site: github, nenhum.
--   4. Nenhuma URL é deduzida: só existe site se foi declarado em `websites`.
--
-- As views são de propósito "security definer" (o padrão do PostgreSQL): um
-- papel de leitura pública recebe SELECT só nelas, nunca nas tabelas
-- (testado em db/tests/sql/040_privileges.sql). Por isso não chamam funções
-- que leiam tabelas: função é executada com o privilégio de quem consulta, e
-- o leitor público não enxerga `repository_exclusions`. O filtro fica em SQL
-- puro, numa view base, uma vez só.

-- Repositórios que podem aparecer em alguma saída: não sumiram do GitHub e
-- não foram excluídos (pelo nome curto ou pelo full_name, sem diferenciar
-- maiúsculas — mesma regra de build_data() em update_profile.py).
CREATE VIEW ecosystem.listed_repositories AS
SELECT r.*
FROM ecosystem.repositories r
WHERE r.gone_at IS NULL
  AND NOT EXISTS (
    SELECT 1 FROM ecosystem.repository_exclusions e
    WHERE lower(e.match_name) IN (lower(r.name), r.full_name_key)
  );

CREATE VIEW ecosystem.public_repositories AS
SELECT r.*
FROM ecosystem.listed_repositories r
WHERE r.visibility = 'public';

CREATE VIEW ecosystem.public_projects AS
SELECT
  p.id                                          AS project_id,
  p.slug,
  p.name,
  r.full_name                                   AS repository,
  'https://github.com/' || r.full_name          AS github_url,
  r.description                                 AS github_description,
  p.summary,
  l.label                                       AS category_label,
  c.name                                        AS category,
  d.title                                       AS domain,
  p.lifecycle_override,
  r.is_fork,
  r.is_archived,
  r.github_pushed_at,
  ws.url                                        AS website_declared,
  coalesce(ws.outcome, 'none')                  AS website_status,
  ws.http_status                                AS website_http_status,
  ws.last_checked_at                            AS website_checked_at,
  CASE WHEN ws.is_online THEN ws.url END        AS website,
  CASE WHEN ws.is_online THEN 'website' ELSE 'github' END AS primary_cta,
  CASE WHEN ws.is_online THEN 'github' END      AS secondary_cta,
  (f.project_id IS NOT NULL)                    AS featured,
  f.position                                    AS featured_position,
  f.priority                                    AS featured_priority,
  f.label                                       AS featured_label,
  f.focus                                       AS featured_focus,
  f.website_required                            AS featured_website_required
FROM ecosystem.projects p
JOIN ecosystem.project_repositories pr ON pr.project_id = p.id AND pr.role = 'primary'
JOIN ecosystem.public_repositories r   ON r.id = pr.repository_id
LEFT JOIN ecosystem.classification_labels l ON l.slug = p.label_slug
LEFT JOIN ecosystem.categories c            ON c.slug = l.category_slug
LEFT JOIN ecosystem.showcase_domains d      ON d.slug = l.domain_slug
LEFT JOIN ecosystem.website_status_current ws ON ws.project_id = p.id AND ws.is_primary
LEFT JOIN ecosystem.featured_entries f      ON f.project_id = p.id
WHERE p.publication = 'public';

COMMENT ON VIEW ecosystem.public_projects IS
  'Contrato público do catálogo. Tudo que sai daqui pode ser publicado; nada de repositório privado ou excluído entra.';

-- Divulgação privada autorizada pelo dono do perfil (bloco PRIVATE-PROJECTS):
-- nome, categoria e descrição pública — nunca site, linguagem ou conteúdo.
CREATE VIEW ecosystem.private_repository_listing AS
SELECT
  r.name,
  'https://github.com/' || r.full_name AS github_url,
  r.description,
  l.label                              AS category_label
FROM ecosystem.listed_repositories r
LEFT JOIN ecosystem.project_repositories pr ON pr.repository_id = r.id
LEFT JOIN ecosystem.projects p              ON p.id = pr.project_id
LEFT JOIN ecosystem.classification_labels l ON l.slug = p.label_slug
WHERE r.visibility <> 'public'
  AND coalesce(p.publication, 'public') <> 'hidden';

-- Matriz de linguagens (bloco LANGUAGE-STATS): só repositórios públicos,
-- forks incluídos — mesma base de language_rows() em update_profile.py.
CREATE VIEW ecosystem.public_language_totals AS
WITH totals AS (
  SELECT rl.language, sum(rl.bytes) AS bytes, count(*) AS repositories
  FROM ecosystem.repository_languages rl
  JOIN ecosystem.public_repositories r ON r.id = rl.repository_id
  GROUP BY rl.language
)
SELECT
  t.language,
  lang.display_name,
  lang.badge_color,
  t.bytes,
  t.repositories,
  round(100.0 * t.bytes / nullif(sum(t.bytes) OVER (), 0), 4) AS share_percent
FROM totals t
JOIN ecosystem.languages lang ON lang.name = t.language;
