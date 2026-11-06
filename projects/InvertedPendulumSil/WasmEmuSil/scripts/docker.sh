#!/usr/bin/env bash
# WasmEmuSil のビルドと確認をすべてコンテナの中でやる入口 (ホストに Rust / Node / clang を入れない)。
#
#   usage: scripts/docker.sh [オプション] <コマンド> [引数...]
#
#   コマンド:
#     images                   ビルド用イメージ (toolchain / cross) を作る
#     artifacts [release|debug]  部品 (wasm) / FW / UI バンドルを build/ に出す
#     check [--quick]          scripts/check.sh をコンテナで通す (fmt / clippy / test / lint / build)
#     client [--all | --target <os>-<arch> ...]
#                              クライアント配布 zip を dist/ に出す。既定は --all (6 プラットフォーム)
#     run -- <cmd...>          ビルド用イメージの中で任意のコマンド (例: run -- cargo run -p ipemu-engine -- --help)
#     shell                    ビルド用イメージの対話シェル
#     server-build             サーバイメージ (ipemu-server:latest) を作る
#     server-keygen            署名鍵を dev/keys に作る (サーバイメージの keygen をコンテナで実行)
#     server-up                compose でサーバを起動 (127.0.0.1:8080)
#     server-verify            起動中のサーバを叩いて健全性を確認する
#     server-logs / server-down
#     all                      images → artifacts → server-build → server-up → server-verify → client --all
#
#   オプション:
#     --engine docker|nerdctl  使うコンテナエンジン (既定: docker が使えれば docker、無ければ nerdctl)
#     --macos-sdk <path>       macOS SDK のディレクトリ。渡したときだけ macos-* もビルドする
#     --rebuild                イメージをキャッシュ無しで作り直す
#     --base-registry <reg>    土台イメージ (rust / node / debian) の取得元
#                              (既定: docker.io/library。認証ヘルパが壊れていれば自動で
#                               mirror.gcr.io/library に切り替えて再試行する)
#     --docker-config <dir>    レジストリ認証に使う docker config ディレクトリ
#                              (既定は認証ヘルパ無しの空 config。公開イメージしか使わないため)
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) IS_WIN=1; export MSYS_NO_PATHCONV=1;;
  *) IS_WIN=0;;
esac
hostpath() { if [ "$IS_WIN" -eq 1 ]; then cygpath -w "$1"; else printf '%s' "$1"; fi; }

ENGINE="${IPEMU_CONTAINER_ENGINE:-}"
MACOS_SDK="${IPEMU_MACOS_SDK:-}"
BASE_REGISTRY="${IPEMU_BASE_REGISTRY:-}"
REBUILD=0
DOCKER_CONFIG_DIR="${IPEMU_DOCKER_CONFIG:-}"

while [ $# -gt 0 ]; do
  case "$1" in
    --engine) ENGINE="${2:?}"; shift 2;;
    --macos-sdk) MACOS_SDK="${2:?}"; shift 2;;
    --rebuild) REBUILD=1; shift;;
    --base-registry) BASE_REGISTRY="${2:?}"; shift 2;;
    --docker-config) DOCKER_CONFIG_DIR="${2:?}"; shift 2;;
    -h|--help) sed -n "2,36p" "$0"; exit 0;;
    *) break;;
  esac
done
CMD="${1:-}"; shift || true
[ -n "$CMD" ] || { sed -n "2,36p" "$0"; exit 2; }

# ---- エンジンを決める --------------------------------------------------------
if [ -z "$ENGINE" ]; then
  if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    ENGINE=docker
  elif command -v nerdctl >/dev/null 2>&1; then
    ENGINE=nerdctl
  else
    echo "docker も nerdctl も使えません (Docker Desktop / Rancher Desktop を起動してください)" >&2; exit 1
  fi
fi
command -v "$ENGINE" >/dev/null 2>&1 || { echo "$ENGINE が見つかりません" >&2; exit 1; }
compose() { "$ENGINE" compose "$@"; }

