FROM rust:1.93.1-bookworm@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31 AS builder

ARG TARGETARCH

WORKDIR /app

RUN test "$TARGETARCH" = amd64 \
    && apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
RUN cargo fetch --locked

COPY src ./src
RUN cargo build --locked --release --target x86_64-unknown-linux-gnu --bin ncn-meta-cron --bin ncn-router

FROM debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 tini \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 ncn-router \
    && useradd --uid 10001 --gid 10001 --no-create-home --home-dir /var/lib/ncn-router --shell /usr/sbin/nologin ncn-router \
    && install -d -o 10001 -g 10001 /var/lib/ncn-router /etc/ncn-router

COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/ncn-meta-cron /usr/local/bin/ncn-meta-cron
COPY --from=builder /app/target/x86_64-unknown-linux-gnu/release/ncn-router /usr/local/bin/ncn-router
COPY config.toml /etc/ncn-router/config.toml

USER 10001:10001
WORKDIR /var/lib/ncn-router

ENV NCN_CONFIG=/etc/ncn-router/config.toml \
    NCN_LOG=/var/lib/ncn-router/ncn_verifier_meta.log \
    NCN_ROUTER_BIND_ADDR=0.0.0.0:8080 \
    NCN_WHITELIST_MAINNET_PATH=/var/lib/ncn-router/ncn_whitelist.mainnet.json \
    NCN_WHITELIST_TESTNET_PATH=/var/lib/ncn-router/ncn_whitelist.testnet.json

EXPOSE 8080

ENTRYPOINT ["/usr/bin/tini", "-g", "--"]
CMD ["/usr/local/bin/ncn-router"]
