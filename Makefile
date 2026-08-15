SHELL := /bin/bash

ifneq (,$(wildcard .env))
include .env
export
endif

.PHONY: setup install db db-down dev dev-api dev-web test lint format format-check check-config api-image database-operations-image migrate import-demo import-tcgdex verify-database smoke-production

setup: install db

install:
	pnpm install --frozen-lockfile
	cargo fetch --locked

db:
	docker compose up -d --wait db

db-down:
	docker compose down

dev:
	$(MAKE) -j2 dev-api dev-web

dev-api:
	cargo run -p bycard-api

dev-web:
	pnpm dev

test:
	pnpm test
	cargo test --workspace

lint:
	pnpm lint
	pnpm typecheck
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --all-features -- -D warnings

format:
	pnpm format
	cargo fmt --all

format-check:
	pnpm format:check
	cargo fmt --all --check

check-config:
	cargo run -p bycard-api --bin check-config

api-image:
	docker build --tag bycard-api:local .

database-operations-image:
	docker build --target operations --tag bycard-database-operations:local .

migrate:
	cargo run -p bycard-api --bin database-operations -- migrate

import-demo: migrate
	cargo run -p bycard-api --bin database-operations -- import-demo

import-tcgdex: migrate
	cargo run -p bycard-api --bin database-operations -- import-tcgdex $(TCGDEX_SET_IDS)

verify-database:
	cargo run -p bycard-api --bin database-operations -- verify $(TCGDEX_SET_IDS)

smoke-production:
	./scripts/smoke-production.sh
