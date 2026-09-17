# 04. ビルド・実行・テスト

## 前提
このプロジェクトはリポジトリのルートではなく `projects/InvertedPendulumSil/` に置かれています。
**以下のコマンドはすべてこのプロジェクトディレクトリを起点**に実行してください
(`Dockerfile` / `compose.yaml` / `InvertedPendulumSil.slnx` がここを基準にしているため)。

```bash
cd projects/InvertedPendulumSil
```

リポジトリルートから直接動かす場合は `--project` のパスを読み替えます。

```bash
dotnet run --project projects/InvertedPendulumSil/src/Ip.Server
```

必要なもの:

- .NET 10 SDK (SDK なしで動かす場合は Docker のみで完結します)
- Docker / Docker Compose (コンテナで動かす場合)
- Node.js (Playwright をローカルで動かす場合。コンテナ実行なら不要)

## 実行

### .NET SDK で動かす
```bash
dotnet run --project src/Ip.Server
# http://localhost:5210 を開く
```
`Ip.Server` が Blazor WASM クライアントをホストするので、起動するプロセスは 1 つだけです。

### Docker で動かす
```bash
docker compose up --build      # http://localhost:5210
```
`compose.yaml` はループバック (`127.0.0.1:5210`) にだけ公開します。
学習・実験用であり LAN に晒す必要がないためです。
`e2e` サービスは compose ネットワーク内の `app:8080` に直接つなぐので影響を受けません。

`Dockerfile` は 3 つのターゲットを持ちます。

| ターゲット | 用途 |
| --- | --- |
| `runtime` (既定) | publish 済みのアプリを `mcr.microsoft.com/dotnet/aspnet` 上で動かす。非 root 実行、`/healthz` のヘルスチェック付き |
| `build` | Blazor WASM クライアントごと `dotnet publish` する中間段 |
| `test` | `docker build --target test .` で .NET のテストを全部走らせる |

`restore` 段は csproj だけを先に COPY するので、ソース変更では restore 層を再実行しません。

### 企業プロキシ下でのビルド
TLS を傍受する企業プロキシの下でビルドする場合は、その CA 証明書 (PEM 形式) を
`docker/ca/*.crt` に置いてください (`docker/ca/README.md`)。
ビルドの最初の段で信頼ストアに取り込まれます。証明書自体はコミットされません。

## テスト
4 つの層に分かれています。下に行くほど本物に近く、遅くなります。

```bash
dotnet test                                    # 1) 単体 + 2) 仮想時間の結合 + 3) 実時間の結合
cd e2e && npm ci && npx playwright test        # 4) ブラウザ結合 (Playwright)
docker compose --profile e2e up --build \
  --abort-on-container-exit --exit-code-from e2e   # 4) をコンテナ同士で実行
```

| 層 | 場所 | 何を確かめるか | 時間の扱い |
| --- | --- | --- | --- |
| 単体 | `tests/Ip.Shared.Tests` | 行列指数・ZOH 離散化・離散 LQR・非線形モデルのエネルギー保存・遅延余裕の表・受信値の矯正・クロック同期の式 | なし |
| 仮想時間の結合 | `tests/Ip.Shared.Tests/SilHarness.cs` | プラント ↔ 遅延線 ↔ コントローラをロックステップで回し、遅延・ジッタ・スパイク・ノイズと倒立維持の境界を再現性のある形で検証 | 仮想 |
| 実時間の結合 | `tests/Ip.Server.Tests` | 実際の SignalR + MessagePack を通して、サーバの制御ループが 5ms 周期に追従できるか。セッションの解放と分離 | 実時間 |
| ブラウザ結合 | `e2e/` | 実 Chromium で Blazor WASM を起動し、操作・表示・保護動作・遅延の効き方を確認 | 実時間 |

### 層ごとの役割分担
量的な主張 (どこまでの遅延なら倒立を維持できるか) は**仮想時間のテストが持ちます**。
ブラウザ側は「設定がバックエンドまで届き、表示が実態と一致する」ことだけを見ます。

実時間で境界ぎりぎりを試すと、共有 CI の負荷で結果が揺れて
「制御が悪いのか環境が遅いのか」を区別できないテストになるためです。

### Playwright の動かし方
`BASE_URL` が設定されていなければ、Playwright が自前で `dotnet run` してアプリを立ち上げます。
`BASE_URL` を渡せば、すでに動いているアプリ (compose の `app` サービスなど) を相手にします。

> プラントは実時間で動くため、E2E は必ず 1 ワーカーで直列に実行します (`workers: 1`)。
> 並列にすると CPU の奪い合いで制御ループの追従が乱れ、
> 「遅延のせいで倒れた」のか「テスト環境が遅かった」のか区別できなくなります。

## CI の組み方
CI は本リポジトリではまだ有効化していません。組む場合は次を目安にしてください。

### 選択肢
| 方針 | コマンド | 向き / 不向き |
| --- | --- | --- |
| 全テスト (Docker) | `docker build --target test .` | 環境差が出ない。ただし実時間の結合テストを含むため、CPU を絞った共有ランナーでは揺れうる |
| 全テスト (SDK) | `dotnet test` | 速い。ランナーに .NET 10 SDK が要る。同じく実時間テストの揺れを受ける |
| 安定層のみ | `dotnet test tests/Ip.Shared.Tests` | **共有ランナーではこれを推奨**。単体 + 仮想時間の結合だけなので負荷に左右されない |
| ブラウザ結合 | `docker compose --profile e2e up --build --abort-on-container-exit --exit-code-from e2e` | 実 Chromium まで通す。専有ランナーか、失敗時に再実行する運用とセットで |

遅延余裕の表 (`DelayMarginTests`) と仮想時間の結合テストは `tests/Ip.Shared.Tests` にあるので、
**「安定層のみ」でも本プロジェクトの主張は検証されます**。

### GitHub Actions の例 (安定層のみ)
そのまま `.github/workflows/` に置けます。
`paths` を絞ってあるので、このプロジェクト以外の変更では起動しません。

```yaml
name: inverted-pendulum-sil

on:
  push:
    paths: ["projects/InvertedPendulumSil/**", ".github/workflows/inverted-pendulum-sil.yml"]
  pull_request:
    paths: ["projects/InvertedPendulumSil/**", ".github/workflows/inverted-pendulum-sil.yml"]

jobs:
  test:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: projects/InvertedPendulumSil
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-dotnet@v4
        with:
          dotnet-version: "10.0.x"
      # 共有ランナーの負荷に左右されない層だけを走らせる。
      # 実時間の結合テスト (tests/Ip.Server.Tests) と e2e は専有ランナー向け。
      - run: dotnet test tests/Ip.Shared.Tests -c Release
```

## ビルド成果物の除外
`projects/InvertedPendulumSil/.gitignore` が .NET の `bin/` `obj/` と
Playwright の生成物を除外します。
リポジトリルートの `.gitignore` に .NET 向けの記述がないため、
プロジェクト単位で持たせています (`.dockerignore` はビルドコンテキスト用で別物です)。
