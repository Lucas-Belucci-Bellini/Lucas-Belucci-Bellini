-- 0008 · Ordem de chegada (reversão)
--
-- A ordem é derivada (a próxima sync github a reescreve), mas descartá-la
-- muda empates do README gerado do banco: com ordem gravada, só com opt-in.

DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM ecosystem.repositories WHERE inventory_position IS NOT NULL) THEN
    PERFORM ecosystem.assert_data_loss_allowed('ecosystem.repositories');
  END IF;
  IF EXISTS (SELECT 1 FROM ecosystem.repository_languages WHERE position IS NOT NULL) THEN
    PERFORM ecosystem.assert_data_loss_allowed('ecosystem.repository_languages');
  END IF;
END
$$;

ALTER TABLE ecosystem.repository_languages DROP COLUMN position;
ALTER TABLE ecosystem.repositories DROP COLUMN inventory_position;
