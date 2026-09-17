# syntax=docker/dockerfile:1

# ============================================================
# 台車型倒立振子 SIL シミュレータ
#   build   : Blazor WASM クライアントごとサーバを publish する
#   test    : docker build --target test で全テストを実行する
#   runtime : 実行用 (既定のターゲット)
# ============================================================

# ---- 依存関係の復元 ----
# csproj だけを先に COPY することで、ソース変更では restore 層を再実行しない。
FROM mcr.microsoft.com/dotnet/sdk:10.0 AS restore
WORKDIR /src
ENV DOTNET_CLI_TELEMETRY_OPTOUT=1 \
    DOTNET_NOLOGO=1

# TLS を傍受する企業プロキシ下でも restore できるよう、docker/ca/*.crt を信頼ストアへ入れる。
# 置かれていなければ何も起きない (docker/ca/README.md 参照)。
COPY docker/ca/ /usr/local/share/ca-certificates/extra/
RUN update-ca-certificates

COPY Directory.Build.props InvertedPendulumSil.slnx ./
COPY src/Ip.Shared/Ip.Shared.csproj src/Ip.Shared/
COPY src/Ip.Client/Ip.Client.csproj src/Ip.Client/
COPY src/Ip.Server/Ip.Server.csproj src/Ip.Server/
COPY tests/Ip.Shared.Tests/Ip.Shared.Tests.csproj tests/Ip.Shared.Tests/
COPY tests/Ip.Server.Tests/Ip.Server.Tests.csproj tests/Ip.Server.Tests/
RUN dotnet restore InvertedPendulumSil.slnx

# ---- publish ----
FROM restore AS build
COPY src/ src/
RUN dotnet publish src/Ip.Server/Ip.Server.csproj -c Release -o /app --no-restore

# ---- テスト ----
# 実時間の結合テスト (Ip.Server.Tests) を含むので、CPU を絞ったビルダでは余裕を見ること。
FROM restore AS test
COPY src/ src/
COPY tests/ tests/
RUN dotnet test InvertedPendulumSil.slnx -c Release --no-restore

# ---- 実行イメージ ----
FROM mcr.microsoft.com/dotnet/aspnet:10.0 AS runtime
# compose / k8s のヘルスチェックから /healthz を叩くためだけに curl を入れる
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /app ./

ENV ASPNETCORE_URLS=http://+:8080 \
    ASPNETCORE_ENVIRONMENT=Production \
    DOTNET_CLI_TELEMETRY_OPTOUT=1 \
    DOTNET_NOLOGO=1

EXPOSE 8080
USER $APP_UID

HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=5 \
    CMD curl -fsS http://localhost:8080/healthz || exit 1

ENTRYPOINT ["dotnet", "Ip.Server.dll"]
