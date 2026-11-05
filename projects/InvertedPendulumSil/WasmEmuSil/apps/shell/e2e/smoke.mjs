#!/usr/bin/env node
/**
 * Electron シェルの E2E スモークテスト (Playwright `_electron`)。
 * 実行: apps/shell/README.md の「E2E スモークテスト」を参照。Electron 本体と `pnpm --filter @ipemu/shell build`、
 * `cargo build -p ipemu-engine`、build/components と build/firmware が必要。check.sh には入れない。
 * シェルの挙動は一切変えず、外から操作して観測するだけ (設定は一時 userData の settings.json で与える)。
 */
import { createRequire } from "node:module";
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdtempSync, writeFileSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
let playwright;
try {
  playwright = require("playwright");
} catch {
  playwright = require(join(process.env.PLAYWRIGHT_DIR ?? "/opt/node-tools/node_modules", "playwright"));
}
const { _electron: electron } = playwright;

const here = dirname(fileURLToPath(import.meta.url));
const shellDir = resolve(here, "..");
const root = resolve(shellDir, "..", "..");
const manifest = join(root, "dev", "cartpole.manifest.json");
const electronBin = require(require.resolve("electron", { paths: [shellDir] }));
const asRoot = process.getuid?.() === 0;

let failed = 0;
const results = [];
function check(name, cond, detail = "") {
  console.log(`  ${cond ? "ok  " : "FAIL"} ${name}${detail ? ` — ${detail}` : ""}`);
  if (!cond) failed++;
  return cond;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function until(fn, ms = 20000, what = "condition") {
  const t0 = Date.now();
  for (;;) {
    const v = await fn();
    if (v) return v;
    if (Date.now() - t0 > ms) throw new Error(`timeout: ${what}`);
    await sleep(150);
  }
}

// ---- アプリの起動 ---------------------------------------------------------------
async function launchShell(settings, extraEnv = {}) {
  const userData = mkdtempSync(join(tmpdir(), "ipemu-e2e-"));
  if (settings) writeFileSync(join(userData, "settings.json"), JSON.stringify(settings));
  const logs = [];
  const args = [shellDir, ...(asRoot ? ["--no-sandbox-if-root"] : [])];
  const env = { ...process.env, IPEMU_SHELL_USER_DATA: userData, ...extraEnv };
  const app = await electron.launch({ executablePath: electronBin, args, env });
  app.process().stdout.on("data", (d) => logs.push(String(d)));
  const pages = () => app.windows();
  const findPage = (pred) => pages().find((p) => !p.isClosed() && pred(p.url()));
  const waitPage = (pred, what) => until(() => findPage(pred), 20000, what);
  const close = async () => {
    await app.close().catch(() => {});
    rmSync(userData, { recursive: true, force: true });
  };
  return { app, userData, logs, pages, findPage, waitPage, close, args, env };
}

/** 信頼 origin のテスト用 HTTP サーバ (127.0.0.1)。/ は空ページ */
function startServer() {
  return new Promise((res) => {
    const srv = createServer((req, r) => {
      r.setHeader("content-type", "text/html");
      r.end(`<!doctype html><title>e2e ${req.url}</title><p>hello</p>`);
    });
    srv.listen(0, "127.0.0.1", () => res({ srv, origin: `http://127.0.0.1:${srv.address().port}` }));
  });
}

/** 任意サイトのタブをホームから開く (chrome のアドレスバー経由と同じ shell:new-tab) */
async function openWebTab(s, url) {
  const home = await s.waitPage((u) => u.startsWith("app://shell/home.html"), "home");
  await home.evaluate((u) => window.shell.newTab(u), url);
  return s.waitPage((u) => u.startsWith(url.replace(/\/$/, "")), `web tab ${url}`);
}

const scenarios = [];
const scenario = (name, fn) => scenarios.push({ name, fn });

// ---- 1. ローカルカタログ → 組み立て → app://ui が状態を受け取る -------------------------
scenario("1. ローカルカタログからの起動と sim UI の状態受信", async () => {
  const s = await launchShell({ trustedServers: [], allowLocalhost: true });
  try {
    const home = await s.waitPage((u) => u.startsWith("app://shell/home.html"), "home");
    await home.waitForSelector("#catalog .row");
    const rows = await home.locator("#catalog .row span").allTextContents();
    check("ホームのカタログに dev/ の manifest が並ぶ", rows.some((t) => t.includes("dev-cartpole")), rows.join(" | "));
    check("dev/ の 3 つとも列挙される", rows.length >= 3, `rows=${rows.length}`);
    await home.locator("#catalog .row", { hasText: "dev-cartpole " }).first().locator("button").click();
    const ui = await s.waitPage((u) => u.startsWith("app://ui/"), "sim tab");
    check("app://ui/ のタブが開く", true, ui.url());
    // sim UI は engine の WS につながり、スナップショットを受けて統計バッジを出す (autoStart=false なので UI の実行ボタンで開始)
    await ui.waitForSelector(".app-grid", { timeout: 30000 });
    const snapshot = () => ui.evaluate(() => document.body.innerText);
    const before = await snapshot();
    const btn = ui.getByRole("button", { name: /^(▶|実行|開始|再開|Run|Start)/ }).first();
    if (await btn.count()) await btn.click();
    const after = await until(async () => {
      const t = await snapshot();
      return /×\d+\.\d+/.test(t) && /µs\/step/.test(t) ? t : null;
    }, 20000, "sim UI の統計バッジ (state を受信)");
    check("sim UI が engine に接続し統計 (×比 / µs/step) を表示する", /engine :\d+/.test(after), after.match(/engine :\d+|×\d+\.\d+|[\d.]+ µs\/step/g)?.join(" "));
    check("sim UI の表示が開始後に変化する", after !== before);
    // 仮想時間: エンジンの状態 (home の engineStatus) で tNs / tick が進む
    const st = await until(async () => {
      const x = await home.evaluate(() => window.shell.engineStatus());
      return x.session?.run?.state === "running" ? x : null;
    }, 20000, "run.state = running");
    await sleep(1000);
    const st2 = await home.evaluate(() => window.shell.engineStatus());
    check("仮想時間 (tNs) が進む", st2.session.run.tNs > st.session.run.tNs, `${st.session.run.tNs} -> ${st2.session.run.tNs} ns (tick ${st.session.run.tick} -> ${st2.session.run.tick})`);
    check("ログにエンジン起動の記録", s.logs.join("").includes("組み立てました"), "組み立てました");
  } finally {
    await s.close();
  }
});

// ---- 2. 信頼 origin だけにブリッジが注入される -----------------------------------------
scenario("2. 信頼 origin のブリッジ注入 (https 必須 / 開発時のみ loopback の http)", async () => {
  const { srv, origin } = await startServer();
  // allowLocalhost: false でも http は信頼されない = https 必須であること
  const strict = await launchShell({ trustedServers: [origin], allowLocalhost: false });
  try {
    const p = await openWebTab(strict, origin + "/");
    const has = await p.evaluate(() => typeof window.ipemu);
    check("allowLocalhost=false では trustedServers に載せた http origin にも注入しない (https 必須)", has === "undefined", `typeof window.ipemu=${has}`);
  } finally {
    await strict.close();
  }
  const s = await launchShell({ trustedServers: [], allowLocalhost: true });
  try {
    const p = await openWebTab(s, origin + "/");
    const before = await p.evaluate(() => typeof window.ipemu);
    check("信頼リストに無い origin には window.ipemu が無い", before === "undefined", `typeof=${before}`);
    // シェル自身の設定機構 (home の shell.setSettings = 設定 UI と同じ経路) で追加
    const home = await s.waitPage((u) => u.startsWith("app://shell/home.html"), "home");
    await home.evaluate((o) => window.shell.setSettings({ trustedServers: [o] }), origin);
    await p.reload();
    await p.waitForLoadState("load");
    const after = await p.evaluate(() => ({ t: typeof window.ipemu, keys: window.ipemu ? Object.keys(window.ipemu).sort() : [] }));
    check("信頼リストに追加し再読み込みすると window.ipemu が注入される", after.t === "object", JSON.stringify(after.keys));
    check("公開 API は launch/status/openSimulator/version のみ", JSON.stringify(after.keys) === JSON.stringify(["launch", "openSimulator", "status", "version"]));
    // 別ポート (別 origin) は信頼されない = 厳密比較
    const other = await startServer();
    try {
      const p2 = await openWebTab(s, other.origin + "/");
      const t = await p2.evaluate(() => typeof window.ipemu);
      check("同じホストの別ポートには注入されない (厳密比較)", t === "undefined", `typeof=${t}`);
    } finally {
      other.srv.close();
    }
  } finally {
    await s.close();
    srv.close();
  }
});

// ---- 3. web タブは app:// へ遷移できない ---------------------------------------------
scenario("3. web タブから app:// への遷移拒否", async () => {
  const { srv, origin } = await startServer();
  const s = await launchShell({ trustedServers: [], allowLocalhost: true });
  try {
    const p = await openWebTab(s, origin + "/");
    // ページ自身のスクリプトによる遷移
    await p.evaluate(() => { location.href = "app://shell/home.html"; }).catch(() => {});
    await sleep(1500);
    check("location.href = app://shell/ は拒否されタブは元の URL のまま", p.url().startsWith(origin), p.url());
    await p.evaluate(() => { location.href = "app://ui/index.html"; }).catch(() => {});
    await sleep(1000);
    check("app://ui/ も同様", p.url().startsWith(origin), p.url());
    // window.open (web タブ内) でも app:// の新しいタブは開かない
    const before = s.pages().length;
    await p.evaluate(() => { window.open("app://shell/home.html"); window.open("app://ui/index.html"); });
    await sleep(1000);
    check("window.open(app://...) で新しいウィンドウ / タブは増えない", s.pages().length === before, `pages ${before} -> ${s.pages().length}`);
    // 対照: http(s) へのページ内遷移は許可される
    const p2url = origin + "/other";
    await p.evaluate((u) => { location.href = u; }, p2url);
    await until(() => p.url() === p2url, 5000, "http navigation allowed");
    check("対照: http への遷移は許可", p.url() === p2url, p.url());
  } finally {
    await s.close();
    srv.close();
  }
});

// ---- 4. app://ui は外部 URL へ fetch できない (CSP) ------------------------------------
scenario("4. app://ui からの外部 fetch は CSP で拒否", async () => {
  const hits = [];
  const ext = createServer((req, r) => {
    hits.push(req.url);
    r.setHeader("access-control-allow-origin", "*");
    r.end("secret");
  });
  await new Promise((r) => ext.listen(0, "127.0.0.1", r));
  const extUrl = `http://127.0.0.1:${ext.address().port}/x`;
  const s = await launchShell({ trustedServers: [], allowLocalhost: true });
  try {
    const home = await s.waitPage((u) => u.startsWith("app://shell/home.html"), "home");
    await home.waitForSelector("#catalog .row");
    await home.locator("#catalog .row", { hasText: "dev-cartpole " }).first().locator("button").click();
    const ui = await s.waitPage((u) => u.startsWith("app://ui/"), "sim tab");
    await ui.waitForLoadState("load");
    const csp = await ui.evaluate(async () => (await fetch(location.href)).headers.get("content-security-policy"));
    check("app://ui の応答に CSP が付く", !!csp && csp.includes("connect-src 'self' ws://127.0.0.1:*"), csp ?? "");
    const r = await ui.evaluate(async (u) => {
      try {
        const res = await fetch(u);
        return { ok: true, text: await res.text() };
      } catch (e) {
        return { ok: false, err: String(e) };
      }
    }, extUrl);
    check("外部 http への fetch が失敗する", !r.ok, r.err ?? r.text);
    check("外部サーバにリクエストが届いていない", hits.length === 0, `hits=${hits.length}`);
    const r2 = await ui.evaluate(async () => {
      try {
        await fetch("https://example.com/");
        return "reached";
      } catch (e) {
        return String(e);
      }
    });
    check("https の外部 fetch も失敗する", r2 !== "reached", r2);
    // 対照: 自分自身 (app://ui) は取れる
    const self = await ui.evaluate(async () => (await fetch(location.href)).status);
    check("対照: app://ui 自身の fetch は成功", self === 200);
  } finally {
    await s.close();
    ext.close();
  }
});

// ---- 5. ipemu://launch リンクの確認ウィンドウ -------------------------------------------
/** 2 つ目のプロセスで ipemu:// を渡す (single-instance の second-instance 経路。実際の OS ハンドラと同じ) */
function sendLink(s, link) {
  // Playwright は自分で --no-sandbox を足すが、2 つ目のプロセスは素で起動するので root なら明示する
  const p = spawn(electronBin, [shellDir, ...(asRoot ? ["--no-sandbox", "--no-sandbox-if-root"] : []), link], { env: s.env, stdio: "ignore" });
  return p;
}
async function confirmDialog(s, link) {
  sendLink(s, link);
  const dlg = await s.waitPage((u) => u.startsWith("app://shell/confirm.html"), "confirm window");
  await dlg.waitForSelector("#kv span");
  await sleep(300);
  return {
    dlg,
    problems: await dlg.locator("#problems .problem").allTextContents(),
    okDisabled: await dlg.locator("#ok").isDisabled(),
    summary: (await dlg.locator("#kv span").first().textContent()) ?? "",
  };
}
/** キャンセルすると回答直後にウィンドウが閉じるので、クリック完了前に閉じても失敗にしない */
const cancel = (dlg) => dlg.locator("#cancel").click().catch((e) => { if (!/closed/.test(String(e))) throw e; });
scenario("5. ipemu:// リンクの確認ウィンドウ", async () => {
  const s = await launchShell({ trustedServers: ["https://trusted.example"], allowLocalhost: true });
  try {
    // 信頼していないサーバ
    const a = await confirmDialog(s, "ipemu://launch?server=https%3A%2F%2Fevil.example&manifest=dev-cartpole&rev=1&ticket=abc");
    check("確認ウィンドウが出る", true, a.summary);
    check("信頼していないサーバは理由が表示される", a.problems.some((t) => t.includes("信頼するサーバに登録されていません")), a.problems.join(" | "));
    check("「起動する」が押せない", a.okDisabled);
    await cancel(a.dlg);
    await until(() => !s.findPage((u) => u.startsWith("app://shell/confirm.html")), 5000, "dialog closes");
    // 信頼しているサーバ: 問題なし (ボタンは有効)。押さずにキャンセル (サーバは実在しない)
    const b = await confirmDialog(s, "ipemu://launch?server=https%3A%2F%2Ftrusted.example&manifest=dev-cartpole&rev=1&ticket=abc");
    check("信頼サーバのリンクは問題なし・ボタン有効", b.problems.length === 0 && !b.okDisabled, b.problems.join(" | "));
    await cancel(b.dlg);
    await until(() => !s.findPage((u) => u.startsWith("app://shell/confirm.html")), 5000, "dialog closes");
    // 既定 (allowLocalLaunch 未指定) ではローカル manifest は無効
    const c = await confirmDialog(s, `ipemu://launch?manifestPath=${encodeURIComponent(manifest)}`);
    check("既定では manifestPath リンクに「ローカル manifest の起動は設定で無効です」が出る", c.problems.includes("✖ ローカル manifest の起動は設定で無効です") || c.problems.some((t) => t.includes("ローカル manifest の起動は設定で無効です")), c.problems.join(" | "));
    check("その「起動する」は押せない", c.okDisabled);
    await cancel(c.dlg);
    check("どのリンクでも確認なしの起動は起きない (組み立て記録なし)", !s.logs.join("").includes("組み立てました"));
  } finally {
    await s.close();
  }
});

// ---- 6. M2-1: ブリッジ経由の manifestPath 起動は allowLocalLaunch で縛られる -----------------
scenario("6. M2-1 window.ipemu.launch({manifestPath}) と allowLocalLaunch", async () => {
  const { srv, origin } = await startServer();
  const call = (p) => p.evaluate(async (m) => {
    try {
      return { ok: true, r: await window.ipemu.launch({ manifestPath: m }) };
    } catch (e) {
      return { ok: false, err: String(e) };
    }
  }, manifest);
  // 既定 (キー無し)
  const d = await launchShell({ trustedServers: [origin] });
  try {
    const p = await openWebTab(d, origin + "/");
    check("信頼ページに window.ipemu がある", (await p.evaluate(() => typeof window.ipemu)) === "object");
    const r = await call(p);
    check("既定 (allowLocalLaunch 未指定) では拒否され allowLocalLaunch のエラーになる", !r.ok && r.err.includes("allowLocalLaunch"), r.err ?? JSON.stringify(r.r));
    check("拒否されたので組み立ては起きない", !d.logs.join("").includes("組み立てました"));
  } finally {
    await d.close();
  }
  // allowLocalLaunch: true
  const t = await launchShell({ trustedServers: [origin], allowLocalLaunch: true });
  try {
    const p = await openWebTab(t, origin + "/");
    const r = await call(p);
    check("allowLocalLaunch=true ではチェックを通り起動に成功する", r.ok && r.r.ok === true, JSON.stringify(r.r ?? r.err));
    await until(() => t.findPage((u) => u.startsWith("app://ui/")), 20000, "sim tab opened");
    check("シミュレータのタブが開く", true);
    // server 起動は他 origin を指定できない (ページ自身の origin のみ)
    const r2 = await p.evaluate(async () => {
      try {
        return { ok: true, r: await window.ipemu.launch({ server: "https://other.example", manifestId: "x" }) };
      } catch (e) {
        return { ok: false, err: String(e) };
      }
    });
    check("対照: 他 origin の server 指定は拒否", !r2.ok && r2.err.includes("ページ自身の origin"), r2.err ?? "");
  } finally {
    await t.close();
    srv.close();
  }
});

// ---- 実行 -------------------------------------------------------------------------
for (const required of [join(shellDir, "dist", "main", "index.cjs"), join(root, "build", "ui", "ui-cartpole.tar.gz")]) {
  if (!existsSync(required)) {
    console.error(`前提が無い: ${required} (README の手順でビルドしてください)`);
    process.exit(2);
  }
}
const only = process.argv[2];
for (const sc of scenarios) {
  if (only && !sc.name.startsWith(only + ".")) continue;
  console.log(`\n${sc.name}`);
  const before = failed;
  try {
    await sc.fn();
  } catch (e) {
    failed++;
    console.log(`  FAIL 例外: ${e?.stack ?? e}`);
  }
  results.push([sc.name, failed === before]);
}
console.log("\n==== 結果 ====");
for (const [n, ok] of results) console.log(`${ok ? "PASS" : "FAIL"}  ${n}`);
process.exit(failed ? 1 : 0);
