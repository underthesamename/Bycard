#!/bin/sh

set -eu

: "${ENCRYPTED_BACKUP_FILE:?ENCRYPTED_BACKUP_FILE is required}"
: "${BACKUP_FILE:?BACKUP_FILE is required}"
: "${BACKUP_ENCRYPTION_KEY:?BACKUP_ENCRYPTION_KEY is required}"

if [ ! -f "$ENCRYPTED_BACKUP_FILE" ]; then
    echo "ENCRYPTED_BACKUP_FILE must point to a regular file" >&2
    exit 1
fi
if [ ! -f "$ENCRYPTED_BACKUP_FILE.sha256" ]; then
    echo "encrypted backup checksum is missing" >&2
    exit 1
fi
if [ "${#BACKUP_ENCRYPTION_KEY}" -lt 32 ]; then
    echo "BACKUP_ENCRYPTION_KEY must contain at least 32 characters" >&2
    exit 1
fi
if [ "$BACKUP_FILE" = "$ENCRYPTED_BACKUP_FILE" ]; then
    echo "BACKUP_FILE must differ from ENCRYPTED_BACKUP_FILE" >&2
    exit 1
fi

encrypted_directory=$(dirname "$ENCRYPTED_BACKUP_FILE")
encrypted_name=$(basename "$ENCRYPTED_BACKUP_FILE")
(cd "$encrypted_directory" && sha256sum -c "$encrypted_name.sha256")

backup_directory=$(dirname "$BACKUP_FILE")
backup_name=$(basename "$BACKUP_FILE")
case "$backup_name" in
    *[!A-Za-z0-9._-]* | "" | .* | *..*)
        echo "backup filename contains unsupported characters" >&2
        exit 1
        ;;
esac
if [ ! -d "$backup_directory" ]; then
    echo "backup directory does not exist" >&2
    exit 1
fi
if [ -e "$BACKUP_FILE" ] || [ -e "$BACKUP_FILE.sha256" ]; then
    echo "decrypted backup output already exists" >&2
    exit 1
fi

umask 077
temporary_backup=$(mktemp "$backup_directory/.bycard-decrypted.XXXXXX")
temporary_checksum=$(mktemp "$backup_directory/.bycard-decrypted-checksum.XXXXXX")
temporary_gnupg_home=$(mktemp -d "$backup_directory/.bycard-gnupg.XXXXXX")
export GNUPGHOME="$temporary_gnupg_home"
published_backup=false
cleanup() {
    gpgconf --kill gpg-agent >/dev/null 2>&1 || true
    rm -rf -- "$temporary_gnupg_home"
    rm -f -- "$temporary_backup" "$temporary_checksum"
    if [ "$published_backup" = true ] && [ ! -f "$BACKUP_FILE.sha256" ]; then
        rm -f -- "$BACKUP_FILE"
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
    --decrypt \
    --output "$temporary_backup" \
    "$ENCRYPTED_BACKUP_FILE"

backup_digest=$(sha256sum "$temporary_backup" | cut -d ' ' -f 1)
printf '%s  %s\n' "$backup_digest" "$backup_name" >"$temporary_checksum"

mv "$temporary_backup" "$BACKUP_FILE"
published_backup=true
mv "$temporary_checksum" "$BACKUP_FILE.sha256"
published_backup=false

cleanup
trap - EXIT HUP INT TERM
printf '%s\n' "$BACKUP_FILE"
