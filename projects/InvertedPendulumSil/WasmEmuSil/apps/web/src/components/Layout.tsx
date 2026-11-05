import { Link, NavLink, Outlet, useNavigate } from "react-router-dom";
import { canViewAdmin, isAdmin, useAuth } from "../auth/AuthContext";
import { hasShellBridge } from "../launch";

export function Layout() {
  const { principal, logout } = useAuth();
  const nav = useNavigate();
  return (
    <div className="app">
      <header className="topbar">
        <Link to="/" className="brand">
          ipemu
        </Link>
        <nav>
          <NavLink to="/" end>
            カタログ
          </NavLink>
          <NavLink to="/me/launches">起動履歴</NavLink>
          {canViewAdmin(principal) && <NavLink to="/admin">管理</NavLink>}
        </nav>
        <div className="spacer" />
        {hasShellBridge() && <span className="pill">シェル内</span>}
        {principal && (
          <span className="user">
            {principal.name} <span className="muted">[{principal.roles.join(", ")}]</span>
          </span>
        )}
        <button
          className="link"
          onClick={() => {
            void logout().then(() => nav("/login"));
          }}
        >
          ログアウト
        </button>
      </header>
      {principal?.mustChangePassword && (
        <div className="banner">
          初回ログインです。<Link to="/me/password">パスワードを変更</Link>してください。
        </div>
      )}
      <main>
        <Outlet />
      </main>
      <footer className="muted">
        WasmEmuSil ipemu-server {isAdmin(principal) ? "(admin)" : ""}
      </footer>
    </div>
  );
}

export function AdminLayout() {
  const { principal } = useAuth();
  return (
    <div className="admin">
      <aside>
        <NavLink to="/admin/artifacts">部品</NavLink>
        <NavLink to="/admin/manifests">構成</NavLink>
        <NavLink to="/admin/approvals">capability 承認</NavLink>
        {isAdmin(principal) && <NavLink to="/admin/users">ユーザ</NavLink>}
        <NavLink to="/admin/clients">クライアント</NavLink>
        <NavLink to="/admin/audit">監査</NavLink>
      </aside>
      <section>
        <Outlet />
      </section>
    </div>
  );
}
