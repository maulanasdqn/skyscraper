.PHONY: all check fmt lint test build clean run-api run-worker db-migrate db-create docker-build docker-up docker-down docker-logs

all: fmt lint test build

# ──── Development ────

check:
	cargo check --workspace

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

build:
	cargo build --workspace

build-release:
	cargo build --workspace --release

clean:
	cargo clean

# ──── Run ────

run-api:
	cargo run --bin skyscraper-api

run-worker:
	cargo run --bin skyscraper-worker

# ──── Database ────

db-create:
	createdb skyscraper

db-migrate:
	cargo sqlx migrate run --source skyscraper-api/migrations

# ──── CI (all checks in one shot) ────

ci: fmt-check lint test
	@echo "All checks passed."

# ──── Docker ────

docker-build:
	docker compose build

docker-up:
	docker compose up -d

docker-down:
	docker compose down

docker-logs:
	docker compose logs -f
