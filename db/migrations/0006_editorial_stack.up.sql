-- 0006 · Arsenal editorial (docs/README_STACK.json)
--
-- Ferramentas e plataformas não são medidas pelo GitHub — são curadoria com
-- evidência pública. A ordem e os ícones das categorias, hoje fixos em
-- render_arsenal_stack() (category_order), viram dado de referência; as
-- ferramentas em si chegam pela importação do manifesto, não por migration.

CREATE TABLE ecosystem.stack_categories (
  slug     text PRIMARY KEY CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
  name     text     NOT NULL UNIQUE,
  icon     text     NOT NULL DEFAULT '🧰',
  -- NULL = categoria sem ordem editorial; aparece depois das ordenadas, em
  -- ordem alfabética (mesmo comportamento de remaining_categories no Python).
  position smallint UNIQUE
);

INSERT INTO ecosystem.stack_categories (slug, name, icon, position) VALUES
  ('frameworks-web',        'Frameworks & Web',        '🧩', 1),
  ('infraestrutura-devops', 'Infraestrutura & DevOps', '🛡', 2),
  ('ia-conhecimento',       'IA & Conhecimento',       '🤖', 3),
  ('hardware-simulacao',    'Hardware & Simulação',    '⚙',  4);

CREATE TABLE ecosystem.stack_tools (
  id            bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  name          text        NOT NULL UNIQUE,
  category_slug text        NOT NULL REFERENCES ecosystem.stack_categories (slug) ON UPDATE CASCADE,
  family        text        NOT NULL,  -- "papel" na tabela do README
  evidence      text        NOT NULL,  -- evidência pública; ferramenta sem evidência não entra
  position      smallint    NOT NULL,  -- ordem de aparição no manifesto
  created_at    timestamptz NOT NULL DEFAULT now(),
  updated_at    timestamptz NOT NULL DEFAULT now(),
  CONSTRAINT stack_tools_evidence_not_blank CHECK (btrim(evidence) <> '')
);

CREATE TRIGGER stack_tools_touch BEFORE UPDATE ON ecosystem.stack_tools
  FOR EACH ROW EXECUTE FUNCTION ecosystem.touch_updated_at();
