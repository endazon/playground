export {}; // 各ページは独立したモジュール (グローバルを共有しない)
// タブ / アドレスバー。状態は main が持ち、ここは描いて操作を送るだけ
const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const tabsEl = $<HTMLDivElement>("tabs");
const addr = $<HTMLInputElement>("addr");
const back = $<HTMLButtonElement>("back");
const fwd = $<HTMLButtonElement>("fwd");
let editing = false;

window.shell.onTabs((tabs) => {
  tabsEl.replaceChildren(
    ...tabs.map((t) => {
      const el = document.createElement("div");
      el.className = `tab kind-${t.kind}${t.active ? " active" : ""}`;
      el.title = t.url;
      const kind = document.createElement("span");
      kind.className = "kind";
      kind.textContent = t.kind === "sim" ? "SIM" : t.kind === "home" ? "HOME" : "WEB";
      const title = document.createElement("span");
      title.className = "title";
      title.textContent = (t.loading ? "⋯ " : "") + (t.title || t.url);
      const close = document.createElement("button");
      close.className = "close";
      close.textContent = "×";
      close.title = "閉じる";
      close.onclick = (e) => {
        e.stopPropagation();
        void window.shell.closeTab(t.id);
      };
      el.append(kind, title, close);
      el.onclick = () => void window.shell.selectTab(t.id);
      return el;
    }),
    (() => {
      const b = document.createElement("button");
      b.className = "newtab";
      b.textContent = "+";
      b.title = "新しいタブ";
      b.onclick = () => void window.shell.newTab();
      return b;
    })(),
  );
  const active = tabs.find((t) => t.active);
  if (active && !editing) addr.value = active.url === "app://shell/home.html" ? "" : active.url;
  back.disabled = !active?.canGoBack;
  fwd.disabled = !active?.canGoForward;
});
addr.addEventListener("focus", () => (editing = true));
addr.addEventListener("blur", () => (editing = false));
addr.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    void window.shell.navigate(addr.value);
    addr.blur();
  }
});
back.onclick = () => void window.shell.back();
fwd.onclick = () => void window.shell.forward();
$("reload").onclick = () => void window.shell.reload();
$("home").onclick = () => void window.shell.navigate("app://shell/home.html");
$("sim").onclick = () => void window.shell.openSimulator();

const toast = $<HTMLDivElement>("toast");
let toastTimer = 0;
window.shell.onToast((m) => {
  toast.textContent = m.text;
  toast.className = `toast show ${m.level}`;
  clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => (toast.className = "toast"), m.level === "error" ? 8000 : 4000);
});
