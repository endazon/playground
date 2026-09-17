# SignalSimulator

名前・グループ・コメント付きの「信号」オブジェクトを C++ で生成・周期更新し、その値を Qt のダイアログで監視するための実験用プロジェクト。単位（SI 接頭辞）付き数値ライブラリと科学定数を土台にしている。Windows / MSVC 専用。

移管元のリポジトリ名は `Training`。中身に合わせて `SignalSimulator` に改名した（CMake の project 名などは `Training` のまま）。

## 概要

### Target/include（ヘッダオンリー）

| ファイル | 内容 |
| --- | --- |
| `UnitOfNumber.hpp` | 数値ラッパー（`NumericEntity` / `NumeralOperators` / `Numeral`）と SI 接頭辞 `SIPrefix`（Q 〜 q の 25 種、単位変換） |
| `ScientificPostulates.hpp` | 科学定数（光速、重力加速度、プランク定数、アボガドロ数など 20 余り） |
| `Simulator.hpp` | `BaseSimulator`: 生成・破棄時に一覧へ登録／解除し、`ISignalInstanceUpdateFunction` で GUI に通知。`BaseTimeSimulator`: 専用スレッドで 100µs 〜 1000ms の 13 周期の `UpdateInXXXCycle()` を呼ぶ |
| `GeneralPurposeTimer.hpp` | 精度別の時間計測（`clock()` / `<chrono>` / `QueryPerformanceCounter`）、周期経過判定、`DateFormat::UTC` による日時文字列化 |
| `Utility.hpp` | `DLLLoader`: `LoadLibrary` / `GetProcAddress` のラッパー |

### 実行ファイル・ライブラリ

- **DebuggingConsole**: `QApplication.dll` と `SimulatorListDialog.dll` を実行時に読み込み、信号の生成・削除・値更新を一覧ダイアログで確認するデモ。
- **QtDebuggingDialog**: Qt 製デバッグ GUI。
  - `QSimulatorList/`: 信号一覧ダイアログ（シリアル番号・Name・Group・Comment・Value）。DLL として `InstanceCreation` / `InstanceDestroyed` をエクスポート。
  - `QCommunicationHistoryList/`: 通信履歴ダイアログ（時刻・種別・メッセージ）。
  - `QApplication/`: `QApplication` を DLL 越しに生成するラッパー。
  - `QtDebuggingDialog.cpp` ほか: 2 つのダイアログを表示するテストアプリ。
- **UnitTest**: googletest による単体テスト（数値演算子、SI 接頭辞変換、処理時間計測、科学定数）。googletest は `cmake/DownloadProject` で configure 時に取得する。
- **BinaryEditorBz**: バイナリエディタ Bz をサブモジュールとして取り込み、`original/Bz` をビルドするだけの CMake。

## 技術スタック

- C++20 / CMake 3.8 以上 / Ninja
- Qt 6（Core / Gui / Widgets、AUTOMOC / AUTOUIC / AUTORCC）
- MSVC x64（`CMakeSettings.json` に `x64-Debug` / `x64-Release`）
- googletest（UnitTest のみ）

## ディレクトリ構成

```text
CMakeLists.txt             ルート（Target / DebuggingConsole / QtDebuggingDialog を有効化）
CMakeSettings.json         Visual Studio 用構成
toolchain.cmake            Qt・zlib・WTL のパスと CRT（/MD）設定。作者環境の絶対パス
Target/include/            コアのヘッダ
DebuggingConsole/          デモ exe
QtDebuggingDialog/         Qt デバッグ GUI（QApplication / QSimulatorList / QCommunicationHistoryList）
UnitTest/                  googletest
BinaryEditorBz/original/   サブモジュール（gitlab.com/devill.tamachan/binaryeditorbz）
cmake/DownloadProject/     サブモジュール（github.com/Crascit/DownloadProject）
```

## ビルド

