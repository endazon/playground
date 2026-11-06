# ipemu-server: 配信 API + 管理 API + SPA。NetworkedSil/Dockerfile と同じ流儀 (multi-stage、非 root、最小ランタイム)。
#   scripts/docker.sh server-up   (compose.yaml 参照)
# BASE_REGISTRY: 土台イメージの取得元 (docker.io の認証ヘルパが壊れている環境では mirror.gcr.io/library)

# ---- SPA ------------------------------------------------------------------
ARG BASE_REGISTRY=docker.io/library
FROM ${BASE_REGISTRY}/node:22-bookworm-slim AS web
WORKDIR /src
RUN corepack enable
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml ./
COPY apps/web/package.json apps/web/
RUN pnpm install --frozen-lockfile --filter @ipemu/web
COPY apps/web apps/web
RUN pnpm --filter @ipemu/web build

# ---- サーバ ----------------------------------------------------------------
FROM ${BASE_REGISTRY}/rust:1-bookworm AS server
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
FROM ${BASE_REGISTRY}/debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home ipemu
WORKDIR /app
COPY --from=server /src/target/release/ipemu-server /usr/local/bin/ipemu-server
COPY --from=web /src/apps/web/dist /app/web
ENV IPEMU_LISTEN=0.0.0.0:8080 \
    IPEMU_STORE=/data/store \
    IPEMU_WEB_DIR=/app/web \
    IPEMU_SIGNING_KEY=/keys/signing.json
# 名前付きボリュームを /data に当てたときに所有者を引き継げるよう、イメージ側で先に作っておく
# (ホストディレクトリを bind すると所有者はホスト側のまま。Windows の drvfs は uid=0 固定で
#  非 root の 10001 から書けないので、compose の既定は名前付きボリューム)
RUN mkdir -p /data/store && chown -R ipemu:ipemu /data
VOLUME ["/data"]
EXPOSE 8080
USER ipemu
HEALTHCHECK --interval=10s --timeout=3s --retries=6 CMD curl -fsS http://127.0.0.1:8080/healthz || exit 1
ENTRYPOINT ["ipemu-server"]
CMD ["serve"]
