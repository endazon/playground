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

`fuel` の `exec_ns` は `round(fuel × nsPerFuel)` です (fuel はその tick 1 回の消費量)。出力は tick ごとに順番待ちの列に入り、
前の tick の出力より先には反映されません。native 形は fuel を数えないので `exec_ns` は常に 0 になり、
`kind: fuel` のまま装着すると「fuel を計測しません」の警告ログを 1 回出します (推定値は作りません。D59)。
native 形で実行時間を模擬したいときは `fixed` を使います。

| cpu-virtual の params | 既定 | 意味 |
| --- | --- | --- |
| `timingModel` | `zero` | 上表 |
| `maxFuelPerTick` | なし (無制限) | `tick` / `irq` 1 回に許す fuel。超えたら trap して FAULT (D57) |
| `maxFuelPerInit` | なし (無制限) | `abi-version` / `init` 1 回に許す fuel。超えたら装着が失敗する |

### ウォッチドッグ (fuel 上限と epoch)
暴走 (無限ループ・予算超過) の歯止めは 2 段です。

| 歯止め | 単位 | 決定性 | 対象 |
| --- | --- | --- | --- |
| **fuel 上限** (`maxFuelPerTick` / `maxFuelPerInit`) | wasm 命令数 | **決定的**。同じ入力なら毎回同じ命令位置・同じ tick で trap | wasm 形。`timingModel` の種類によらず効く |
| epoch interruption (後ろ盾) | 壁時計 (既定 5 s / 呼び出し) | 非決定的。機械の速さと負荷で位置が変わる | wasm 形の全部品とファームウェア。`--debug-friendly` で無制限 |

- 呼び出しのたびに Store の fuel を上限ちょうどに張り直すので、前の tick の消費が次の tick の予算に響きません。
- `tick` の trap は既存の経路で FAULT になります (cpu-virtual がログに `firmware.tick が失敗: … maxFuelPerTick=N …` を出し、
  以後ファームウェアを呼ばない → `step` のエラー → 制御器が FAULT)。
- `init` の trap は装着の失敗です。起動時なら起動不可、ホット差替なら古いファームウェアを残します。
- `irq` は `maxFuelPerTick` に従います。`idle` は cpu-virtual がまだ呼びません (呼ぶようにするときは tick と同じ上限を使う)。
- native 形には fuel も epoch も効きません。上限を書いた manifest で native を装着すると「効かない」旨を 1 回警告します。
- fuel はブレーク中に減らないので、wasm 形を LLDB で止めても fuel 上限は誤って鳴りません (epoch は `--debug-friendly` で切る)。
- 上限の目安は、下の較正表の最大値に余裕を掛けた値です (例: 最大の 10 倍。fw-balance なら `maxFuelPerTick: 30000`、`maxFuelPerInit: 5000000` 程度)。

### nsPerFuel の較正
**1 fuel の意味。** Wasmtime (50) の既定のコスト表で、wasm 命令 1 個 = 1 fuel です。`nop` / `drop` / `block` / `loop` /
`end` / `else` / `return` / `unreachable` は 0、それ以外 (`i32.add` も `f64.div` も `call` も) は一律 1 です。
ホスト関数 (HAL の `pwm-write` など) の中の処理は数えません。コンポーネントの境界で走る wasm 側の処理 (文字列の受け渡しの
`cabi_realloc` など) は数えます。つまり fuel は「実行した wasm 命令の数」であり、命令ごとの重さの違いは表しません。

**このビルドでの実測** (2026-11、Wasmtime 50.0.1。C 版は clang 18.1.3 `-O2`、Rust 版は rustc 1.97.0 `opt-level=3` + fat LTO。
`dev/cartpole.manifest.json` の閉ループ 20 s = 20000 tick を `nsPerFuel: 1` で走らせ、`exec-ns` = fuel として集計。
init は通る最小の `maxFuelPerInit` を二分探索して求めた値):

| fuel | fw-balance-c | fw-balance-rs |
| --- | --- | --- |
| 制御 tick (200 Hz、5 tick に 1 回) 倒立維持中 | 854 (一定) | 1101 (一定) |
| 制御 tick スイングアップ中を含む 平均 / 最大 | 962 / 2536 | 1193 / 2912 |
| 制御しない tick (UART 確認とタイマ) 通常 / 最大 | 113 / 1378 | 70 / 1701 |
| 全 tick の平均 | 283 | 295 |
| `init` (cfg の解釈を含む) | 約 317,000 | 約 525,000 |
| `nsPerFuel: 2.0` (既定 manifest) での倒立中の制御 tick | 1.7 µs | 2.2 µs |