# 認証ヘルパ (wincred / osxkeychain) が壊れていると公開イメージの pull まで失敗するので、
# 既定では空の docker config を使う。私物のレジストリに入るときは --docker-config で本物を渡す
if [ -z "$DOCKER_CONFIG_DIR" ]; then
  DOCKER_CONFIG_DIR="$ROOT/build/docker-config"
  mkdir -p "$DOCKER_CONFIG_DIR"
  [ -f "$DOCKER_CONFIG_DIR/config.json" ] || echo '{}' > "$DOCKER_CONFIG_DIR/config.json"
fi
DOCKER_CONFIG="$(hostpath "$DOCKER_CONFIG_DIR")"; export DOCKER_CONFIG

TOOLCHAIN_IMAGE=ipemu-build:latest
CROSS_IMAGE=ipemu-cross:latest
SERVER_IMAGE=ipemu-server:latest
WORK_VOL=ipemu-work
CARGO_VOL=ipemu-cargo
CACHE_VOL=ipemu-cache

step() { echo; echo "### $*"; }

# 土台イメージ (rust / node / debian) の取得元。docker.io の認証ヘルパ (wincred /
# secretservice / osxkeychain) が壊れていると、資格情報が無いだけで
# "error getting credentials" でビルドごと落ちる環境がある。docker.io 以外のミラーなら
# 認証ヘルパが呼ばれないので、その症状が出たら 1 度だけミラーで作り直す。
FALLBACK_REGISTRY="${IPEMU_FALLBACK_REGISTRY:-mirror.gcr.io/library}"

# run_build <タグ> <-f とターゲットの指定...> — 認証ヘルパで落ちたらミラーで 1 度だけ再試行
run_build() {
  local tag="$1"; shift
  local extra=(); [ "$REBUILD" -eq 1 ] && extra=(--no-cache)
  local log; log="$(mktemp)"
  local reg="${BASE_REGISTRY:-docker.io/library}" tried_fallback=0
  while :; do
    step "イメージをビルド: $tag (BASE_REGISTRY=$reg)"
    if "$ENGINE" build "${extra[@]+"${extra[@]}"}" --build-arg "BASE_REGISTRY=$reg" \
         "$@" -t "$tag" "$(hostpath "$ROOT")" 2>&1 | tee "$log"; then
      rm -f "$log"; return 0
    fi
    if [ "$tried_fallback" -eq 0 ] && [ -z "$BASE_REGISTRY" ] \
       && grep -q "error getting credentials" "$log"; then
      echo
      echo "!! docker.io の認証ヘルパが資格情報を返せずビルドが落ちました。" >&2
      echo "   土台イメージの取得元を $FALLBACK_REGISTRY に替えて作り直します" >&2
      echo "   (恒久的に直すなら ~/.docker/config.json の credsStore を外す。" >&2
      echo "    取得元を固定したいときは IPEMU_BASE_REGISTRY で指定する)" >&2
      reg="$FALLBACK_REGISTRY"; tried_fallback=1; continue
    fi
    rm -f "$log"; return 1
  done
}

build_image() { # build_image <target> <tag>
  run_build "$2" -f docker/Dockerfile.build --target "$1"
}

have_image() { "$ENGINE" image inspect "$1" >/dev/null 2>&1; }

run_in() { # run_in <image> <コマンド...>
  local image="$1"; shift
  # 名前付きボリュームは --mount で渡す: Windows の nerdctl は `-v name:/path` の name を
  # カレントディレクトリからの相対パスと解釈してしまい、リポジトリの中に実体ディレクトリを
  # 作ってしまう (= drvfs 上に置かれて猛烈に遅い)。--mount type=volume なら曖昧さがない
  local mounts=(
    -v "$(hostpath "$ROOT"):/src:ro"
    -v "$(hostpath "$ROOT"):/out"
    --mount "type=volume,source=$WORK_VOL,target=/work"
    --mount "type=volume,source=$CARGO_VOL,target=/usr/local/cargo/registry"
    --mount "type=volume,source=$CACHE_VOL,target=/root/.cache"
  )
  [ -n "$MACOS_SDK" ] && mounts+=(-v "$(hostpath "$MACOS_SDK"):/opt/macos-sdk:ro")
  local tty=(); [ -t 0 ] && [ -t 1 ] && tty=(-it)
  "$ENGINE" run --rm "${tty[@]+"${tty[@]}"}" "${mounts[@]}" "$image" "$@"
}

