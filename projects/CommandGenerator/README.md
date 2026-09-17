# CommandGenerator

command generator for Windows application

通信ツールに流すコマンド（バイト列）を、JSON で定義した「基本コマンドリスト」から組み立てる Windows Forms アプリ。組み立てたコマンドは CSV に書き出すか、TCP で送信できる。

## 概要

- **基本コマンドリスト（JSON）**: コマンド（Item: `Name`, `Length`）→ 詳細（Detail: `Name`, `Offset`, `Size`）→ パラメータ（Parameter: `Name`, `Offset`, `Size`, `Type`, `Value`, `Fixed`）の 3 階層。`Type` は `HEX` / `DEC` / `ASCII` / `FILE_SELECT`。
- **生成結果（CSV）**: 列は `No., Name, Command, Length, Type`。`Command` はコマンドの 16 進文字列。
- サンプル: `SourceCode/Sample/default.json`、`SourceCode/Sample/default.csv`
- ユースケース図は `Document/UseCase.pu`、画面モックは `Document/mock-up/main.pu`（PlantUML）。

### 画面

| フォーム | 場所 | 役割 |
| --- | --- | --- |
| `FormMain`（CommandList） | `Target/FormMain.cs` | 起動画面。コマンド一覧、File / Seting / Mode / EditFileOpen メニュー |
| `ListSettings.FormMain`（ListSetting） | `Target/11_ListSetting/Form/` | 基本コマンドリストを TreeView で編集 |
| `FormEdit`（CommandEdit） | `Target/12_Mode/FormEdit.cs` | 各モード共通の基底。コマンド追加・パラメータ編集・CSV 保存 |
| `FormGenerate` | `Target/12_Mode/01_Generate/` | 生成モード。「Start Generation」で CSV 保存 |
| `FormCommunication` | `Target/12_Mode/02_Communication/` | 通信モード。ホスト・ポートを入力して TCP 接続し、選択コマンドを送信（UTF-8 + CRLF） |
| `FormCommunicationDsplay` | 同上 | 送受信ログ表示 |

TCP クライアントは `Target/Class/Com/TcpClient.cs`（Socket の非同期 API）。

## 技術スタック

- C# / Windows Forms / .NET Framework 4.7.2
- NuGet: `Newtonsoft.Json` 12.0.3（`SourceCode/packages/` にコミット済み）
- Visual Studio 2019（ソリューション形式 16.0）

## ディレクトリ構成

```text
Document/                       ユースケース図・画面モック（PlantUML）
SourceCode/
  CommandGenerator.sln
  CommandGenerator/             本体アプリ（WinExe）
    Target/
      01_Common/                拡張メソッド、Range<T>
      02_Utility/               入力ダイアログ、入力画面生成（Adapter / Facade / Strategy）
      11_ListSetting/           基本コマンドリスト編集画面
      12_Mode/                  FormEdit 基底、01_Generate、02_Communication
      Class/Com/                TCP クライアント
      Class/Display/            パラメータ入力画面
      Class/FileOperation/      CSV / JSON ファイル読み書き
      Class/Storage/            データモデル・変換・Parser
  WinFormsCtrlLibInputScreen/   入力用 UserControl ライブラリ（Decimal / Hexadecimal / String / Selection / File）
  TestWindowsFormsApp/          試験用アプリ（Form1 は空）
  Sample/                       サンプル JSON / CSV
  packages/                     NuGet パッケージ
```

### 設計上の特徴

- **Adapter**（`02_Utility/InputDisplay/Adapter/`）: `IInput` がライブラリの UserControl をラップし、fluent API で設定する。
- **Facade**（`InputDisplay/Facade/Searcher.cs`）: 型名から対応する `IInput` を返す（実質 Factory）。
- **Strategy**（`InputDisplay/Strategy/`）: `ILayout` の縦・横レイアウト実装を切り替える。
- `FormGenerate` / `FormCommunication` は `FormEdit` の virtual メソッドを override する。

## ビルド・実行

1. Windows に Visual Studio 2019 以降と .NET Framework 4.7.2 Developer Pack を用意する。
2. `SourceCode/CommandGenerator.sln` を開き、スタートアッププロジェクトを `CommandGenerator` にしてビルド・実行する。
3. ListSetting でリストを作る（または `Sample/default.json` を開く）→ Mode で Generate / Communication を選ぶ → コマンドを追加・編集 → CSV 保存または TCP 送信。

移管時点ではビルドを確認していない。

## 注意点・既知の課題

- 最終コミットが「Refactoring in progress1」で、リファクタリング途中の状態。
  - `02_Communication` に同名クラス `FormCommunicationDsplay` が 2 つある（`_FormCommunicationDsplay.cs` が旧版で、実際に使われているのはこちら）。
  - `11_ListSetting/Form/FormMain.cs` の `AddRange` / `Add` は中身が空。
  - `FormCommunication.cs` の接続先入力は「TODO:要変更」の暫定実装。`TcpClient` の改行区切り処理はコメントアウトのまま。
- `FormGenerate` を閉じると、カレントディレクトリ内の名前に「FileData」を含むファイルを削除する。
- `bin/`、`obj/`、`.vs/`、`packages/` がコミットされている（`.gitignore` なし）。`bin/Release` や `App.config` には旧名 `CommandCreator` が残る。
- 自動テストなし。メニュー表記「Seting」は原文のまま。

## ライセンス

MIT License（Copyright (c) 2020 endazon）。`LICENSE` を参照。

## 移管情報

| 項目 | 内容 |
| --- | --- |
| 移管元 | `https://github.com/endazon/CommandGenerator`（公開、master） |
| 移管日 | 2026-09-17 |
| 開発期間 | 2020-04-26 〜 2020-08-09（14 コミット） |
| 履歴 | パスを `projects/CommandGenerator/` に書き換えて全コミットを保持 |

### 主な経緯

- 2020-04: 初版（一覧・編集画面の骨組み、JSON / CSV Storage、Newtonsoft.Json 導入）
- 2020-05: コマンドリスト編集機能、リファクタリング、コマンドの型対応
- 2020-06: Command Parser、TCP 通信（送受信ログ画面）、ユースケース図
- 2020-08: `Target/` 配下の番号付きフォルダへ再構成、入力 UserControl ライブラリ分離（リファクタリング途中で停止）

### アーカイブブランチ

移管元にあった未マージのブランチ・PR は、パスを書き換えたうえで次のブランチに残した。

| ブランチ | 内容 |
| --- | --- |
| `archive/CommandGenerator/dependabot-Newtonsoft.Json-13.0.2` | Dependabot PR #3（OPEN）: Newtonsoft.Json 12.0.3 → 13.0.2 |
| `archive/CommandGenerator/pr-2-dependabot-Newtonsoft.Json-13.0.1` | Dependabot PR #2（CLOSED、#3 で置き換え）: 12.0.3 → 13.0.1 |