制御しない tick の最大は、自動開始などのモード遷移でログ文字列を組み立てる tick と見ています (tick ごとの内訳は未確認)。値は関係のテスト
(`crates/ipemu-engine/tests/fuel_timing.rs`) では固定しません (コンパイラの版で変わるため)。
同じ条件で壁時計 (ホストの `tick` 呼び出し時間、開発ビルドのエンジン) を 3 回測ると、制御 tick の中央値は 35〜40 µs でほぼ揃う一方、
最大値は 0.28〜4.3 ms と実行ごとに 15 倍ばらつきました。fuel の列は実行ごとに一致します (同テストの決定性の項)。

**目標 MCU への対応付け。** 実機があれば、同じ tick を実機で測って割るのが最も確かです。

```text
nsPerFuel = (実機での tick の実行時間 [ns]) / (同じ tick の fuel)
            例: DWT->CYCCNT で 制御 tick = C サイクル、クロック f [Hz] なら  nsPerFuel = C / f × 1e9 / fuel
```

実機がないときの見積もりは `nsPerFuel ≈ (1 wasm 命令あたりの MCU サイクル) / f_clk` です。
wasm はスタックマシンで `local.get` / `local.set` の多くはレジスタに消えるため、整数演算主体なら 1 wasm 命令 ≈ 0.5〜1.5 サイクルが目安です
(例: Cortex-M4 168 MHz で 1 サイクル ≈ 6 ns → nsPerFuel ≈ 3〜9)。既定 manifest の `nsPerFuel: 2.0` は
FPU 付きの数百 MHz 級 (例: Cortex-M7 480 MHz で 1 命令 ≈ 1 サイクル) に近い値です。

**近似の限界。**
- 命令の重さを区別しない。fw-balance は double で計算するので、倍精度 FPU のない MCU (Cortex-M4F は単精度のみ) では
  `f64.mul` 1 個が数十サイクルのソフトウェア演算になり、fuel の比例は大きく崩れます。`cos` / `tanh` も中身の命令数だけ数えます。
- キャッシュ・パイプライン・フラッシュの待ち・分岐予測・割込みの入口出口は表しません。
- HAL 呼び出しの中 (実機ならレジスタ書き込みや UART ドライバ) は 0 として扱います。
- fuel の値はコンパイラ (clang / rustc) と最適化レベルで変わります。ファームウェアを作り直したら較正表を測り直します。
  同じ wasm と同じ Wasmtime なら、OS や CPU が違っても同じ値です。
- 制御 tick と他の tick で消費が大きく違うので、平均ではなく制御 tick (最大) で較正します。

**なぜ壁時計ではなく fuel か。** 壁時計で tick を測ると、ホスト (x86 / Apple Silicon) の速さ・JIT・負荷・電源管理・
デバッガでの停止がそのまま実行時間に入り、同じ manifest でも実行ごとに軌道が変わります (L4 の決定性が崩れる)。
fuel は wasm 命令の数なので、同じファームウェアなら全 OS・全 CPU で同じ値になり、オーバーランや出力の遅れも再現します。
ホストの速さは目標 MCU の速さとも無関係で、壁時計の値をそのまま使う理由もありません。

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
1. 配布 zip を `dist/ipemu-client-<version>-windows-x64/` に展開する。テンプレートの `exe` は
   そこの `resources/bin/ipemu-engine.exe` を指しているので、これだけで動きます。
   別の場所に置くなら `exe` を実パスに書き換えてください。
   環境変数で指したい場合は `"exe": "${env.IPEMU_ENGINE_EXE}"` にして、**VS を起動する前に**
   設定しておきます (`${env.*}` は VS 自身の環境を見るので、同じファイルの `env` ブロックでは
   効きません。`env` はデバッグ対象プロセスに渡すものです):
   ```cmd
   setx IPEMU_ENGINE_EXE C:\path\to\ipemu-client\resources\bin\ipemu-engine.exe
   ```
2. Visual Studio の「フォルダーを開く」で `firmware/fw-balance-c` を開く (CMake プロジェクトとして認識される)。
   同梱の [`firmware/fw-balance-c/launch.vs.json`](../firmware/fw-balance-c/launch.vs.json) を
   **`.vs/launch.vs.json` にコピー**する (`.vs/` は無視対象なのでコミットされない)。
   構成は 2 つ入っています:
   - `ipemu run (native firmware, 仮想時間)` — 仮想時間で回す。ブレークで止めても仮想時間ごと止まるので、
     再開してもウォッチドッグや保護は鳴りません。`run` に `--debug-friendly` はありません (要りません)
   - `ipemu serve (native firmware, 実時間 + シェル)` — UI シェルから繋いで動かす。実時間ペーシングなので
     `--debug-friendly` を付けます (ブレーク中に溜まった壁時計を捨て、epoch interruption の予算を無制限にする)
3. `src/fw_balance.c` (状態機械と LQR。設定の読み出しは `src/cfg.c`、HAL は `src/ip_hal_native.c`) に
   ブレークポイントを置いて F5。`fw_balance_step` で止めると 1 tick ずつ追えます。
