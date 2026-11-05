import { resolve } from "node:path";
import { defineConfig } from "vite";

// renderer のページ (シェル自身の UI): chrome (タブ / アドレスバー)、home、confirm、consent。
// app://shell/<page>.html で配信する。外部の origin からは到達できない
export default defineConfig({
  root: resolve(__dirname, "src/renderer"),
  base: "./",
  build: {
    outDir: resolve(__dirname, "dist/renderer"),
    emptyOutDir: true,
    target: "es2022",
    rollupOptions: {
      input: {
        chrome: resolve(__dirname, "src/renderer/chrome.html"),
        home: resolve(__dirname, "src/renderer/home.html"),
        confirm: resolve(__dirname, "src/renderer/confirm.html"),
        consent: resolve(__dirname, "src/renderer/consent.html"),
      },
    },
  },
});
