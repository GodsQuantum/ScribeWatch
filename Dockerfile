# syntax=docker/dockerfile:1

FROM node:26.9.0-trixie-slim@sha256:3a771f83944bb763050c23c0225c260638c4b7899e7a72485ef75e5e570499e5 AS frontend
WORKDIR /src/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run check && npm test && npm run build

FROM rust:1.98.0-trixie@sha256:7f7a53a25a0319dd8284e279d529d45759cb384d59b14cc6806132910f45522e AS builder
ARG TARGETARCH
WORKDIR /src
COPY rust-toolchain.toml ./
RUN apt-get update \
 && apt-get install -y --no-install-recommends musl-tools \
 && rm -rf /var/lib/apt/lists/* \
 && case "$TARGETARCH" in \
      amd64) target=x86_64-unknown-linux-musl ;; \
      arm64) target=aarch64-unknown-linux-musl ;; \
      *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac \
 && rustup target add "$target" \
 && printf '%s' "$target" >/tmp/rust-target
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN target="$(cat /tmp/rust-target)" \
 && cargo build --release --locked --target "$target" \
 && mkdir -p /out \
 && cp "target/$target/release/scribewatch" /out/scribewatch

FROM alpine:3.23@sha256:85fe1e81d6758c208f3e1eed4338a1997e19d4be002d4dd32d3100c9a8c010a0 AS runtime
ENV SCRIBEWATCH_HOST=0.0.0.0 \
    SCRIBEWATCH_PORT=3000 \
    SCRIBEWATCH_CONFIG_DIR=/config \
    SCRIBEWATCH_DATA_DIR=/data \
    SCRIBEWATCH_DIST_DIR=/app/frontend \
    SCRIBEWATCH_ALLOWED_ROOTS=/media \
    HOME=/tmp \
    XDG_CACHE_HOME=/tmp/.cache
RUN apk add --no-cache \
      ca-certificates curl ffmpeg font-dejavu pandoc weasyprint \
 && addgroup -S -g 1000 scribewatch \
 && adduser -S -D -H -u 1000 -G scribewatch scribewatch \
 && mkdir -p /app/frontend /config /data /media /tmp/.cache \
 && chown -R 1000:1000 /config /data /media /tmp/.cache \
 && fc-cache -f
WORKDIR /app
COPY --from=builder /out/scribewatch /app/scribewatch
COPY --from=frontend /src/frontend/build /app/frontend
USER 1000:1000
EXPOSE 3000
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["curl","--fail","--silent","--show-error","http://127.0.0.1:3000/api/v1/health"]
ENTRYPOINT ["/app/scribewatch"]
