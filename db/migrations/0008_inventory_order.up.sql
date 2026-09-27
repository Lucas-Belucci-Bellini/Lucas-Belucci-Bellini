-- 0008 · Ordem de chegada do inventário e das linguagens
--
-- O README desempata pela ordem em que o GitHub listou os repositórios
-- (max() fica com o primeiro, sorted() é estável) e mostra as linguagens de
-- cada repositório na ordem da API (as cinco primeiras, na coluna de stack).
-- Sem essa ordem, gerar o README a partir do banco (render --from-db, Fase 5)
-- trocaria empates. As duas colunas são derivadas: cada `sync github`
-- reescreve a ordem; NULL é "ainda não sincronizado depois desta migration".

ALTER TABLE ecosystem.repositories
  ADD COLUMN inventory_position integer CHECK (inventory_position >= 0);

ALTER TABLE ecosystem.repository_languages
  ADD COLUMN position smallint CHECK (position >= 0);

COMMENT ON COLUMN ecosystem.repositories.inventory_position IS
  'Posição na última listagem do GitHub (0 = primeiro). Derivada; reescrita a cada sync github.';
COMMENT ON COLUMN ecosystem.repository_languages.position IS
  'Posição da linguagem na resposta da API (0 = primeira). Derivada; reescrita quando o mapa é trocado.';
