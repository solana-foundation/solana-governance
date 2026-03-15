# Minimal runtime image for verifier-service (expects host-built binary)
# Build steps:
# 1) cargo build --release --bin verifier-service
# 2) docker build -f verifier-service/Dockerfile -t verifier-service:local .
# 3) docker run --rm -p 3000:3000 -e OPERATOR_PUBKEY=... -v $(pwd)/data:/data verifier-service:local

FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-0 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -r -u 10001 appuser

WORKDIR /app

# Data directory for SQLite
RUN mkdir -p /data && chown -R appuser:appuser /data

# Default env
ENV DB_PATH=/data/governance.db
ENV RUST_LOG=info

EXPOSE 3000

# Copy prebuilt binary from host build context (built at repo root)
COPY target/release/verifier-service /usr/local/bin/verifier-service

USER appuser

CMD ["verifier-service"]

# TODO: Add HEALTHCHECK using /healthz
