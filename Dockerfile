FROM rust:1.85 AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY skyscraper-api/ skyscraper-api/
COPY skyscraper-worker/ skyscraper-worker/

RUN cargo build --release --workspace

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/skyscraper-api /usr/local/bin/
COPY --from=builder /app/target/release/skyscraper-worker /usr/local/bin/

EXPOSE 3002

CMD ["skyscraper-api"]
