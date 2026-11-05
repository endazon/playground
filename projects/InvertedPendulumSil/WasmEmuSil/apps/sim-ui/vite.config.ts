import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// 出力は app://ui/ (シェル) でも file:// でも動くよう相対パス。build/ui/cartpole/ が tar.gz の中身になる
export default defineConfig({
  plugins: [react()],
  base: "./",
  build: {
    outDir: "../../build/ui/cartpole",
    emptyOutDir: true,
    target: "es2022",
    sourcemap: false,
    chunkSizeWarningLimit: 1000, // three.js を同梱するので 1 チャンクが大きい
  },
  server: {
    port: 5173,
    strictPort: false,
  },
});
