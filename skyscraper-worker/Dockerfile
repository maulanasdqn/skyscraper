FROM rust:1.85-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY skyscraper-api/Cargo.toml skyscraper-api/Cargo.toml
COPY skyscraper-worker/Cargo.toml skyscraper-worker/Cargo.toml

RUN mkdir -p skyscraper-api/src skyscraper-worker/src \
    && echo "fn main() {}" > skyscraper-api/src/main.rs \
    && echo "pub fn lib() {}" > skyscraper-api/src/lib.rs \
    && echo "fn main() {}" > skyscraper-worker/src/main.rs \
    && cargo build --release --workspace 2>/dev/null || true

COPY . .
RUN touch skyscraper-api/src/main.rs skyscraper-api/src/lib.rs skyscraper-worker/src/main.rs \
    && cargo build --release --bin skyscraper-worker

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/skyscraper-worker /usr/local/bin/

ENV RUST_LOG=skyscraper=info
CMD ["skyscraper-worker"]
