# 03. 部品モデル (WIT・ポート・配線・スケジューラ)

## 部品とは
エミュレータを構成する最小単位です。1 部品 = 1 つの WebAssembly **コンポーネント** (Component Model) で、
WIT パッケージ `ip:emu` の `part` ワールドを実装します。定義は [`../abi/wit/`](../abi/wit/) にあります。

```text
          params (JSON, 起動時に 1 回)
              │
   inputs ──▶ │ ┌──────────────┐ │ ──▶ outputs
   (f64[] +   │ │   part       │ │     (f64[] + bytes ports)
    bytes)    │ │  step(dt_ns) │ │
              │ └──────────────┘ │
              │  import: ip:emu/host (now-ns, log, trace)
              │  import: wasi:* (capability として承認されたものだけ)
```

| 要素 | 内容 |
| --- | --- |
| `describe()` | 部品名・版・入出力ポート・パラメータ・必要 capability を返す。エンジンは manifest と突き合わせる |
| `init(params-json)` | パラメータを受け取って内部状態を作る。失敗は `Err(string)` |
| `reset()` | 初期状態へ戻す。`Epoch` (リセット世代) の管理はエンジン側 |
| `set-inputs(list<f64>)` / `get-outputs() -> list<f64>` | スカラーポートを**一括**で読み書きする (ステップあたり往復 1 回ずつ) |
| `write-bytes(port, list<u8>)` / `read-bytes(port) -> list<u8>` | バイト列ポート (UART 相当) |
| `step(dt-ns)` | 仮想時間を `dt` だけ進める。失敗は `Err` → エンジンは FAULT 停止 |
| `snapshot() -> list<u8>` / `restore(list<u8>)` | 状態の保存・復元 (巻き戻し、テストの途中開始用)。任意 |

### 実装 (M1)
- 部品は Rust で書き、`crates/ip-part-sdk` の `PartImpl` トレイトと `part_entry!` マクロで WIT 境界を生やす。
- ビルドは `wasm32-unknown-unknown` → `wasm-tools component new`。WASI を一切 import しないので adapter が要らない。
  `cargo component` や `wasm32-wasip2` を使わないのは、std 経由で `wasi:cli` 等の import が混ざるのを避けるため (決定性と V7 の前提)。
- bytes ポートの配線は v1 では未対応 (`GraphError::BytesPortUnsupported`)。UART は仮想 MCU の内部で扱う。
- `encoder` の入力ポートは `input` (単位なし)。mode (angular / linear) で意味が変わるため単位検査の対象外にしている。

### なぜ Component Model (WIT) か
- 型付きのインターフェースを**バイナリから読める** (`wasm-tools component wit`)。
  サーバは部品をアップロードされた時点で import / export を検査でき、manifest の capability 宣言と突き合わせられます。
- **WASI preview 2 の capability 付与と一体**です。ソケット・ファイルは import の有無とホスト側の `WasiCtx` の両方で制御できます。
- 部品・ファームウェアを C / C++ / Rust のどれで書いても `wit-bindgen` が同じ境界を生成します。
- 代替のフラット C ABI (コア モジュール + 線形メモリ直読み) は .NET ホストなら必要でしたが、エンジンを Rust にしたので不要です
  ([08-decisions.md](08-decisions.md))。

### 呼び出しコストの見積り
1ms ステップ、部品 5 個、各部品あたり `set-inputs` / `step` / `get-outputs` の 3 呼び出しで 15k 呼び出し/秒。
Component Model のホスト呼び出しは 1 回あたりサブマイクロ秒〜数マイクロ秒なので、実時間の 1〜5% 程度です。
ポートごとの個別呼び出しにしないのはこのためです。

## ポート
| 種別 | 型 | 用途 |
| --- | --- | --- |
| scalar | `f64` | 連続量 (位置、速度、電圧、デューティ)。整数や真偽も f64 に載せる (パルス数、E-STOP) |
| bytes | `list<u8>` | UART 相当のストリーム。ステップごとにキューを転記する |

ポートには `unit` (SI 文字列) と `doc` を付け、manifest 検証で**単位の不一致を警告**します
(例: `rad` の出力を `deg` の入力に配線)。

## 配線 (wires)
```json
{ "from": "plant.theta", "to": "encoder.angle", "latency": 1 }
```
- `latency: 1` (既定): ステップ k の出力を、ステップ k+1 の入力として転記する。
  これだけで構成すれば**実行順に依存せず**、どんな閉路も許される。
- `latency: 0`: 同一ステップ内で、送り手を先に `step` してから受け手へ転記する。
  実行順に制約がつくので、エンジンは latency 0 の配線だけで**閉路ができないこと**を検証する (できれば manifest 拒否)。
  典型的な用途は「センサ → MCU の同一ステップ読み取り」。

実行順は latency 0 の辺でトポロジカルソートし、同順位は manifest の記載順で安定化します。
未配線の入力はポートの `default` (describe が返す) になります。

