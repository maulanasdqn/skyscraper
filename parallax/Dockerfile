FROM rust:1.85-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY parallax-core parallax-core
COPY parallax-proxy parallax-proxy
COPY parallax-router parallax-router
COPY parallax-server parallax-server
COPY parallax-api parallax-api
COPY parallax-platform parallax-platform
RUN cargo build --release --bin parallax-server --bin parallax-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/parallax-server /usr/local/bin/
COPY --from=builder /build/target/release/parallax-api /usr/local/bin/
EXPOSE 1080 3001
CMD ["parallax-server"]
