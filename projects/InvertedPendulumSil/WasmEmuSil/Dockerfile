# ipemu-server: 配信 API + 管理 API + SPA。NetworkedSil/Dockerfile と同じ流儀 (multi-stage、非 root、最小ランタイム)。
#   docker compose up --build   (compose.yaml 参照)

# ---- SPA ------------------------------------------------------------------
FROM node:22-bookworm-slim AS web
WORKDIR /src
RUN corepack enable
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY apps/web/package.json apps/web/
RUN pnpm install --frozen-lockfile --filter @ipemu/web
COPY apps/web apps/web
RUN pnpm --filter @ipemu/web build

# ---- サーバ ----------------------------------------------------------------
FROM rust:1-bookworm AS server
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates crates
COPY components components
COPY firmware firmware
COPY schemas schemas
COPY abi abi
# ワークスペース全体の依存解決が要るが、ビルドするのはサーバだけ
RUN cargo build --release -p ipemu-server --locked

# ---- ランタイム --------------------------------------------------------------
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home ipemu
WORKDIR /app
COPY --from=server /src/target/release/ipemu-server /usr/local/bin/ipemu-server
COPY --from=web /src/apps/web/dist /app/web
ENV IPEMU_LISTEN=0.0.0.0:8080 \
    IPEMU_STORE=/data/store \
    IPEMU_WEB_DIR=/app/web \
    IPEMU_SIGNING_KEY=/keys/signing.json
VOLUME ["/data"]
EXPOSE 8080
USER ipemu
HEALTHCHECK --interval=10s --timeout=3s --retries=6 CMD curl -fsS http://127.0.0.1:8080/healthz || exit 1
ENTRYPOINT ["ipemu-server"]
CMD ["serve"]
