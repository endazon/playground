# syntax=docker/dockerfile:1

# ブラウザ入りの公式イメージ。@playwright/test のバージョンと必ず揃えること。
# ビルドコンテキストはリポジトリのルート (docker/ca を共有するため)。
FROM mcr.microsoft.com/playwright:v1.56.1-noble

WORKDIR /e2e
ENV CI=1

# TLS を傍受する企業プロキシ下でも npm install できるようにする (docker/ca/README.md 参照)。
# Node は OS の信頼ストアを見ないので、NODE_EXTRA_CA_CERTS で明示的に渡す。
COPY docker/ca/ /usr/local/share/ca-certificates/extra/
RUN update-ca-certificates
ENV NODE_EXTRA_CA_CERTS=/etc/ssl/certs/ca-certificates.crt

COPY e2e/package.json e2e/package-lock.json ./
RUN npm ci --no-audit --no-fund

COPY e2e/ ./

CMD ["npx", "playwright", "test"]