# イメージが無ければ作る (--rebuild が付いていれば常に作り直す)
ensure_toolchain() {
  { [ "$REBUILD" -eq 0 ] && have_image "$TOOLCHAIN_IMAGE"; } || build_image toolchain "$TOOLCHAIN_IMAGE"
}
ensure_cross() {
  { [ "$REBUILD" -eq 0 ] && have_image "$CROSS_IMAGE"; } || build_image cross "$CROSS_IMAGE"
}

server_build() {
  run_build "$SERVER_IMAGE" --target runtime
}

server_keygen() {
  have_image "$SERVER_IMAGE" || server_build
  mkdir -p "$ROOT/dev/keys"
  if [ -f "$ROOT/dev/keys/signing.json" ]; then
    echo "dev/keys/signing.json は既にあります (作り直すなら消してから)"; return 0
  fi
  step "署名鍵を作る → dev/keys/ (秘密鍵はコミットしない)"
  # イメージの既定ユーザ (uid 10001 = サーバと同じ) で作る。root で作ると signing.json が
  # 0600 root 所有になり、非 root で走るサーバから読めない
  "$ENGINE" run --rm -v "$(hostpath "$ROOT/dev/keys"):/keys" \
    --entrypoint ipemu-server "$SERVER_IMAGE" keygen --out /keys
  ls -l "$ROOT/dev/keys"
}

# コンテナの中から curl を叩く (ホストに curl が無くてもよい。公開ポートにも依存しない)
in_server() { compose exec -T server curl -sS "$@"; }

