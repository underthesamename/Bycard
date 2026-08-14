#!/bin/sh

set -eu

: "${PGHOST:?PGHOST is required}"
: "${PGPORT:?PGPORT is required}"
: "${PGDATABASE:?PGDATABASE is required}"
: "${PGUSER:?PGUSER is required}"
: "${PGPASSWORD:?PGPASSWORD is required}"
: "${BACKUP_DIRECTORY:?BACKUP_DIRECTORY is required}"

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

backup_name=${BACKUP_NAME:-"bycard-$(date -u +%Y%m%dT%H%M%SZ).dump"}
case "$backup_name" in
    *[!A-Za-z0-9._-]* | "" | .* | *..*)
        echo "BACKUP_NAME contains unsupported characters" >&2
        exit 1
        ;;
esac

mkdir -p "$BACKUP_DIRECTORY"
umask 077

backup_path="$BACKUP_DIRECTORY/$backup_name"
checksum_path="$backup_path.sha256"
if [ -e "$backup_path" ] || [ -e "$checksum_path" ]; then
    echo "backup output already exists" >&2
    exit 1
fi

temporary_backup=$(mktemp "$BACKUP_DIRECTORY/.bycard-backup.XXXXXX")
temporary_checksum=$(mktemp "$BACKUP_DIRECTORY/.bycard-checksum.XXXXXX")
cleanup() {
    rm -f -- "$temporary_backup" "$temporary_checksum"
}
trap cleanup EXIT HUP INT TERM

pg_dump \
    --format=custom \
    --compress=zstd:6 \
    --no-owner \
    --no-privileges \
    --file="$temporary_backup"
pg_restore --list "$temporary_backup" >/dev/null

mv "$temporary_backup" "$backup_path"
(cd "$BACKUP_DIRECTORY" && sha256sum "$backup_name") >"$temporary_checksum"
mv "$temporary_checksum" "$checksum_path"

trap - EXIT HUP INT TERM
printf '%s\n' "$backup_path"