1. `toolchain.cmake` の `QTDIR` / `QT_LIBRARY_DIR` などを自分の環境に合わせて書き換える。
2. Visual Studio の「フォルダーを開く」で `projects/SignalSimulator` を開き、`CMakeSettings.json` の構成でビルドする。コマンドラインなら MSVC x64 開発者プロンプトで次を実行する。

   ```bash
   cmake -S . -B out/build -G Ninja -DCMAKE_TOOLCHAIN_FILE=toolchain.cmake -DQT_BUILD_SHARED_LIBS=ON
   ```

3. UnitTest / BinaryEditorBz はルート `CMakeLists.txt` でコメントアウトされている。有効にする場合はサブモジュールを取得する（リポジトリルートで実行）。

   ```bash
   git submodule update --init projects/SignalSimulator/cmake/DownloadProject projects/SignalSimulator/BinaryEditorBz/original
   ```

DebuggingConsole が読み込む DLL は別ディレクトリに出力されるため、実行時にコピーが必要になる可能性がある。移管時点ではビルドを確認していない。

## 注意点・未完成部分

- ソースの文字コードが混在している。Shift-JIS: `UnitOfNumber.hpp`、`ScientificPostulates.hpp`、`Simulator.hpp`、`Utility.hpp`、`UnitTest.cpp`、`CommunicationHistoryListDialog.cpp`、`toolchain.cmake`。その他は UTF-8（BOM 付き）。
- Windows 専用（`windows.h`、`LoadLibrary`、`__declspec`）で、`toolchain.cmake` は作者 PC の絶対パスに依存する。
- `GeneralPurposeTimer.hpp` の `DateFormat` で `sprintf_s` が `assert()` 内にあり、Release ビルドでは文字列が生成されない可能性がある。
- `BaseTimeSimulator` のスレッドは detach したまま。複数スレッドからの呼び出しの不具合修正はアーカイブブランチ側にしかない。
- `QApplication/CMakeLists.txt` が `SIMULATORLISTDIALOG_LIBRARY` マクロを流用している。
- 作者のコミットメッセージ上も「リファクタリング必要」「DLL対応中」「Boost 対応途中」の箇所が残る。

## 移管情報

| 項目 | 内容 |
| --- | --- |
| 移管元 | `https://github.com/endazon/Training`（非公開、master） |
| 移管日 | 2026-09-17 |
| 開発期間 | 2022-01-16 〜 2023-01-08（master 42 コミット） |
| 履歴 | パスを `projects/SignalSimulator/` に書き換えて全コミットを保持 |
| サブモジュール | `.gitmodules` はリポジトリルートへ移し、パスを書き換えた |

### 主な経緯

- 2022-01: 数値クラス、処理時間計測、SI 接頭辞、科学定数、DebuggingConsole
- 2022-02: CMake 化、タイマー・時刻取得、Signals（後の Simulator）、Qt ビルド環境、SimulatorListDialog の DLL 化
- 2022-04〜05: Qt フォルダ構成整理、サブモジュール追加、Bz ビルド対応、CommunicationHistoryListDialog
- 2023-01: 汎用タイマクラス変更（master 最終）

### アーカイブブランチ

移管元にあった未マージのブランチ・PR は、パスを書き換えたうえで次のブランチに残した。

| ブランチ | 内容 |
| --- | --- |
| `archive/SignalSimulator/BinaryEditorBzForQt` | 2022-05 の master から分岐し 2 コミット（master 最終コミットは含まない）。Bz の Qt 移植着手（`BinaryEditorBz/qt/`、アドレス付きテキストビューア）と、複数スレッドからの呼び出し異常の修正（`SimulatorListDialog` の行削除処理など） |
| `archive/SignalSimulator/pr-1-inverted-pendulum-sil-simulator` | PR #1（CLOSED・未マージ、2026-09-17）の 3 コミット。`InvertedPendulumSil/` に台車型倒立振子の SIL シミュレータを新規追加（.NET 10、Blazor WebAssembly のプラント ＋ ASP.NET Core のコントローラを SignalR で接続、LQR・スイングアップ、単体・結合・Playwright E2E テスト、Docker 構成）。整理のうえ [`projects/InvertedPendulumSil`](../InvertedPendulumSil/README.md) として移設済みのため、このブランチは記録用 |