4. 変更 → ビルド → F5 のサイクル。native 形は**エンジンを再起動して**読み直します (ネイティブ DLL の
   プロセス内アンロードは信頼できないためです)。プラントも初期状態に戻ります。

DLL の名前は CMake の `OUTPUT_NAME` / `PREFIX` の設定により **`fw-balance-c.dll`** で、
場所は **`<buildRoot>\bin\`** です。CMakeLists が出力先を `bin/` に固定しているので、
VS の「フォルダーを開く」の既定 (Ninja、単一構成) でも `-G "Visual Studio NN"` (多構成) でも
同じ場所に出ます。PDB は隣に出ます。`--firmware` のスロット名は manifest の `balance` です。
コマンドラインで同じことをするなら:
```cmd
ipemu-engine.exe run --manifest dev/cartpole.manifest.json --no-signature-check ^
    --firmware balance=native:...\build\ninja\bin\fw-balance-c.dll --run-seconds 60
```

### 方法 B: 起動中のエンジンにアタッチ
「デバッグ → プロセスにアタッチ」で `ipemu-engine.exe` を選び、コードの種類を「ネイティブ」にします。
PDB が DLL の隣にあればブレークポイントが効きます。UI から「ファームウェア再読込」を選ぶとエンジンを再起動します。

### 実機で確認したこと (Windows 11 + Visual Studio Community 2026、2026-11-06。Q13)
| 段階 | 結果 |
| --- | --- |
| ネイティブ DLL のビルド | ○ VS 同梱の CMake (4.3.1) + MSVC 14.51 / ジェネレータ `Visual Studio 18 2026`。`fw-balance-c.dll` と PDB が出る |
| エンジンが DLL を読む | ○ Docker で作った `windows-x64` の zip の `ipemu-engine.exe` に `--firmware balance=native:<dll>` で装着できる |
| 閉ループ | ○ `Idle -> SwingUp` (0.51 s) → `SwingUp -> Balance` (6.22 s) → 倒立維持。`driver.fault = 0` / `mcu.overruns = 0` |
| wasm 形との一致 | ○ 最終状態が wasm 形と同一 (`plant.theta = 6.290254954721464`)。trace-hash は異なるが、これは native 形が fuel を数えず `exec-ns = 0` になるため (想定どおり。上の「native 形には fuel も epoch も効きません」) |
| ブレークポイントで止める | **未確認** (対話操作が要るため) |

ビルド時に native 形で 1 回出る警告 (`timingModel=fuel ですが native 形は fuel を計測しません`) は想定どおりです。
**この DLL のビルドだけはコンテナではなくホストで行います** (VS のデバッガが読む PDB を作るのが目的なので、
VS 自身にビルドさせるのが筋です)。配布物の wasm 形は従来どおり `scripts/docker.sh` で作れます。
### `--debug-friendly`
- 実時間ペーシングの追いつきを無効化 (ブレーク中に溜まった壁時計を捨てる)
- epoch interruption の予算を無制限にする (ブレーク中に trap させない)
- デバッガの接続 (`IsDebuggerPresent` / `ptrace` 検出) で自動的に有効化もする

## M1 での実装状況
- 両版とも実装済みで、同じ manifest (`dev/cartpole.manifest.json`) でスイングアップ → 倒立維持まで動く。
  wasm / native の 4 形態すべてでプラントの軌道がビット一致する (結合テスト `c_and_rust_firmware_produce_equivalent_trajectories`)。
- C 版の wasm は **freestanding** (libc なし) でビルドし、数学関数は `crates/ip-cmath` (Rust の `libm` を C ABI で公開) をリンクする。
  wasi-sdk が不要で、WASI を import しない。詳細は [`../firmware/fw-balance-c/README.md`](../firmware/fw-balance-c/README.md)。
- cfg JSON は**フラット** (入れ子なし) にして、C 版は極小パーサ (`cfg.c`)、Rust 版も同じ規則の `cfg.rs` で読む。
  数値は 15 桁以内で書けば両版で同じ double になる (`ipemu-engine lqr` はそう出力する)。
- ドライブへの「指令タイムアウト」は、FW が制御周期ごとに反転する **ハートビート (gpio-out1)** で表現する。
  ハートビートが止まると motor-driver が遮断する (通信断・FW 停止の模擬)。
- オペレータ操作は UART0 の 1 バイトコマンド (`s` スイングアップ / `b` 倒立 / `x` 停止 / `c` 異常リセット)。
  M1 の headless では `autoStart` パラメータで自動開始する。
- fuel 計測は wasm 形のみ (`timingModel: fuel` で `exec-ns` に出る)。native 形は 0。M6 で較正の指針と fuel 上限を追加した (上記)。

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
