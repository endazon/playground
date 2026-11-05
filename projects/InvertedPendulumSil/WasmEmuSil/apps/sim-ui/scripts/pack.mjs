// build/ui/cartpole/ を UI バンドル (単一ファイルのアーティファクト = tar.gz) に固める。
// dev/cartpole.manifest.json の ui-cartpole@0.1.0 が ../build/ui/ui-cartpole.tar.gz を指す。
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../..");
const src = resolve(root, "build/ui/cartpole");
const out = resolve(root, "build/ui/ui-cartpole.tar.gz");
if (!existsSync(resolve(src, "index.html"))) {
  console.error(`pack: ${src}/index.html がありません (vite build が先)`);
  process.exit(1);
}
// tar は Windows 10+ (bsdtar) / macOS / Linux にある。決定的にするため mtime と所有者を固定する
const args = ["-czf", out, "-C", src, "."];
try {
  execFileSync("tar", ["--version"], { stdio: "ignore" });
} catch {
  console.error("pack: tar コマンドが見つかりません");
  process.exit(1);
}
execFileSync("tar", args, { stdio: "inherit" });
const bytes = readFileSync(out);
const sha = createHash("sha256").update(bytes).digest("hex");
console.log(`ui bundle: ${out} (${statSync(out).size} bytes)\n  sha256 ${sha}`);
