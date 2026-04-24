# syntax=docker/dockerfile:1.7

FROM oven/bun:1.3.12 AS frontend-builder

WORKDIR /usr/src/app/web

COPY Cargo.toml /usr/src/app/Cargo.toml
COPY web/package.json web/bun.lock ./
RUN set -eux; \
    for attempt in 1 2 3; do \
        bun install --frozen-lockfile && exit 0; \
        rm -rf /root/.bun/install/cache; \
        if [ "${attempt}" -eq 3 ]; then \
            exit 1; \
        fi; \
        sleep 2; \
    done

COPY web ./
RUN bun run build

FROM rust:1.88.0-slim AS builder

WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY locales ./locales
COPY --from=frontend-builder /usr/src/app/web/dist ./web/dist

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/src/app/target \
    cargo build --release && \
    cp /usr/src/app/target/release/embystream /tmp/embystream

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /config/embystream /data/logs /data/web-config/data /data/web-config/logs

COPY src/config/config.toml.template /config/embystream/config.toml
COPY --from=builder /tmp/embystream /usr/local/bin/embystream

EXPOSE 6888
VOLUME ["/config/embystream", "/data"]

CMD ["embystream", "run", "--config", "/config/embystream/config.toml", "--web", "--web-listen", "0.0.0.0:6888", "--web-data-dir", "/data/web-config/data", "--web-runtime-log-dir", "/data/web-config/logs"]
