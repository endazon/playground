# 台車型倒立振子 SIL — 2 つの実現形

同じ「台車型倒立振子を SIL (Software In the Loop) で回す」課題を、
**制御ループをどこに置くか**が異なる 2 つの形で実現します。

| ディレクトリ | 形 | 制御ループの置き場所 | 状態 |
| --- | --- | --- | --- |
| [`NetworkedSil/`](NetworkedSil/README.md) | 通信路をまたぐ純 SIL | バックエンド (ASP.NET Core)。プラントはブラウザ (Blazor WASM)、間は SignalR | 実装済み。遅延余裕の結論あり |
| [`WasmEmuSil/`](WasmEmuSil/README.md) | WASM 部品で組むクライアント側エミュレータ | クライアント内の仮想 MCU。制御プログラムは DLL / WASM で外部から注入 | **設計中** (草案) |

## 2 つの関係
`NetworkedSil` の結論は「安定化ループをバックエンドに置く設計は LAN 内の長い振子でぎりぎり。
実機では安定化をドライブ (組み込み側) に置き、バックは目標値とモード管理に専念する階層型にすべき」
でした ([NetworkedSil/Docs/03-delay-margin.md](NetworkedSil/Docs/03-delay-margin.md))。

`WasmEmuSil` はその階層型を前提にした次の段です。
安定化ループは仮想 MCU の中で閉じ、バックエンド (サーバアプリ) は
**構成情報と部品 (WASM) の配信・管理**に役割を変えます。
制御ループの内側に通信路が入らないので、遅延の実験台ではなく
**組み込みファームウェアの開発・デバッグ台**として使えるのが狙いです。

```text
NetworkedSil                           WasmEmuSil
┌────────────┐                         ┌──────────────┐
│ サーバ      │ ← 制御ループ (LQR)     │ サーバ        │ ← 構成情報 + 部品 WASM の配信 / 管理画面
└─────┬──────┘                         └──────┬───────┘
      │ SignalR (ループ内)                     │ HTTPS (起動時のみ)
┌─────┴──────┐                         ┌──────┴───────┐
│ ブラウザ    │ ← プラント             │ クライアント   │ ← エミュレータ全体 (仮想 MCU + プラント + 周辺)
└────────────┘                         │  Chromium: UI │    制御プログラムは DLL / WASM を注入
                                       │  Wasmtime: 実行│    Visual Studio でデバッグ可能
                                       └──────────────┘
```

## 共有するもの
物理モデル・LQR・遅延余裕の計算 (`NetworkedSil/src/Ip.Shared`) は実時間にも UI にも依存しない純粋な C# です。
`WasmEmuSil` では部品の**参照実装 (オラクル)** として使い、
WASM 化した部品がこれと同じ軌道を出すことをテストで確かめます
([WasmEmuSil/Docs/08-decisions.md](WasmEmuSil/Docs/08-decisions.md))。
