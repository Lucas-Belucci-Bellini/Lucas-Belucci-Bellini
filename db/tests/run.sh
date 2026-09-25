#!/usr/bin/env bash
# Testes das migrations do núcleo do ecossistema.
#
#   db/tests/run.sh
#
# Servidor:
#   * com PGHOST definido, usa esse servidor (CI: container postgres:16);
#   * sem PGHOST, sobe um cluster PostgreSQL temporário e o apaga no fim.
#
# Em qualquer caso cria um banco NOVO, com nome aleatório, e o remove no fim:
# o harness nunca toca num banco existente. Precisa de permissão CREATEDB.
#
# O que é verificado:
#   1. cada migration: up → down devolve o schema exatamente ao estado anterior;
#   2. o seed de desenvolvimento carrega sobre o schema completo;
#   3. os testes SQL (restrições, views públicas, histórico append-only,
#      privilégios do leitor público);
#   4. todo "down" que apagaria dados recusa sem opt-in e passa com opt-in,
#      até o schema sumir por completo;
#   5. reaplicar tudo reproduz o mesmo schema (idempotência do ciclo);
#   6. (se o sqlx-cli estiver instalado) o mesmo diretório roda no sqlx:
#      migrate run + revert até zero.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MIGRATIONS="$ROOT/db/migrations"
SEED="$ROOT/db/seeds/dev/0001_demo_ecosystem.sql"
TESTS="$ROOT/db/tests/sql"
WORK="$(mktemp -d)"
TEST_DB="ecosystem_test_$$_$RANDOM"
SQLX_DB="${TEST_DB}_sqlx"
STARTED_CLUSTER=0

