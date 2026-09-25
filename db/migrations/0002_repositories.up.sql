-- 0002 · Repositórios (espelho mínimo do GitHub)
--
-- O GitHub é a fonte de verdade destes fatos; aqui fica só o recorte que o
-- catálogo usa, mais o que o GitHub não guarda: quando vimos o repositório
-- pela primeira vez, quando sincronizamos e quando ele deixou de aparecer.
--
-- Identidade é `github_id`, não o nome: renomear um repositório muda
-- `full_name` e não pode criar um "projeto novo" nem perder o histórico.

CREATE TABLE ecosystem.github_owners (
  id         bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  github_id  bigint      NOT NULL UNIQUE,
  login      text        NOT NULL,
  kind       text        NOT NULL CHECK (kind IN ('User', 'Organization')),
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

-- Login não é único ao longo do tempo (contas renomeiam e nomes são reusados);
-- por isso índice comum, não UNIQUE.
CREATE INDEX github_owners_login_idx ON ecosystem.github_owners (lower(login));

CREATE TRIGGER github_owners_touch BEFORE UPDATE ON ecosystem.github_owners
  FOR EACH ROW EXECUTE FUNCTION ecosystem.touch_updated_at();

-- Linguagens: o nome exato que o GitHub devolve é a chave; o rótulo público e
-- as cores de apresentação moram aqui em vez de em dicionários no código
-- (hoje há dois: LANGUAGE_DISPLAY/LANGUAGE_COLORS em update_profile.py e
-- LANG_COLORS em lang_stats.py, com paletas diferentes).
CREATE TABLE ecosystem.languages (
  name         text PRIMARY KEY,
  display_name text        NOT NULL,
  badge_color  text        CHECK (badge_color ~ '^[0-9A-Fa-f]{6}$'),
  chart_color  text        CHECK (chart_color ~ '^[0-9A-Fa-f]{6}$'),
  created_at   timestamptz NOT NULL DEFAULT now()
);

-- Dado de referência, paridade com update_profile.py (LANGUAGE_DISPLAY e
-- LANGUAGE_COLORS). Linguagem nova é criada pelo coletor com display_name = name.
INSERT INTO ecosystem.languages (name, display_name, badge_color) VALUES
  ('JavaScript', 'JavaScript', 'F7DF1E'),
  ('TypeScript', 'TypeScript', '3178C6'),
  ('HTML',       'HTML',       'E34F26'),
  ('CSS',        'CSS',        '1572B6'),
  ('Java',       'Java',       'ED8B00'),
  ('Python',     'Python',     '3776AB'),
  ('C#',         'C#',         '239120'),
  ('PLpgSQL',    'PL/pgSQL',   '336791'),
  ('Rust',       'Rust',       'DEA584'),
  ('Shell',      'Shell',      '4EAA25'),
  ('GDScript',   'GDScript',   '478CBF'),
  ('PowerShell', 'PowerShell', '5391FE'),
  ('Portugol',   'Portugol',   '6A5ACD'),
  ('Batchfile',  'Batch',      '5C2D91'),
  ('ShaderLab',  'ShaderLab',  'A48EFA'),
  ('Dockerfile', 'Dockerfile', '2496ED');

CREATE TABLE ecosystem.repositories (
  id                bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  github_id         bigint      NOT NULL UNIQUE,
  github_node_id    text        UNIQUE,
  owner_id          bigint      NOT NULL REFERENCES ecosystem.github_owners (id),
  name              text        NOT NULL,
  full_name         text        NOT NULL,
  full_name_key     text        GENERATED ALWAYS AS (lower(full_name)) STORED,
  description       text,
  visibility        text        NOT NULL CHECK (visibility IN ('public', 'private', 'internal')),
  is_fork           boolean     NOT NULL DEFAULT false,
  is_archived       boolean     NOT NULL DEFAULT false,
  is_template       boolean     NOT NULL DEFAULT false,
  default_branch    text,
  primary_language  text        REFERENCES ecosystem.languages (name) ON UPDATE CASCADE,
  homepage          text,
  topics            text[]      NOT NULL DEFAULT '{}',
  size_kb           integer     CHECK (size_kb >= 0),
  github_created_at timestamptz,
  github_updated_at timestamptz,
  github_pushed_at  timestamptz,
  etag              text,        -- requisição condicional: 304 não gasta rate limit
  first_seen_at     timestamptz NOT NULL DEFAULT now(),
  last_synced_at    timestamptz,
  gone_at           timestamptz, -- sumiu do inventário (apagado, transferido, sem acesso). A linha nunca é apagada.
  created_at        timestamptz NOT NULL DEFAULT now(),
  updated_at        timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT repositories_full_name_matches_name CHECK (right(full_name, length(name) + 1) = '/' || name),
  -- Um nome ativo por vez; repositórios que sumiram podem repetir o nome.
  -- DEFERRABLE permite trocar nomes na mesma transação (A→B e um novo A).
  CONSTRAINT repositories_active_full_name_key
    UNIQUE NULLS NOT DISTINCT (full_name_key, gone_at) DEFERRABLE INITIALLY DEFERRED
);

COMMENT ON COLUMN ecosystem.repositories.github_id IS 'Identidade estável do GitHub; sobrevive a renomeação e transferência.';
COMMENT ON COLUMN ecosystem.repositories.gone_at IS 'Soft delete: quando o repositório deixou de aparecer. Histórico continua ligado a ele.';

CREATE INDEX repositories_owner_idx ON ecosystem.repositories (owner_id);
CREATE INDEX repositories_visibility_idx ON ecosystem.repositories (visibility) WHERE gone_at IS NULL;

CREATE TRIGGER repositories_touch BEFORE UPDATE ON ecosystem.repositories
  FOR EACH ROW EXECUTE FUNCTION ecosystem.touch_updated_at();

-- Estado atual do mapa de linguagens (bytes por linguagem). É substituído a
-- cada sincronização; a evolução no tempo, quando interessar, vira série em
-- metric_samples (ecosystem.languages.public_bytes, dimensão = linguagem).
-- Percentual NÃO é armazenado: é derivado e sairia de sincronia.
CREATE TABLE ecosystem.repository_languages (
  repository_id bigint      NOT NULL REFERENCES ecosystem.repositories (id),
  language      text        NOT NULL REFERENCES ecosystem.languages (name) ON UPDATE CASCADE,
  bytes         bigint      NOT NULL CHECK (bytes >= 0),
  observed_at   timestamptz NOT NULL DEFAULT now(),
  sync_run_id   bigint      NOT NULL REFERENCES ecosystem.sync_runs (id),
  PRIMARY KEY (repository_id, language)
);

-- Exclusões editoriais (docs/README_EXCLUDED.json). Guardadas por nome, sem
-- chave estrangeira, de propósito: o coletor não armazena nem enriquece um
-- repositório excluído. As views de 0007 filtram por aqui também, como
-- defesa em profundidade caso uma linha escape. Casa pelo nome curto OU pelo
-- full_name, sem diferenciar maiúsculas (mesma regra do update_profile.py).
CREATE TABLE ecosystem.repository_exclusions (
  id         bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  match_name text        NOT NULL,  -- 'nome' ou 'owner/nome', como no manifesto
  reason     text        NOT NULL,
  source     text        NOT NULL DEFAULT 'docs/README_EXCLUDED.json',
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX repository_exclusions_match_key ON ecosystem.repository_exclusions (lower(match_name));
