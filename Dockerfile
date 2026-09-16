# syntax=docker/dockerfile:1

FROM node:26.8-trixie-slim@sha256:14bf3eac4bf209d906d3c41256597d3ab1f926b2e93a79e9bdfe1efd32454239 AS frontend
WORKDIR /src/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run check && npm test && npm run build

FROM rust:1.98.0-trixie@sha256:7f7a53a25a0319dd8284e279d529d45759cb384d59b14cc6806132910f45522e AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:trixie-20260824-slim@sha256:d7e12182ce18b85b93007c1dedf31f2d29e01ccf3182cc4017c709b6259bc132 AS runtime
ENV DEBIAN_FRONTEND=noninteractive \
    SCRIBEWATCH_HOST=0.0.0.0 \
    SCRIBEWATCH_PORT=3000 \
    SCRIBEWATCH_CONFIG_DIR=/config \
    SCRIBEWATCH_DATA_DIR=/data \
    SCRIBEWATCH_DIST_DIR=/app/frontend \
    SCRIBEWATCH_ALLOWED_ROOTS=/media
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates curl \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd --gid 1000 scribewatch \
 && useradd --uid 1000 --gid 1000 --home-dir /nonexistent --shell /usr/sbin/nologin scribewatch \
 && mkdir -p /app/frontend /config /data /media \
 && chown -R 1000:1000 /config /data /media
WORKDIR /app
COPY --from=builder /src/target/release/scribewatch /app/scribewatch
COPY --from=frontend /src/frontend/build /app/frontend
USER 1000:1000
EXPOSE 3000
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["curl","--fail","--silent","--show-error","http://127.0.0.1:3000/api/v1/health"]
ENTRYPOINT ["/app/scribewatch"]
