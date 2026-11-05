import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// 開発時は ipemu-server (既定 127.0.0.1:8080) に API をプロキシする。本番はサーバが dist/ を静的配信する
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "/api": { target: process.env.IPEMU_DEV_SERVER ?? "http://127.0.0.1:8080", changeOrigin: true },
      "/healthz": { target: process.env.IPEMU_DEV_SERVER ?? "http://127.0.0.1:8080" },
    },
  },
  build: {
    outDir: "dist",
    sourcemap: false,
    rollupOptions: {
      output: {
        // エディタ (CodeMirror) は管理画面でしか使わないので別チャンクに分ける
        manualChunks: { codemirror: ["codemirror", "@codemirror/lang-json", "@codemirror/lint", "@codemirror/state", "@codemirror/view"] },
      },
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
  },
});
