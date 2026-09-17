# GeometryControl

2 次元平面上に図形を描き、マウスで選択・移動・サイズ変更できる Windows Forms 用ユーザーコントロール `GeometricDrawing`（ライブラリ `Geometrical`）と、その動作確認アプリ。

## 概要

### 図形

- 長方形・正方形・楕円・円・点・多角形を、「塗り」「線」「塗り＋線＋文字」の 3 種類で用意。ほかに直線（`StraightLineFigure`）と文字列（`StringFigure`）。
- `FigureList` で複数の図形を 1 つにまとめられる（座標軸 `CoordinateAxisDraw`、グリッド `CoordinateGridDraw` もこれを利用）。
- `PolygonType` が回転角から正多角形の頂点を計算する（`Monogon` 〜 `Megagon`）。
- `String.AutoFontSizeAdjustment = true` で、図形の枠に収まるフォントサイズを自動計算する。

### 座標系（`CoordinateSystem`）

原点 `Origin`、右手系／左手系 `Direction`、回転 `Rotation`（0/90/180/270°）、縮尺 `ReducedScale`（既定 1.0 = 1mm）、拡大率 `MagnificationRate`（既定 1.0 = 100%）を持ち、図形の座標・サイズ・ペン・フォントを画面座標に変換する。

`GridFigurePlane` はグリッド線（5 目盛りごとに濃色）、原点と軸線、目盛り数値、左上の +X（赤）/ +Y（緑）矢印を描画する。下部のステータスバーに位置・縮尺・拡大率を表示する。

### 操作

| 操作 | 動作 |
| --- | --- |
| 左クリック / Ctrl+左クリック | 図形の選択 / 複数選択（シアンの枠） |
| 右ドラッグ | 原点の移動（パン） |
| Ctrl+ホイール | 拡大率 ±1.0 |
| 選択枠の内側をドラッグ | 移動 |
| 右下の角 / 右端 / 下端をドラッグ | 幅と高さ / 幅 / 高さの変更 |
| 右上の角をドラッグ | 回転（未実装） |

キー状態はマウスが平面上にある間だけ低レベルキーボードフック（`WH_KEYBOARD_LL`）で取得する。

イベント: `SelectFigureChanged`、`MouseMouseMoveForPlane`。

## 使い方

```csharp
var fig = new RectangleFigure();
fig.Location = new(50, 50);
fig.Size = new(10, 20);
fig.Line.Color = Brushes.Orange;
fig.Fill.Color = Brushes.Red;
fig.String.Text = "aaa";
fig.String.AutoFontSizeAdjustment = true;
geometricDrawing1.Add(fig);

var poly = new PolygonFigure();
poly.Vertex = PolygonType.Hexagon(90f);
poly.Location = new(150, 50);
poly.Size = new(10, 10);
geometricDrawing1.Add(poly);
```

`GeometricDrawing` は `IList<IFigure>`（`Add` / `Insert` / `Remove` / `Clear` など）を実装し、`Origin`、`Direction`、`Rotation`、`ReducedScale`、`MagnificationRate`、`EditingProhibited` を公開する。使用例は `DebuggingForms/DebuggingForm.cs`。

## 技術スタック

- C# / Windows Forms / .NET 6（`net6.0-windows`、SDK 形式 csproj、Nullable 有効）
- NuGet 依存なし（Win32 P/Invoke のみ）
- Visual Studio 2022 17.5 以降

## ディレクトリ構成

```text
GeometryControl.sln
DebuggingForms/                 動作確認用アプリ（WinExe）
Geometrical/                    ライブラリ本体（名前空間 Geometrical）
  Resources/RotationArrows.png  回転カーソル画像（未使用）
  SourceCode/
    UserControl/                GeometricDrawing（公開コントロール）
    Plane/                      CoordinateSystem / FigurePlane（描画・選択・編集）/ GridFigurePlane
    Figure/
      IFigure.cs, BasicFigure.cs, FigureOperation.cs, PolygonType.cs
      FillFigure/               塗りのみ
      LineFigure/               線のみ・直線
      StringFigure/             文字列
      FillAndLineAndStringFigure/ 塗り＋線＋文字の複合
      CompositeFigure/          FigureList / 座標軸 / グリッド
    Utility/PointFConverter.cs  デザイナ用 TypeConverter
```

複合図形（`BasicTemplateFillAndLineAndStringFigure<Fill, Line, String>`）は `Fill` / `Line` / `String` の 3 部品を持ち、`Location`・`Size`・`Visible` は全部品に反映される。

## ビルド・実行

Visual Studio で `GeometryControl.sln` を開き、`DebuggingForms` をスタートアッププロジェクトにして実行する。CLI の場合:

```bash
dotnet run --project DebuggingForms
```

移管時点ではビルドを確認していない。

## 注意点・未完成部分

- 回転は未実装（`FigurePlane.OnMouseMove` に `//TODO:FugureRotate`、カーソルは仮の `Cursors.No`）。
- `EditingProhibited` が true のときに編集でき、false で編集できない（論理が逆の可能性）。
- 選択枠が作られるのは `RectangleFigure` / `EllipseFigure` / `PolygonFigure` のみ。
- `MouseMouseMoveForPlaneEventArgs` が基底 `MouseEventArgs` の y に x を渡している（`XF` / `YF` は正しい）。
- `PolygonLineFigure.Draw` は線幅を縮尺変換していない。
- 既定フォントは `MS UI Gothic` 固定。テストプロジェクトなし。
- ソースの文字コードは UTF-8（BOM）と Shift-JIS（`FigurePlane.cs`、`BasicFigure.cs`）が混在。
- クラス名 `BasicPalygonFigure` の綴りは原文のまま。

## 移管情報

| 項目 | 内容 |
| --- | --- |
| 移管元 | `https://github.com/endazon/GeometryControl`（非公開、master） |
| 移管日 | 2026-09-17 |
| 開発期間 | 2023-11-23 〜 2023-12-10（14 コミット） |
| 履歴 | パスを `projects/GeometryControl/` に書き換えて全コミットを保持 |
| ブランチ | master のみ（未マージのブランチ・PR なし） |

### 主な経緯

- 2023-11-23〜26: プロジェクト作成、途中経過を経て「ベータ版初版」（座標系を拡張）
- 2023-12-09: 多角形、フォントサイズ自動調整を追加。サイズ変更時の例外やキーイベント残留などを修正
- 2023-12-10: 移動・サイズ変更を追加、フォルダ構成を現在の形に整理、回転操作の骨組みを追加
