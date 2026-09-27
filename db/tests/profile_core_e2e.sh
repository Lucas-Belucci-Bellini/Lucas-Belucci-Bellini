#!/usr/bin/env bash
# O binário `profile-core` contra o mesmo PostgreSQL do harness (run.sh):
#
#   1. o schema que `profile-core db migrate` produz é idêntico (pg_dump
#      --schema-only) ao que as migrations aplicadas uma a uma pelo psql
#      produzem — fora a tabela de controle `_sqlx_migrations`;
#   2. o seed e os testes SQL (db/tests/sql) passam sobre o schema do binário;
#   3. `db revert --all` recusa com dado e sai com 1; com --allow-data-loss
#      reverte tudo e não sobra o schema.
#
# Uso: PGHOST=… PGPORT=… PGUSER=… [PGPASSWORD=…] db/tests/profile_core_e2e.sh <binário>
# Cria e apaga os próprios bancos; nunca toca num banco existente.
set -euo pipefail

BIN="${1:?uso: $0 <caminho do profile-core>}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
: "${PGHOST:?defina PGHOST}" "${PGUSER:?defina PGUSER}"
PGPORT="${PGPORT:-5432}"
SUFFIX="$$"
DB_BIN="e2e_bin_$SUFFIX"
DB_PSQL="e2e_psql_$SUFFIX"
WORK="$(mktemp -d)"

cleanup() {
  for db in "$DB_BIN" "$DB_PSQL"; do
    psql -X -q -d postgres -c "DROP DATABASE IF EXISTS $db WITH (FORCE)" >/dev/null 2>&1 || true
  done
  rm -rf "$WORK"
}
trap cleanup EXIT

log() { printf '\n== %s\n' "$*"; }
fail() { printf 'FALHOU: %s\n' "$*" >&2; exit 1; }
url() { printf 'postgres://%s%s@%s:%s/%s' "$PGUSER" "${PGPASSWORD:+:$PGPASSWORD}" "$PGHOST" "$PGPORT" "$1"; }
sql_file() { psql -X -q -v ON_ERROR_STOP=1 -o /dev/null -d "$1" "${@:3}" -f "$2"; }

psql -X -q -d postgres -c "CREATE DATABASE $DB_BIN" -c "CREATE DATABASE $DB_PSQL" >/dev/null

log "1/3 schema do binário = schema das migrations pelo psql"
DATABASE_URL="$(url "$DB_BIN")" "$BIN" db migrate
for up in "$ROOT"/db/migrations/*.up.sql; do sql_file "$DB_PSQL" "$up" -1; done
dump() { pg_dump --schema-only --no-owner --no-privileges --exclude-table=_sqlx_migrations -d "$1" | grep -v '^\\\(un\)\?restrict '; }
dump "$DB_BIN" > "$WORK/bin.sql"
dump "$DB_PSQL" > "$WORK/psql.sql"
diff -u "$WORK/psql.sql" "$WORK/bin.sql" || fail "o schema aplicado pelo binário difere do aplicado pelo psql"
echo "  ok  idênticos ($(grep -c '^CREATE' "$WORK/bin.sql") CREATE)"

log "2/3 seed e testes SQL sobre o schema do binário"
sql_file "$DB_BIN" "$ROOT/db/seeds/dev/0001_demo_ecosystem.sql" -1
for test in "$ROOT"/db/tests/sql/*.sql; do
  sql_file "$DB_BIN" "$test"
  echo "  ok  $(basename "$test")"
done
psql -X -q -d "$DB_BIN" -c 'SET client_min_messages = warning' -c 'DROP SCHEMA ecosystem_test CASCADE'

log "3/3 revert com dado: recusa sem opt-in, passa com opt-in"
set +e
DATABASE_URL="$(url "$DB_BIN")" "$BIN" db revert --all 2> "$WORK/revert.err"
code=$?
set -e
[ "$code" = 1 ] || fail "revert --all com dado deveria sair com 1, saiu com $code"
grep -q "apagaria dados" "$WORK/revert.err" || fail "mensagem de recusa ausente: $(cat "$WORK/revert.err")"
DATABASE_URL="$(url "$DB_BIN")" "$BIN" db status --json | grep -q '"state":"pending"' && fail "a recusa reverteu alguma migration"
echo "  ok  recusado, nada revertido"
DATABASE_URL="$(url "$DB_BIN")" "$BIN" db revert --all --allow-data-loss >/dev/null
left="$(psql -X -At -d "$DB_BIN" -c "SELECT count(*) FROM pg_namespace WHERE nspname = 'ecosystem'")"
[ "$left" = 0 ] || fail "o schema ecosystem sobrou depois do revert"
echo "  ok  revertido com --allow-data-loss"

printf '\nprofile-core: e2e ok\n'
