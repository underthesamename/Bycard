#!/bin/sh

set -eu

: "${PGHOST:?PGHOST is required}"
: "${PGPORT:?PGPORT is required}"
: "${PGDATABASE:?PGDATABASE is required}"
: "${PGUSER:?PGUSER is required}"
: "${PGPASSWORD:?PGPASSWORD is required}"
: "${BACKUP_FILE:?BACKUP_FILE is required}"

app_env=${APP_ENV:-production}
if [ "$app_env" = production ]; then
    if [ "${PGSSLMODE:-}" != verify-full ]; then
        echo "PGSSLMODE must be verify-full in production" >&2
        exit 1
    fi
    if [ "${PGCHANNELBINDING:-}" != require ]; then
        echo "PGCHANNELBINDING must be require in production" >&2
        exit 1
    fi
fi

if [ ! -f "$BACKUP_FILE" ]; then
    echo "BACKUP_FILE must point to a regular file" >&2
    exit 1
fi
checksum_file="$BACKUP_FILE.sha256"
if [ ! -f "$checksum_file" ]; then
    echo "backup checksum is missing" >&2
    exit 1
fi

backup_directory=$(dirname "$BACKUP_FILE")
backup_name=$(basename "$BACKUP_FILE")
(cd "$backup_directory" && sha256sum -c "$backup_name.sha256")

table_count=$(psql --no-psqlrc --tuples-only --no-align --set=ON_ERROR_STOP=1 --command="
    SELECT COUNT(*)
    FROM pg_class AS relation
    JOIN pg_namespace AS namespace ON namespace.oid = relation.relnamespace
    WHERE namespace.nspname NOT IN ('pg_catalog', 'information_schema')
      AND namespace.nspname !~ '^pg_toast'
      AND relation.relkind IN ('r', 'p');
")
if [ "$table_count" != 0 ]; then
    echo "restore target is not empty; refusing to overwrite it" >&2
    exit 1
fi

pg_restore \
    --dbname="$PGDATABASE" \
    --exit-on-error \
    --single-transaction \
    --no-owner \
    --no-privileges \
    "$BACKUP_FILE"

printf 'Backup restored into empty database %s.\n' "$PGDATABASE"
