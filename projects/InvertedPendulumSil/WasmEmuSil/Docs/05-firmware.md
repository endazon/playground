# 05. 仮想 MCU とファームウェア注入

## 仮想 MCU (`builtin:cpu-virtual`)
エンジンが実装する組み込み部品です。「HAL 付きの関数呼び出しモデル」であり、命令セットは持ちません。

```text
 ┌ builtin:cpu-virtual ──────────────────────────────────────────────┐
 │ ポート (周辺)                                                      │
 │   in : enc0, enc1 (累積カウント), gpio-in[0..7], uart0-rx (bytes)   │
 │   out: pwm0, pwm1 (duty -1..1), dac0, gpio-out[0..7], uart0-tx      │
 │                                                                    │
 │ tickHz (例 1000) ごとに:                                            │
 │   1. 入力ポート → HAL レジスタ像へ転記                               │
 │   2. firmware.tick()  (割込みハンドラ相当。単一スレッド)             │
 │   3. HAL レジスタ像 → 出力ポートへ転記                               │
 │   4. 実行時間モデルに従って「CPU が次に空く仮想時刻」を更新           │
 │ irq: エンコーダ Z 相、UART RX、GPIO エッジを firmware.irq(id) で通知 │
 └────────────────────────────────────────────────────────────────────┘
```

### 実行時間モデル
実機では制御計算に時間がかかり、PWM が更新されるのは tick 開始から数十〜数百 µs 後です。これを模擬します。

| `timingModel.kind` | 仮想実行時間の決め方 | 対象 |
| --- | --- | --- |
| `zero` | 0 (理想) | 既定。テストの比較基準 |
| `fixed` | `tick` ごとに `fixedNs` | ネイティブ形でも使える |
| `fuel` | Wasmtime の fuel 消費量 × `nsPerFuel` | wasm 形のみ。命令数に比例した近似 |

実行時間ぶん出力の反映を遅らせます (tick 開始から `exec_ns` 後のステップで出力ポートへ書く)。
`exec_ns` が `1/tickHz` を超えたら **オーバーラン**としてカウントし、UI に出します。

## ファームウェアの形
| 形 | ファイル | 用途 | 実行 | デバッガ |
| --- | --- | --- | --- | --- |
| **wasm (正)** | `fw-balance.wasm` (コンポーネント) | 配布。全 OS で同一 | Wasmtime (fuel 可) | VS Code + CodeLLDB (DWARF)、全 OS |
| native (第 2) | `fw_balance.dll` / `.so` / `.dylib` | 開発・デバッグ | `libloading` でエンジンに読み込む | **Visual Studio** (Windows)、LLDB / GDB |

同じソースから両方をビルドします。HAL の境界は WIT の `ip:emu/hal` (wasm) と `abi/ip_hal.h` (native) で、
**関数名・引数・意味は 1 対 1** です。

v1 では **C と Rust の 2 本**を同じ仕様で実装します。HAL 境界が特定言語のバインディングに寄っていないことを
初回から確かめるためで、2 本は同じゴールデントレースを通さなければなりません。

```text
firmware/fw-balance-c/                    ← ip_hal.h だけを include
  src/ fw_main.c  controller.c  lqr_gain.h …
  CMakeLists.txt
    target wasm   : clang --target=wasm32-wasip2 (wasi-sdk) + wit-bindgen c → fw-balance.wasm
    target native : cl / clang / gcc → fw_balance.{dll,so,dylib}  (PDB / DWARF 付き)
    IP_HAL_TARGET_WASM / IP_HAL_TARGET_NATIVE でバックエンドを切替
firmware/fw-balance-rs/                   ← ip-hal クレート (wit-bindgen 生成 + native 用 vtable 実装) だけに依存
  Cargo.toml : crate-type = ["cdylib"] + target wasm32-wasip2
  src/ lib.rs  controller.rs …
```

### HAL (抜粋。全文は [`../abi/ip_hal.h`](../abi/ip_hal.h))
| 関数 | 意味 |
| --- | --- |
| `ip_hal_micros()` | 仮想時刻 [µs]。壁時計ではない |
| `ip_hal_enc_read(ch)` | 累積カウント (int32、ラップあり) |
| `ip_hal_pwm_write(ch, duty)` | -1.0〜1.0。範囲外は飽和 |
| `ip_hal_gpio_read / write` | デジタル入出力 |
| `ip_hal_uart_write / read` | バイト列 (comm-link や UI コンソールへ) |
| `ip_hal_log(level, msg)` | 診断ログ。エンジンがトレースとして UI / ファイルへ |
| `ip_hal_trace(id, value)` | 数値トレース (スコープに載せられる) |

TCP・ファイルなど HAL 外の資源は、wasm 形では **WASI preview 2 の import** として、manifest の capability 宣言と承認・同意を経た場合にのみ使えます。
ネイティブ形には制限をかけられないため、**ネイティブ形は開発環境専用**とし、配布用 manifest では `allowOverride` に `native` を含めません。

