FROM rust:1-bookworm AS builder
WORKDIR /workspace

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    dbus \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --home /var/lib/bluetty --create-home --shell /usr/sbin/nologin bluetty

COPY --from=builder /workspace/target/release/bluetty /usr/local/bin/bluetty
COPY bluetty.conf.minimal /etc/bluetty/bluetty.conf

ENV RUST_LOG=info
ENV BLUETTY_CONFIG=/etc/bluetty/bluetty.conf

ENTRYPOINT ["/usr/local/bin/bluetty"]
