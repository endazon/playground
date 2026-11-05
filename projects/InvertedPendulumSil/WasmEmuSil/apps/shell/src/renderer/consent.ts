export {}; // 各ページは独立したモジュール (グローバルを共有しない)
interface ConsentData {
  requestId: string;
  instance: string;
  artifactSha256: string;
  capability: string;
  reason: string;
  manifestId: string;
  revision: number;
}
const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
window.shell.onDialogData((d) => {
  const c = d as ConsentData;
  const rows: [string, string][] = [
    ["構成", `${c.manifestId} rev ${c.revision}`],
    ["部品", c.instance],
    ["capability", c.capability],
    ["理由", c.reason],
    ["artifact", c.artifactSha256],
  ];
  $("kv").replaceChildren(
    ...rows.flatMap(([k, v]) => [Object.assign(document.createElement("b"), { textContent: k }), Object.assign(document.createElement("span"), { textContent: v })]),
  );
});
$("deny").onclick = () => void window.shell.answer({ grant: false, remember: false });
$("once").onclick = () => void window.shell.answer({ grant: true, remember: false });
$("remember").onclick = () => void window.shell.answer({ grant: true, remember: true });
