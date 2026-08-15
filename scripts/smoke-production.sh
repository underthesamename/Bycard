#!/usr/bin/env bash

set -Eeuo pipefail

readonly REQUEST_TIMEOUT_SECONDS=20
readonly RESPONSE_ATTEMPTS=9
readonly RESPONSE_RETRY_DELAY_SECONDS=10

fail() {
  printf 'smoke test failed: %s\n' "$1" >&2
  exit 1
}

require_https_origin() {
  local name=$1
  local origin=$2
  local authority

  case "$origin" in
    https://*) ;;
    *) fail "$name must be an HTTPS origin" ;;
  esac

  authority=${origin#https://}
  case "$authority" in
    "" | */* | *\?* | *\#* | *@*)
      fail "$name must contain only scheme, host, and optional port"
      ;;
  esac
}

fetch_body() {
  curl \
    --fail \
    --silent \
    --show-error \
    --connect-timeout 10 \
    --location \
    --max-time "$REQUEST_TIMEOUT_SECONDS" \
    --proto '=https' \
    --proto-redir '=https' \
    --retry 1 \
    --retry-all-errors \
    --header 'Accept: application/json' \
    "$1"
}

assert_eventually_contains() {
  local url=$1
  local expected=$2
  local body
  local attempt

  for ((attempt = 1; attempt <= RESPONSE_ATTEMPTS; attempt++)); do
    if body=$(fetch_body "$url") && [[ "$body" == *"$expected"* ]]; then
      return
    fi

    if ((attempt < RESPONSE_ATTEMPTS)); then
      printf 'waiting for %s (%d/%d)\n' \
        "$url" "$attempt" "$RESPONSE_ATTEMPTS" >&2
      sleep "$RESPONSE_RETRY_DELAY_SECONDS"
    fi
  done

  fail "$url returned an unexpected response"
}

assert_web_security_headers() {
  local headers

  headers=$(
    curl \
      --fail \
      --silent \
      --show-error \
      --connect-timeout 10 \
      --location \
      --max-time "$REQUEST_TIMEOUT_SECONDS" \
      --proto '=https' \
      --proto-redir '=https' \
      --retry 1 \
      --retry-all-errors \
      --dump-header - \
      --output /dev/null \
      "$WEB_ORIGIN/"
  )
  headers=${headers,,}

  [[ "$headers" == *$'x-content-type-options: nosniff\r'* ]] ||
    fail "the frontend is missing X-Content-Type-Options"
  [[ "$headers" == *$'x-frame-options: deny\r'* ]] ||
    fail "the frontend is missing X-Frame-Options"
  [[ "$headers" == *$'referrer-policy: strict-origin-when-cross-origin\r'* ]] ||
    fail "the frontend is missing Referrer-Policy"
}

: "${WEB_ORIGIN:?WEB_ORIGIN is required}"
: "${API_ORIGIN:?API_ORIGIN is required}"

require_https_origin WEB_ORIGIN "$WEB_ORIGIN"
require_https_origin API_ORIGIN "$API_ORIGIN"

assert_eventually_contains "$API_ORIGIN/health/live" '"status":"ok"'
assert_eventually_contains "$API_ORIGIN/health/ready" '"status":"ready"'
assert_eventually_contains "$API_ORIGIN/api/v1/sets?pageSize=1" '"data":['
assert_eventually_contains "$WEB_ORIGIN/api/v1/sets?pageSize=1" '"data":['
assert_web_security_headers

printf 'production smoke test passed\n'
