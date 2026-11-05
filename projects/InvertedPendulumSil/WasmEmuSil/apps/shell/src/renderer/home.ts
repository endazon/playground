/// <reference path="./shell-api.d.ts" />
export {}; // 各ページは独立したモジュール (グローバルを共有しない)
const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

async function refreshEngine() {
  try {
    const s = await window.shell.engineStatus();
    $("engine").textContent = s.engine.running
      ? `起動中 (pid ${s.engine.pid}, ws :${s.engine.port}, v${s.engine.version})` + (s.session ? ` — ${s.session.name} [${s.session.run.state}]` : " — セッションなし")
      : "未起動 (構成を組み立てると起動します)";
    $<HTMLButtonElement>("open-sim").disabled = !s.uiReady;
  } catch (e) {
    $("engine").textContent = `状態を取得できません: ${String(e)}`;
  }
}

async function refreshCatalog() {
  const box = $("catalog");
  try {
    const c = await window.shell.catalogLocal();
    box.replaceChildren(
      ...(c.entries.length
        ? c.entries.map((e) => {
            const row = document.createElement("div");
            row.className = "row";
            const name = document.createElement("span");
            name.textContent = `${e.name} (${e.id} rev ${e.revision})`;
            const b = document.createElement("button");
            b.className = "btn primary";
            b.textContent = "組み立てる";
            b.onclick = () => e.source.kind === "local" && launch(e.source.manifestPath);
            row.append(name, b);
            return row;
          })
        : [Object.assign(document.createElement("span"), { className: "sub", textContent: "カタログは空です (settings.catalogDirs)" })]),
    );
  } catch (e) {
    box.textContent = `カタログを取得できません: ${String(e)}`;
  }
  void refreshEngine();
}

async function launch(path: string) {
  $("launch-msg").textContent = "組み立て中…";
  const r = await window.shell.launchLocal(path);
  $("launch-msg").textContent = r.ok ? "組み立てました。シミュレータのタブを開きます" : `失敗: ${r.error}`;
  void refreshEngine();
}

async function refreshSettings() {
  const s = await window.shell.getSettings();
  $("settings-path").textContent = s.settingsPath;
  $<HTMLInputElement>("allow-localhost").checked = s.allowLocalhost;
  $("trusted").replaceChildren(
    ...(s.trustedServers.length
      ? s.trustedServers.map((o) => {
          const row = document.createElement("div");
          row.className = "row";
          const code = document.createElement("code");
          code.textContent = o;
          const b = document.createElement("button");
          b.className = "btn";
          b.textContent = "削除";
          b.onclick = async () => {
            await window.shell.setSettings({ trustedServers: s.trustedServers.filter((x) => x !== o) });
            void refreshSettings();
          };
          row.append(code, b);
          return row;
        })
      : [Object.assign(document.createElement("span"), { className: "sub", textContent: "(なし)" })]),
  );
}

$("refresh").onclick = () => void refreshCatalog();
$("open-sim").onclick = () => void window.shell.openSimulator();
$("launch-path").onclick = () => {
  const p = $<HTMLInputElement>("manifest-path").value.trim();
  if (p) void launch(p);
};
$("trusted-add-btn").onclick = async () => {
  const v = $<HTMLInputElement>("trusted-add").value.trim();
  if (!v) return;
  const s = await window.shell.getSettings();
  await window.shell.setSettings({ trustedServers: [...new Set([...s.trustedServers, v])] });
  $<HTMLInputElement>("trusted-add").value = "";
  void refreshSettings();
};
$("allow-localhost").onchange = async (e) => {
  await window.shell.setSettings({ allowLocalhost: (e.target as HTMLInputElement).checked });
};
void refreshCatalog();
void refreshSettings();
