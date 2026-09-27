-- 0003 · Projetos, taxonomia e curadoria
--
-- Repositório é fato do GitHub; projeto é a unidade editorial que o perfil
-- apresenta. Hoje a relação é 1:1 (um projeto por repositório), mas o modelo
-- aceita um projeto com vários repositórios — o Projeto Baluarte e seus
-- domínios `baluarte-*`, por exemplo.
--
-- A taxonomia que hoje vive espalhada no código (CATEGORIES e
-- CATEGORY_ALIASES em project_catalog.py, rótulos de classify() e DOMAINS em
-- update_profile.py) vira dado de referência com chave estrangeira: a regra
-- "todo rótulo cai em algum domínio", que hoje é um teste, passa a ser uma
-- restrição NOT NULL.

-- Taxonomia canônica fechada (CATEGORIES). Cresce por migration, nunca por dado de entrada.
CREATE TABLE ecosystem.categories (
  slug     text PRIMARY KEY CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
  name     text     NOT NULL UNIQUE,
  position smallint NOT NULL UNIQUE
);

INSERT INTO ecosystem.categories (slug, name, position) VALUES
  ('ai-intelligence',     'AI & Intelligence',     1),
  ('web-saas',            'Web & SaaS',            2),
  ('games',               'Games',                 3),
  ('infrastructure',      'Infrastructure',        4),
  ('automation',          'Automation',            5),
  ('education',           'Education',             6),
  ('productivity',        'Productivity',          7),
  ('research',            'Research',              8),
  ('security',            'Security',              9),
  ('hardware-simulation', 'Hardware & Simulation', 10),
  ('software-tools',      'Software & Tools',      11),
  ('experimental',        'Experimental',          12);

-- Domínios da vitrine (DOMAINS, bloco WHAT-I-BUILD).
CREATE TABLE ecosystem.showcase_domains (
  slug     text PRIMARY KEY CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
  icon     text     NOT NULL,
  title    text     NOT NULL UNIQUE,
  summary  text     NOT NULL,
  position smallint NOT NULL UNIQUE
);

INSERT INTO ecosystem.showcase_domains (slug, icon, title, summary, position) VALUES
  ('web-saas',       '🌐', 'WEB & SAAS',       'Plataformas, ferramentas e produtos web publicados.',   1),
  ('ai-automation',  '🤖', 'AI & AUTOMATION',  'Agentes, automação e sistemas de conhecimento.',        2),
  ('hardware-logic', '⚙',  'HARDWARE & LOGIC', 'Lógica digital, CPUs do zero e eletrônica.',            3),
  ('games-worlds',   '🎮', 'GAMES & WORLDS',   'Jogos, simulações e mundos jogáveis.',                  4),
  ('tools-systems',  '🛠', 'TOOLS & SYSTEMS',  'Utilitários, scripts e infraestrutura de apoio.',       5),
  ('academic-labs',  '🎓', 'ACADEMIC & LABS',  'Trabalhos de curso, estudos dirigidos e experimentos.', 6);

-- Rótulos editoriais exibidos no README (saída de classify()), cada um ligado
-- à categoria canônica (CATEGORY_ALIASES) e ao domínio da vitrine (DOMAINS).
CREATE TABLE ecosystem.classification_labels (
  slug          text PRIMARY KEY CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
  label         text NOT NULL UNIQUE,
  category_slug text NOT NULL REFERENCES ecosystem.categories (slug) ON UPDATE CASCADE,
  domain_slug   text NOT NULL REFERENCES ecosystem.showcase_domains (slug) ON UPDATE CASCADE
);

INSERT INTO ecosystem.classification_labels (slug, label, category_slug, domain_slug) VALUES
  ('digital-logic-hardware',       'Digital Logic / Hardware',         'hardware-simulation', 'hardware-logic'),
  ('ecossistema-baluarte',         'Ecossistema Baluarte',             'web-saas',            'web-saas'),
  ('academia',                     'Academia',                         'education',           'academic-labs'),
  ('games',                        'Games',                            'games',               'games-worlds'),
  ('ia-automacao',                 'IA & Automação',                   'ai-intelligence',     'ai-automation'),
  ('infraestrutura-backend-dados', 'Infraestrutura / Backend / Dados', 'infrastructure',      'tools-systems'),
  ('web',                          'Web',                              'web-saas',            'web-saas'),
  ('experimentos',                 'Experimentos',                     'experimental',        'academic-labs'),
  ('software-ferramentas',         'Software & Ferramentas',           'software-tools',      'tools-systems');

