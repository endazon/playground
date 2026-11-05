/// <reference path="./shell-api.d.ts" />
export {}; // 各ページは独立したモジュール (グローバルを共有しない)
interface ConfirmData {
  summary: string;
  link: { server?: string; manifestId?: string; revision?: number; ticket?: string; manifestPath?: string };
  problems: string[];
  raw: string;
}
const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
window.shell.onDialogData((d) => {
  const data = d as ConfirmData;
  const kv = $("kv");
  const rows: [string, string][] = [
    ["要約", data.summary],
    ["サーバ", data.link.server ?? "(ローカル)"],
    ["構成", data.link.manifestId ?? data.link.manifestPath ?? "-"],
    ["revision", data.link.revision !== undefined ? String(data.link.revision) : "最新"],
    ["チケット", data.link.ticket ? "あり" : "なし"],
    ["元の URL", data.raw],
  ];
  kv.replaceChildren(
    ...rows.flatMap(([k, v]) => {
      const b = document.createElement("b");
      b.textContent = k;
      const s = document.createElement("span");
      s.textContent = v; // textContent なので外部文字列で見た目は崩せない
      return [b, s];
    }),
  );
  const p = $("problems");
  p.replaceChildren(...data.problems.map((t) => Object.assign(document.createElement("div"), { className: "problem", textContent: `✖ ${t}` })));
  $<HTMLButtonElement>("ok").disabled = data.problems.length > 0;
});
$("ok").onclick = () => void window.shell.answer({ ok: true });
$("cancel").onclick = () => void window.shell.answer({ ok: false });
