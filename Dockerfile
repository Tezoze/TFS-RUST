# TFS Rust server. LuaJIT is vendored via mlua; compile with SQLX_OFFLINE (committed `.sqlx/`).
# C++ leftover image (Alpine + cmake `tfs`) was replaced 2026-08-14.
# No BuildKit `--mount=type=cache` — Compose on some hosts still uses the legacy builder.

FROM rust:1.96-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src

COPY Cargo.toml ./
COPY rust-src ./rust-src
COPY crates ./crates
COPY tools ./tools
COPY .sqlx ./.sqlx

ENV SQLX_OFFLINE=true \
    CARGO_TERM_COLOR=never

RUN cargo build --release --bin tfs-rust

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libgcc-s1 bash \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 1000 --home-dir /srv --create-home tfs

COPY --from=builder /src/target/release/tfs-rust /usr/local/bin/tfs-rust
COPY --chown=tfs:tfs config.lua.dist /srv/config.lua
COPY --chown=tfs:tfs key.pem /srv/key.pem
COPY --chown=tfs:tfs data /srv/data
COPY --chown=tfs:tfs crates/tfs-rust-db/migrations /srv/migrations
COPY docker/entrypoint.sh /usr/local/bin/tfs-rust-entrypoint.sh

RUN chmod +x /usr/local/bin/tfs-rust-entrypoint.sh \
    && mkdir -p /srv/data/logs \
    && chown -R tfs:tfs /srv

ENV TFS_CONFIG=/srv/config.lua \
    TFS_DATA_DIR=/srv/data \
    TFS_RSA_PEM=/srv/key.pem \
    TFS_MIGRATIONS_DIR=/srv/migrations

WORKDIR /srv
USER tfs
EXPOSE 7171 7172
ENTRYPOINT ["/usr/local/bin/tfs-rust-entrypoint.sh"]