## 組み込み部品 (builtin)
WASM ではなくエンジン本体が実装する部品です。manifest では `"kind": "builtin:<name>"` で指定します。

| 名前 | 役割 |
| --- | --- |
| `builtin:cpu-virtual` | 仮想 MCU。周辺をポートとして持ち、注入されたファームウェアを `tickHz` で呼ぶ。詳細は [05-firmware.md](05-firmware.md) |
| `builtin:snapshot` | 指定ポートを UI 向けスナップショットに載せる (50Hz 既定) |
| `builtin:recorder` | 指定ポートを CSV / Parquet に記録する (headless 用) |
| `builtin:scope` | 高レートのリングバッファ。UI のスコープに流す |

組み込み部品にするのは「エンジンの内部構造 (スケジューラ・IPC・ファームウェアローダ) に触る必要があるもの」だけです。
物理や保護ロジックは組み込みにしません。差し替え可能であることが WASM 部品にする理由だからです。

## 標準部品 (v1 で用意するもの)
| 部品 | 入力 | 出力 | 中身 |
| --- | --- | --- | --- |
| `plant-cartpole` | `voltage` [V], `estop` | `x`, `theta`, `x-dot`, `theta-dot`, `force`, `hit-stopper` | 台車+振子の非線形モデル、DC モータ (逆起電力)、メカストッパ。1ms RK4。`ip-model` クレート。任意の `pushes` で周期外乱 (振子軸トルクのパルス、NetworkedSil の `Push` 相当、D62) |
| `motor-driver` | `duty` [-1..1], `enable`, `heartbeat`, `estop-in`, `position` [m] | `voltage` [V], `fault`, `drive-enabled`, `rejected` | PWM → 電圧、保護 (E-STOP・メカリミット・ハートビート断 = 指令タイムアウト・無効中の指令拒否)。復帰は `enable` の立ち上がり |
| `encoder` | `input` (角度 [rad] か位置 [m]、mode で決まる) | `count` (累積, 32bit ラップ), `delta` | 量子化 (PPR)。NetworkedSil の `ReadEncoders` と同じ丸め |
| `comm-link` | `in` (scalar) | `out`, `in-flight`, `spikes`, `hol-blocked` | 遅延・ジッタ・再送スパイク・追い越し禁止。シード付き PRNG で決定的。NetworkedSil の `DelayLine` を部品化。制御ループの外側 (遠隔監視・目標値) にも、M6 では**制御ループの中** (MCU ↔ 駆動系、`dev/cartpole-netdelay.manifest.json`、D60 / D61) にも置く |

NetworkedSil の `VirtualPlant` は「プラント + エンコーダ + ドライブ保護」を 1 つの型に持っていました。
ここでは 3 部品に分けます。manifest で差し替えや挿入 (例: モータ特性だけ変える) ができるようにするためです。

## スケジューラ
```text
loop:
  drain(control_queue)                        // 操作はステップ境界で反映
  for inst in order:                          // latency-0 の辺でトポロジカル順
      if tick % inst.rate_divider != 0: continue
      inst.set_inputs(input_buf[inst])        // latency-1 分は前ステップで転記済み
      inst.step(base_step_ns)?                // Err → FAULT 停止、UI へ通知
      outputs = inst.get_outputs()
      for w in wires_from(inst):
          if w.latency == 0: input_buf[w.to] = outputs[w.from]   // 同一ステップで後続へ
          else:              next_buf[w.to]  = outputs[w.from]   // 次ステップへ
  swap(input_buf, next_buf)
  t_ns += base_step_ns; tick += 1
  if tick % snapshot_divider == 0: publish(snapshot)
  pace()                                      // realtime のときだけ待つ
```
- **単一スレッド。** 複数の `Store` を持ちますが、すべて scheduler スレッドからしか触りません。
- **epoch interruption** を全部品に設定し、`step` が仮想 1 ステップぶんの予算 (例: 壁時計 50ms) を超えたら trap させます。
  無限ループした部品で UI まで固まるのを防ぎます。
- **fuel** はファームウェア slot だけに使い、消費量から仮想実行時間を見積もります ([05-firmware.md](05-firmware.md))。

## 決定性
同じ manifest・同じファームウェア・同じ操作列なら、OS や CPU が違っても同じ軌道を出すことを目標にします。

| 要因 | 対処 |
| --- | --- |
| 浮動小数点の NaN ビットパターン | Wasmtime の `cranelift_nan_canonicalization` を有効にする |
| 壁時計・乱数 | 既定で WASI の clocks / random を与えない。乱数が要る部品はシード付き PRNG をパラメータで受ける |
| スレッド | 部品に threads を与えない |
| ホストの浮動小数点 | エンジン自身は積分しない (転記するだけ)。ホスト側の算術は整数のみ |
| ネイティブ形のファームウェア | x87 / FMA 等の差で wasm 形と最終ビットが一致しないことがある。**ネイティブ形は決定性の保証外**と明記する |

検証はゴールデントレースのハッシュ比較で行います ([09-verification.md](09-verification.md))。