### エントリポイント
| wasm (`ip:emu/firmware` world の export) | native (C シンボル) | 役割 |
| --- | --- | --- |
| `abi-version() -> u32` | `ip_fw_abi_version` | 互換性確認 |
| `init(cfg-json) -> result` | `ip_fw_init(const char*, size_t)` | パラメータ受領。ネイティブ形はここで HAL の関数テーブルも受け取る |
| `tick()` | `ip_fw_tick` | 周期割込み |
| `irq(id)` | `ip_fw_irq(uint32_t)` | 非周期割込み |
| `idle()` (任意) | `ip_fw_idle` | メインループ 1 回分。tick の残り時間で呼ぶ (timingModel が zero 以外のとき) |

ファームウェアは**自分でループしません**。`tick` から戻らないと仮想世界が止まります
(epoch interruption で trap させ、FAULT として表示します)。

## Visual Studio でのデバッグ実行 (Windows)
前提: Visual Studio 2022 以降 + 「C++ によるデスクトップ開発」。エンジンは同梱の `ipemu-engine.exe` をそのまま使い、
Rust のビルド環境は不要です (エンジン自体をデバッグする場合だけ Rust が要ります)。

### 方法 A: F5 で起動 (推奨)
1. `firmware/fw-balance` を CMake プロジェクトとして開く (または生成した `.vcxproj` をソリューションに含める)。
2. `launch.vs.json` (CMake) またはプロジェクトの「デバッグ」設定で、起動するプログラムを `ipemu-engine.exe` にする。
   ```jsonc
   // firmware/fw-balance/.vs/launch.vs.json (テンプレートをリポジトリに同梱する)
   {
     "version": "0.2.1",
     "configurations": [{
       "type": "default", "name": "ipemu (native firmware)",
       "project": "CMakeLists.txt", "projectTarget": "fw_balance.dll",
       "exe": "${env.IPEMU_HOME}/ipemu-engine.exe",
       "args": ["--manifest", "${workspaceRoot}/../../dev/cartpole.manifest.json",
                "--firmware", "balance=native:${cmake.buildRoot}/fw_balance.dll",
                "--no-signature-check", "--debug-friendly", "--shell", "auto"]
     }]
   }
   ```
3. `controller.c` にブレークポイントを置いて F5。エンジンが起動し、`--shell auto` で UI シェルも立ち上がります。
   ブレークで止まると**仮想時間ごと止まる**ので、再開してもウォッチドッグや保護は鳴りません。
4. 変更 → ビルド → F5 のサイクル。native 形は**エンジンを再起動して**読み直します (ネイティブ DLL の
   プロセス内アンロードは信頼できないためです)。プラントも初期状態に戻ります。

### 方法 B: 起動中のエンジンにアタッチ
「デバッグ → プロセスにアタッチ」で `ipemu-engine.exe` を選び、コードの種類を「ネイティブ」にします。
PDB が DLL の隣にあればブレークポイントが効きます。UI から「ファームウェア再読込」を選ぶとエンジンを再起動します。

### `--debug-friendly`
- 実時間ペーシングの追いつきを無効化 (ブレーク中に溜まった壁時計を捨てる)
- epoch interruption の予算を無制限にする (ブレーク中に trap させない)
- デバッガの接続 (`IsDebuggerPresent` / `ptrace` 検出) で自動的に有効化もする

## リビルド後の反映 (ホットリロード)
| 形 | `firmware.load` の挙動 | プラント状態 |
| --- | --- | --- |
| wasm | 実行中に slot だけ差し替える。古い `Store` を破棄し、新しいコンポーネントを `init` して次 tick から呼ぶ | **保持** (倒立中に差し替えると、そのまま制御が引き継がれる) |
| native | エンジンを再起動する (シェルが自動で spawn し直し、同じ manifest と上書きで再 bootstrap) | 初期状態 |

挙動が違うので、UI の「ファームウェア再読込」は形ごとにラベルを変え (「ホット差替」/「再起動して差替」)、
`--watch` 指定時はビルド成果物の更新を検知して同じ処理を自動で行います。
wasm のホット差替は、新しいファームウェアの `init` が失敗したら古い slot を残して FAULT にせず警告だけ出します。

## VS Code + LLDB (全 OS、wasm 形も可)
- **ネイティブ形**: CodeLLDB の `attach` / `launch` で `ipemu-engine` を対象にする。方法 A と同じ引数。
- **wasm 形**: エンジンを `--wasm-debug-info` で起動すると Wasmtime が DWARF を JIT デバッガインターフェースに登録し、
  LLDB からソース行でブレークできます。最適化なし (`-O0 -g`) でビルドした wasm が必要です。
  Visual Studio 本体は Wasmtime 上の wasm をデバッグできません。これが native 形を残す理由です。

## ファームウェア側のテスト
HAL をモックすれば、ファームウェアはエンジンなしで単体テストできます (`tests/hal_mock.c`)。
さらに `ipemu-engine --headless --run-seconds 10 --record out.csv` で閉ループの受入テストができ、
NetworkedSil の `SilHarness` に相当する役割をエンジンの headless モードが担います。
