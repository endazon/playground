# three.js r128 (MIT)

3D ビュー用に同梱しています。CDN から読むのをやめた理由:

- 外部 CDN が落ちる・社内プロキシで遮断されると 3D ビューが無言で消える (フォールバックに落ちる)
- SRI を付けても、オフライン/エアギャップ環境では動かない
- 外部オリジンを許すと Content-Security-Policy が緩くなる

更新するときは https://github.com/mrdoob/three.js のリリースから `build/three.min.js` を取得し、
`sim-view.js` の 3D 部分 (r128 の API を使用) が動くことを E2E で確認してください。
