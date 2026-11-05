/** シェル (Electron) の preload が app://ui/ のページにだけ注入するブリッジ。ブラウザ開発時は存在しない */
interface IpemuEngineBridge {
  /** エンジンの接続先 (ループバック WS のポートとトークン) */
  endpoint(): Promise<{ port: number; token: string } | null>;
  /** native FW などで restartRequired になったとき、エンジンを再起動して同じ launch をやり直す */
  restartEngine(): Promise<void>;
  /** エンジンが再起動して接続先が変わったときに呼ばれる */
  onEndpointChanged(cb: (ep: { port: number; token: string }) => void): () => void;
}
interface Window {
  ipemuEngine?: IpemuEngineBridge;
}