CREATE TABLE ecosystem.projects (
  id                 bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  slug               text        NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
  name               text        NOT NULL,
  -- Texto editorial (substitui FEATURED_SUMMARIES). NULL = usar a descrição do GitHub.
  summary            text,
  label_slug         text        REFERENCES ecosystem.classification_labels (slug) ON UPDATE CASCADE,
  label_source       text        NOT NULL DEFAULT 'heuristic' CHECK (label_source IN ('heuristic', 'editorial')),
  classifier_version text,       -- versão da heurística que gravou label_slug; NULL quando editorial
  -- Status de ciclo de vida é calculado (idade do push, arquivado, acadêmico...).
  -- Aqui mora só a sobreposição editorial (hoje: nomes fixos em status_for()).
  lifecycle_override text        CHECK (lifecycle_override IN (
                       'active', 'in_development', 'experimental', 'academic', 'archived')),
  publication        text        NOT NULL DEFAULT 'public' CHECK (publication IN ('public', 'unlisted', 'hidden')),
  created_at         timestamptz NOT NULL DEFAULT now(),
  updated_at         timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT projects_heuristic_label_is_versioned
    CHECK (label_source <> 'heuristic' OR label_slug IS NULL OR classifier_version IS NOT NULL),
  CONSTRAINT projects_editorial_label_present
    CHECK (label_source <> 'editorial' OR label_slug IS NOT NULL)
);

COMMENT ON COLUMN ecosystem.projects.publication IS
  'public = pode aparecer em saídas públicas; unlisted = só por link direto; hidden = só uso interno. '
  'Visibilidade privada do repositório sempre vence: projeto de repositório privado nunca anuncia site.';

CREATE TRIGGER projects_touch BEFORE UPDATE ON ecosystem.projects
  FOR EACH ROW EXECUTE FUNCTION ecosystem.touch_updated_at();

CREATE TABLE ecosystem.project_repositories (
  project_id    bigint NOT NULL REFERENCES ecosystem.projects (id) ON DELETE CASCADE,
  repository_id bigint NOT NULL UNIQUE REFERENCES ecosystem.repositories (id),
  role          text   NOT NULL DEFAULT 'primary'
                  CHECK (role IN ('primary', 'component', 'documentation', 'mirror')),
  PRIMARY KEY (project_id, repository_id)
);

COMMENT ON TABLE ecosystem.project_repositories IS
  'Um repositório pertence a no máximo um projeto; um projeto tem no máximo um repositório primário.';

CREATE UNIQUE INDEX project_repositories_one_primary
  ON ecosystem.project_repositories (project_id) WHERE role = 'primary';

-- Curadoria da vitrine (docs/README_FEATURED.json, schema readme-featured@2).
CREATE TABLE ecosystem.featured_entries (
  project_id       bigint PRIMARY KEY REFERENCES ecosystem.projects (id) ON DELETE CASCADE,
  position         smallint    NOT NULL CHECK (position > 0),           -- "order"
  priority         smallint    CHECK (priority BETWEEN 0 AND 100),      -- "priority"
  label            text        NOT NULL,                                -- "label"
  focus            text,                                                -- "focus"
  reason           text,                                                -- "reason" (documentação, não renderizado)
  website_required boolean     NOT NULL DEFAULT false,                  -- "website_required"
  created_at       timestamptz NOT NULL DEFAULT now(),
  updated_at       timestamptz NOT NULL DEFAULT now(),
  -- DEFERRABLE: reordenar a vitrine troca posições dentro da mesma transação.
  CONSTRAINT featured_entries_position_key UNIQUE (position) DEFERRABLE INITIALLY DEFERRED
);

CREATE TRIGGER featured_entries_touch BEFORE UPDATE ON ecosystem.featured_entries
  FOR EACH ROW EXECUTE FUNCTION ecosystem.touch_updated_at();
