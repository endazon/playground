// main / preload を esbuild で束ねる (Electron の main は Node、preload は sandbox 下の CommonJS)。
// ui-kit (TS ソース) を取り込むので tsc だけでは済まない。renderer のページは vite が作る。
import { build } from "esbuild";
import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
mkdirSync(resolve(root, "dist"), { recursive: true });

const common = {
  bundle: true,
  platform: "node",
  format: "cjs",
  target: "node22",
  external: ["electron"],
  sourcemap: true,
  logLevel: "info",
  define: { "process.env.NODE_ENV": JSON.stringify(process.env.NODE_ENV ?? "production") },
};

await build({ ...common, entryPoints: [resolve(root, "src/main/index.ts")], outfile: resolve(root, "dist/main/index.cjs") });
await build({
  ...common,
  entryPoints: {
    bridge: resolve(root, "src/preload/bridge.ts"),
    simui: resolve(root, "src/preload/simui.ts"),
    shellui: resolve(root, "src/preload/shellui.ts"),
  },
  outdir: resolve(root, "dist/preload"),
  outExtension: { ".js": ".cjs" },
});