server_verify() {
  local base=http://127.0.0.1:8080 fails=0
  step "サーバの健全性を確認 (コンテナの中から叩く)"

  local tries=0
  until in_server -f "$base/healthz" >/dev/null 2>&1; do
    tries=$((tries + 1))
    [ "$tries" -lt 30 ] || { echo "healthz が返りません" >&2; compose logs --tail 50 server; return 1; }
    sleep 2
  done

  # expect <期待する HTTP コード> <メソッド> <パス> [curl の追加引数...]
  expect() {
    local want="$1" method="$2" path="$3"; shift 3
    local got
    got="$(in_server -o /dev/null -w '%{http_code}' -X "$method" "$base$path" "$@" 2>/dev/null || echo 000)"
    if [ "$got" = "$want" ]; then
      printf '  ok   %-6s %-34s %s\n' "$method" "$path" "$got"
    else
      printf '  NG   %-6s %-34s %s (期待 %s)\n' "$method" "$path" "$got" "$want"; fails=$((fails + 1))
    fi
  }

  echo "- 公開面"
  expect 200 GET /healthz
  expect 200 GET /                       # ポータル SPA
  expect 401 GET /api/v1/catalog         # API キーが無ければ拒否
  expect 401 GET /api/auth/me            # 未ログイン
  expect 401 GET /api/admin/users        # 管理 API も未ログインでは拒否

  # 初期 admin でログインできるか。パスワードは環境変数 → .env (compose と同じ) →
  # どちらも無ければ起動ログに出た生成値
  local pw="${IPEMU_ADMIN_BOOTSTRAP_PASSWORD:-}"
  if [ -z "$pw" ] && [ -f "$ROOT/.env" ]; then
    pw="$(sed -n 's/^[[:space:]]*IPEMU_ADMIN_BOOTSTRAP_PASSWORD=//p' "$ROOT/.env" | tail -1 | tr -d '"'"'"'\r' || true)"
  fi
  [ -n "$pw" ] || pw="$(compose logs server 2>/dev/null | grep -o '"password":"[^"]*"' | tail -1 | sed 's/.*:"//;s/"$//' || true)"
  if [ -n "$pw" ]; then
    echo "- admin でログイン (セッションは Authorization: Bearer)"
    expect 200 POST /api/auth/login -H 'content-type: application/json' \
      -d "{\"username\":\"admin\",\"password\":\"$pw\"}"
    local token
    token="$(in_server -X POST "$base/api/auth/login" -H 'content-type: application/json' \
      -d "{\"username\":\"admin\",\"password\":\"$pw\"}" 2>/dev/null \
      | sed -n 's/.*"token":"\([^"]*\)".*/\1/p' || true)"
    if [ -n "$token" ]; then
      local hdr="authorization: Bearer $token"
      expect 200 GET /api/auth/me       -H "$hdr"
      expect 200 GET /api/admin/users   -H "$hdr"
      expect 200 GET /api/admin/groups  -H "$hdr"
      expect 200 GET /api/admin/audit   -H "$hdr"
      expect 200 GET /api/v1/keys       -H "$hdr"   # 署名公開鍵 (エンジンの --trusted-key 用)

      # パスワード変更: 今と同じもの (= 初期パスワード) は長さを問わず選べる。
      # 別の値にするなら 10 文字以上。同じものを選んだときは変更待ちが残る
      local json='content-type: application/json'
      expect 200 POST /api/auth/password -H "$hdr" -H "$json" \
        -d "{\"currentPassword\":\"$pw\",\"newPassword\":\"$pw\"}"
      expect 400 POST /api/auth/password -H "$hdr" -H "$json" \
        -d "{\"currentPassword\":\"$pw\",\"newPassword\":\"short\"}"
      expect 401 POST /api/auth/password -H "$hdr" -H "$json" \
        -d "{\"currentPassword\":\"wrong\",\"newPassword\":\"$pw\"}"
      local must
      must="$(in_server "$base/api/auth/me" -H "$hdr" 2>/dev/null \
        | sed -n 's/.*"mustChangePassword":\(true\|false\).*/\1/p' || true)"
      if [ "$must" = true ]; then
        printf '  ok   %-6s %-34s %s\n' GET '(mustChangePassword)' 'true (変更画面が毎回出る)'
      else
        printf '  NG   %-6s %-34s %s\n' GET '(mustChangePassword)' "$must (期待 true)"
        fails=$((fails + 1))
      fi

      expect 200 POST /api/auth/logout  -H "$hdr"
      expect 401 GET /api/auth/me       -H "$hdr"   # ログアウト後は無効
    else
      echo "  NG   ログイン応答から token を取り出せません"; fails=$((fails + 1))
    fi
  else
    echo "- admin のログインは省略 (初期パスワードが分かりません。.env の IPEMU_ADMIN_BOOTSTRAP_PASSWORD を設定して作り直す)"
  fi

  echo
  echo "- 署名鍵とストア"
  in_server -o /dev/null -w '' "$base/healthz" >/dev/null 2>&1
  compose exec -T server sh -c 'ls -l /keys; echo; ls -l /data/store | head' || true
  echo
  compose ps
  if [ "$fails" -gt 0 ]; then
    echo; echo "!! $fails 件が期待と違います" >&2; compose logs --tail 40 server; return 1
  fi
  echo; echo "サーバの確認: すべて期待どおり"
}

case "$CMD" in
  images)        build_image toolchain "$TOOLCHAIN_IMAGE"; build_image cross "$CROSS_IMAGE";;
  artifacts)     ensure_toolchain; run_in "$TOOLCHAIN_IMAGE" artifacts "$@";;
  check)         ensure_toolchain; run_in "$TOOLCHAIN_IMAGE" check "$@";;
  shell)         ensure_toolchain; run_in "$TOOLCHAIN_IMAGE" shell;;
  run)           [ "${1:-}" = "--" ] && shift; ensure_toolchain; run_in "$TOOLCHAIN_IMAGE" "$@";;
  client)        ensure_cross; [ $# -gt 0 ] || set -- --all; run_in "$CROSS_IMAGE" client "$@";;
  server-build)  server_build;;
  server-keygen) server_keygen;;
  server-up)     server_build; server_keygen; step "compose up -d"; compose up -d; compose ps;;
  server-verify) server_verify;;
  server-logs)   compose logs "$@";;
  server-down)   step "compose down"; compose down "$@";;
  all)
    ensure_toolchain; ensure_cross
    run_in "$TOOLCHAIN_IMAGE" artifacts release
    server_build; server_keygen; compose up -d; server_verify
    run_in "$CROSS_IMAGE" client --all
    ;;
  *) echo "不明なコマンド: $CMD (-h で使い方)" >&2; exit 2;;
esac