log()  { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
fail() { printf '\033[31mFALHOU:\033[0m %s\n' "$*" >&2; exit 1; }

as_postgres() {
  # initdb/pg_ctl recusam rodar como root.
  if [ "$(id -u)" = 0 ]; then runuser -u postgres -- "$@"; else "$@"; fi
}

cleanup() {
  set +e
  if [ -n "${PGHOST:-}" ]; then
    psql -X -q -d postgres -c "DROP DATABASE IF EXISTS $TEST_DB" >/dev/null 2>&1
    psql -X -q -d postgres -c "DROP DATABASE IF EXISTS $SQLX_DB" >/dev/null 2>&1
  fi
  if [ "$STARTED_CLUSTER" = 1 ]; then
    as_postgres "$PG_BIN/pg_ctl" -D "$WORK/data" -m immediate stop >/dev/null 2>&1
  fi
  rm -rf "$WORK"
}
trap cleanup EXIT

start_temporary_cluster() {
  PG_BIN=""
  for dir in "${PG_BIN_DIR:-}" "$(pg_config --bindir 2>/dev/null || true)" /usr/lib/postgresql/*/bin; do
    if [ -n "$dir" ] && [ -x "$dir/initdb" ] && [ -x "$dir/pg_ctl" ]; then PG_BIN="$dir"; break; fi
  done
  [ -n "$PG_BIN" ] || fail "initdb/pg_ctl não encontrados; defina PGHOST para usar um servidor existente ou PG_BIN_DIR"
  [ "$(id -u)" != 0 ] || chown postgres "$WORK"
  local port=$(( 55000 + RANDOM % 5000 ))
  as_postgres "$PG_BIN/initdb" -D "$WORK/data" -U postgres --auth=trust -E UTF8 --locale=C.UTF-8 >/dev/null
  as_postgres "$PG_BIN/pg_ctl" -D "$WORK/data" -l "$WORK/postgres.log" -w \
    -o "-c listen_addresses=127.0.0.1 -p $port -k $WORK" start >/dev/null
  STARTED_CLUSTER=1
  export PGHOST=127.0.0.1 PGPORT="$port" PGUSER=postgres
}

run_sql_file() {  # arquivo [opções extras do psql]
  local file="$1"; shift
  psql -X -q -v ON_ERROR_STOP=1 -o /dev/null -d "$TEST_DB" "$@" -f "$file"
}

dump_schema() {
  # Sem comentários de cabeçalho nem as linhas \restrict aleatórias do pg_dump ≥ 16.10.
  pg_dump --schema-only --no-owner --no-privileges -d "$TEST_DB" \
    | grep -v -E '^(--|\\restrict|\\unrestrict|SET |SELECT pg_catalog\.set_config)' \
    | sed '/^$/d'
}

schema_exists() {
  [ "$(psql -X -At -d "$TEST_DB" -c "SELECT count(*) FROM pg_namespace WHERE nspname = 'ecosystem'")" = 1 ]
}

if [ -z "${PGHOST:-}" ]; then
  log "iniciando cluster PostgreSQL temporário"
  start_temporary_cluster
fi
psql -X -q -d postgres -c "CREATE DATABASE $TEST_DB" >/dev/null
echo "servidor: $(psql -X -At -d "$TEST_DB" -c 'SHOW server_version') · banco: $TEST_DB"

mapfile -t UPS < <(find "$MIGRATIONS" -name '*.up.sql' | sort)
[ "${#UPS[@]}" -gt 0 ] || fail "nenhuma migration em $MIGRATIONS"

# ------------------------------------------------------------------ 1
log "1/6 round-trip por migration (up → down → schema idêntico → up)"
for up in "${UPS[@]}"; do
  down="${up%.up.sql}.down.sql"
  name="$(basename "${up%.up.sql}")"
  [ -f "$down" ] || fail "$name não tem .down.sql — as migrations são reversíveis"
  dump_schema > "$WORK/before.sql"
  run_sql_file "$up" -1
  run_sql_file "$down" -1
  dump_schema > "$WORK/after.sql"
  diff -u "$WORK/before.sql" "$WORK/after.sql" > "$WORK/diff.txt" \
    || { cat "$WORK/diff.txt"; fail "$name: o down não desfaz exatamente o up"; }
  run_sql_file "$up" -1
  echo "  ok  $name"
done
dump_schema > "$WORK/full-schema.sql"

# ------------------------------------------------------------------ 2
log "2/6 seed de desenvolvimento"
run_sql_file "$SEED" -1
echo "  ok  $(basename "$SEED")"

# ------------------------------------------------------------------ 3
log "3/6 testes SQL"
for test in "$TESTS"/*.sql; do
  run_sql_file "$test"
  echo "  ok  $(basename "$test")"
done
psql -X -q -d "$TEST_DB" -c 'SET client_min_messages = warning' -c 'DROP SCHEMA ecosystem_test CASCADE'

# ------------------------------------------------------------------ 4
log "4/6 downs com dados: recusam sem opt-in, passam com opt-in"
for (( i=${#UPS[@]}-1; i>=0; i-- )); do
  down="${UPS[$i]%.up.sql}.down.sql"
  name="$(basename "${UPS[$i]%.up.sql}")"
  if run_sql_file "$down" -1 2> "$WORK/down.err"; then
    # Migration que cria tabela tem dados no seed; reverter sem opt-in é perda silenciosa.
    if grep -q 'CREATE TABLE' "${UPS[$i]}"; then
      fail "$name apagou tabelas com dados sem opt-in — falta ecosystem.assert_data_loss_allowed() no down (ou o seed não cobre a tabela)"
    fi
    echo "  ok  $name (só views; nada a perder)"
    continue
  fi
  grep -q 'down migration would destroy data' "$WORK/down.err" \
    || { cat "$WORK/down.err"; fail "$name falhou por outro motivo"; }
  schema_exists || fail "$name recusou mas não deixou o schema intacto"
  PGOPTIONS='-c ecosystem.allow_data_loss=on' run_sql_file "$down" -1
  echo "  ok  $name (recusou sem opt-in; passou com opt-in)"
done
schema_exists && fail "schema ecosystem sobrou depois de reverter tudo"
leftovers="$(psql -X -At -d "$TEST_DB" -c "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname NOT IN ('pg_catalog', 'information_schema', 'pg_toast')")"
[ "$leftovers" = 0 ] || fail "sobraram $leftovers objetos fora do schema ecosystem"

# ------------------------------------------------------------------ 5
log "5/6 reaplicar tudo reproduz o mesmo schema"
for up in "${UPS[@]}"; do run_sql_file "$up" -1; done
dump_schema > "$WORK/full-schema-again.sql"
diff -u "$WORK/full-schema.sql" "$WORK/full-schema-again.sql" \
  || fail "o schema reaplicado difere do primeiro"
echo "  ok  ${#UPS[@]} migrations, schema idêntico"

# ------------------------------------------------------------------ 6
log "6/6 compatibilidade com sqlx-cli"
if command -v sqlx >/dev/null 2>&1; then
  psql -X -q -d postgres -c "CREATE DATABASE $SQLX_DB" >/dev/null
  auth="${PGUSER:-postgres}${PGPASSWORD:+:$PGPASSWORD}"
  url="postgres://$auth@${PGHOST}:${PGPORT:-5432}/$SQLX_DB"
  sqlx migrate run --source "$MIGRATIONS" --database-url "$url"
  applied="$(psql -X -At -d "$SQLX_DB" -c 'SELECT count(*) FROM _sqlx_migrations WHERE success')"
  [ "$applied" = "${#UPS[@]}" ] || fail "sqlx aplicou $applied de ${#UPS[@]} migrations"
  for _ in "${UPS[@]}"; do sqlx migrate revert --source "$MIGRATIONS" --database-url "$url" >/dev/null; done
  remaining="$(psql -X -At -d "$SQLX_DB" -c "SELECT count(*) FROM _sqlx_migrations")"
  [ "$remaining" = 0 ] || fail "sqlx deixou $remaining migrations aplicadas depois do revert"
  [ "$(psql -X -At -d "$SQLX_DB" -c "SELECT count(*) FROM pg_namespace WHERE nspname = 'ecosystem'")" = 0 ] \
    || fail "sqlx revert deixou o schema ecosystem"
  echo "  ok  $(sqlx --version): run + revert de ${#UPS[@]} migrations"
else
  echo "  --  sqlx-cli não instalado; etapa pulada (cargo install sqlx-cli --no-default-features --features postgres,rustls)"
fi

log "migrations: todas as verificações passaram"
