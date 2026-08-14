#!/bin/sh

set -eu

: "${BACKUP_FILE:?BACKUP_FILE is required}"
: "${BACKUP_ENCRYPTION_KEY:?BACKUP_ENCRYPTION_KEY is required}"

if [ ! -f "$BACKUP_FILE" ]; then
    echo "BACKUP_FILE must point to a regular file" >&2
    exit 1
fi
if [ ! -f "$BACKUP_FILE.sha256" ]; then
    echo "backup checksum is missing" >&2
    exit 1
fi
if [ "${#BACKUP_ENCRYPTION_KEY}" -lt 32 ]; then
    echo "BACKUP_ENCRYPTION_KEY must contain at least 32 characters" >&2
    exit 1
fi

backup_directory=$(dirname "$BACKUP_FILE")
backup_name=$(basename "$BACKUP_FILE")
(cd "$backup_directory" && sha256sum -c "$backup_name.sha256")

encrypted_backup=${ENCRYPTED_BACKUP_FILE:-"$BACKUP_FILE.gpg"}
if [ "$encrypted_backup" = "$BACKUP_FILE" ]; then
    echo "ENCRYPTED_BACKUP_FILE must differ from BACKUP_FILE" >&2
    exit 1
fi

encrypted_directory=$(dirname "$encrypted_backup")
encrypted_name=$(basename "$encrypted_backup")
case "$encrypted_name" in
    *[!A-Za-z0-9._-]* | "" | .* | *..*)
        echo "encrypted backup filename contains unsupported characters" >&2
        exit 1
        ;;
esac
if [ ! -d "$encrypted_directory" ]; then
    echo "encrypted backup directory does not exist" >&2
    exit 1
fi

encrypted_checksum="$encrypted_backup.sha256"
if [ -e "$encrypted_backup" ] || [ -e "$encrypted_checksum" ]; then
    echo "encrypted backup output already exists" >&2
    exit 1
fi

maximum_bytes=${MAX_ENCRYPTED_BACKUP_BYTES:-0}
case "$maximum_bytes" in
    *[!0-9]* | "")
        echo "MAX_ENCRYPTED_BACKUP_BYTES must be a non-negative integer" >&2
        exit 1
        ;;
esac

umask 077
temporary_backup=$(mktemp "$encrypted_directory/.bycard-encrypted.XXXXXX")
temporary_checksum=$(mktemp "$encrypted_directory/.bycard-encrypted-checksum.XXXXXX")
temporary_gnupg_home=$(mktemp -d "$encrypted_directory/.bycard-gnupg.XXXXXX")
export GNUPGHOME="$temporary_gnupg_home"
published_backup=false
cleanup() {
    gpgconf --kill gpg-agent >/dev/null 2>&1 || true
    rm -rf -- "$temporary_gnupg_home"
    rm -f -- "$temporary_backup" "$temporary_checksum"
    if [ "$published_backup" = true ] && [ ! -f "$encrypted_checksum" ]; then
        rm -f -- "$encrypted_backup"
    fi
}
trap cleanup EXIT HUP INT TERM

printf '%s' "$BACKUP_ENCRYPTION_KEY" | gpg \
    --no-options \
    --batch \
    --yes \
    --no-tty \
    --pinentry-mode loopback \
    --passphrase-fd 0 \
    --symmetric \
    --cipher-algo AES256 \
    --compress-algo none \
    --s2k-mode 3 \
    --s2k-digest-algo SHA512 \
    --s2k-count 1048576 \
    --output "$temporary_backup" \
    "$BACKUP_FILE"

encrypted_bytes=$(wc -c <"$temporary_backup" | tr -d ' ')
if [ "$maximum_bytes" -gt 0 ] && [ "$encrypted_bytes" -gt "$maximum_bytes" ]; then
    echo "encrypted backup exceeds the configured storage limit" >&2
    exit 1
fi

encrypted_digest=$(sha256sum "$temporary_backup" | cut -d ' ' -f 1)
printf '%s  %s\n' "$encrypted_digest" "$encrypted_name" >"$temporary_checksum"

mv "$temporary_backup" "$encrypted_backup"
published_backup=true
mv "$temporary_checksum" "$encrypted_checksum"
published_backup=false

cleanup
trap - EXIT HUP INT TERM
printf '%s\n' "$encrypted_backup"
