-- 0001 · Fundação
--
-- Schema dedicado, utilitários compartilhados e o registro de execuções de
-- sincronização. Tudo o que o núcleo grava depois aponta para um `sync_runs`:
-- cada linha de histórico sabe qual execução a produziu.
--
-- Requer PostgreSQL 15+ (UNIQUE NULLS NOT DISTINCT em 0002). Nenhuma extensão.

CREATE SCHEMA ecosystem;

COMMENT ON SCHEMA ecosystem IS
  'Núcleo do ecossistema do perfil Lucas-Belucci-Bellini. Dados de repositório vêm do GitHub; '
  'dados editoriais e históricos moram aqui; o README é uma representação derivada.';

-- Mantém updated_at coerente sem depender da aplicação lembrar de preenchê-lo.
CREATE FUNCTION ecosystem.touch_updated_at() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  NEW.updated_at := now();
  RETURN NEW;
END
$$;

-- Guarda das migrations "down": reverter uma migration que apaga uma tabela
-- com dados exige opt-in explícito. É a regra "nunca destruir dados sem uma
-- migração explícita" virando código, em vez de ficar só na documentação.
--
--   PGOPTIONS='-c ecosystem.allow_data_loss=on' psql ...        (psql)
--   postgres://.../db?options=-c%20ecosystem.allow_data_loss%3Don  (sqlx)
CREATE FUNCTION ecosystem.assert_data_loss_allowed(target regclass) RETURNS void
LANGUAGE plpgsql AS $$
DECLARE
  has_rows boolean;
BEGIN
  IF current_setting('ecosystem.allow_data_loss', true) = 'on' THEN
    RETURN;
  END IF;
  EXECUTE format('SELECT EXISTS (SELECT 1 FROM %s)', target) INTO has_rows;
  IF has_rows THEN
    RAISE EXCEPTION 'down migration would destroy data in %', target
      USING HINT = 'Exporte os dados antes e rode com ecosystem.allow_data_loss=on.';
  END IF;
END
$$;

-- Tabelas de histórico são append-only: UPDATE nunca, DELETE/TRUNCATE só com
-- o mesmo opt-in (um job de retenção explícito, por exemplo).
CREATE FUNCTION ecosystem.forbid_history_rewrite() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF TG_OP = 'UPDATE' THEN
    RAISE EXCEPTION '%.% is append-only: UPDATE is not allowed', TG_TABLE_SCHEMA, TG_TABLE_NAME;
  END IF;
  IF current_setting('ecosystem.allow_data_loss', true) IS DISTINCT FROM 'on' THEN
    RAISE EXCEPTION '%.% is append-only: % requires ecosystem.allow_data_loss=on',
      TG_TABLE_SCHEMA, TG_TABLE_NAME, TG_OP;
  END IF;
  IF TG_LEVEL = 'ROW' THEN
    RETURN OLD;
  END IF;
  RETURN NULL;
END
$$;

CREATE TABLE ecosystem.sync_runs (
  id            bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  kind          text        NOT NULL CHECK (kind IN (
                  'full', 'github_inventory', 'github_languages', 'commits',
                  'websites', 'contributions', 'manifests', 'render', 'legacy_import')),
  trigger       text        NOT NULL CHECK (trigger IN ('schedule', 'manual', 'push', 'api', 'test')),
  source        text        NOT NULL,  -- ex.: 'github-actions:ecosystem-watch.yml', 'cli', 'seed:dev'
  code_version  text,                  -- SHA do código que executou
  external_ref  text,                  -- ex.: id do run no GitHub Actions
  status        text        NOT NULL DEFAULT 'running'
                  CHECK (status IN ('running', 'succeeded', 'partial', 'failed')),
  started_at    timestamptz NOT NULL DEFAULT now(),
  finished_at   timestamptz,
  items_seen    integer     NOT NULL DEFAULT 0 CHECK (items_seen >= 0),
  items_changed integer     NOT NULL DEFAULT 0 CHECK (items_changed >= 0),
  error_message text,
  CONSTRAINT sync_runs_running_has_no_end CHECK ((status = 'running') = (finished_at IS NULL)),
  CONSTRAINT sync_runs_end_after_start    CHECK (finished_at IS NULL OR finished_at >= started_at),
  CONSTRAINT sync_runs_failure_explained  CHECK (status <> 'failed' OR error_message IS NOT NULL)
);

COMMENT ON TABLE ecosystem.sync_runs IS
  'Uma linha por execução de coleta/geração. Substitui o "scanned_at" solto dos JSONs e dá proveniência a todo histórico.';

CREATE INDEX sync_runs_kind_started_idx ON ecosystem.sync_runs (kind, started_at DESC);

-- Falha parcial é dado, não exceção: um repositório que respondeu 409 não
-- derruba a varredura dos outros (mesma regra do ecosystem_watch.py).
CREATE TABLE ecosystem.sync_run_errors (
  id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  sync_run_id bigint      NOT NULL REFERENCES ecosystem.sync_runs (id) ON DELETE CASCADE,
  subject     text        NOT NULL,  -- ex.: 'Lucas-Belucci-Bellini/MOD-PACK-MINE-BACKUP'
  stage       text        NOT NULL,  -- ex.: 'latest_commit'
  message     text        NOT NULL,
  occurred_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX sync_run_errors_run_idx ON ecosystem.sync_run_errors (sync_run_id);
