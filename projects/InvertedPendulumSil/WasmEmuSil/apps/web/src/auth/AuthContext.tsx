import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { ApiError, get, getToken, post, setToken } from "../api/client";
import type { LoginResponse, Principal, Role } from "../api/types";

interface AuthState {
  principal: Principal | null;
  loading: boolean;
  token: string | null;
  login: (username: string, password: string) => Promise<Principal>;
  logout: () => Promise<void>;
  refresh: () => Promise<void>;
}

const Ctx = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [principal, setPrincipal] = useState<Principal | null>(null);
  const [loading, setLoading] = useState(true);
  const [token, setTok] = useState<string | null>(getToken());

  const refresh = useCallback(async () => {
    if (!getToken()) {
      setPrincipal(null);
      setLoading(false);
      return;
    }
    try {
      const r = await get<{ principal: Principal }>("/api/auth/me");
      setPrincipal(r.principal);
    } catch (e) {
      if (e instanceof ApiError && e.status === 401) {
        setToken(null);
        setTok(null);
      }
      setPrincipal(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const login = useCallback(async (username: string, password: string) => {
    const r = await post<LoginResponse>("/api/auth/login", { username, password });
    setToken(r.token);
    setTok(r.token);
    setPrincipal(r.principal);
    return r.principal;
  }, []);

  const logout = useCallback(async () => {
    try {
      await post("/api/auth/logout");
    } catch {
      /* 失効済みでも構わない */
    }
    setToken(null);
    setTok(null);
    setPrincipal(null);
  }, []);

  const value = useMemo(() => ({ principal, loading, token, login, logout, refresh }), [principal, loading, token, login, logout, refresh]);
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useAuth(): AuthState {
  const v = useContext(Ctx);
  if (!v) throw new Error("AuthProvider がありません");
  return v;
}

// ロール判定 (サーバの auth.rs と同じ規則)
export const hasRole = (p: Principal | null, ...roles: Role[]) => !!p && roles.some((r) => p.roles.includes(r));
export const canViewAdmin = (p: Principal | null) => hasRole(p, "viewer", "editor", "approver", "admin");
export const canEdit = (p: Principal | null) => hasRole(p, "editor", "admin");
export const canApprove = (p: Principal | null) => hasRole(p, "approver", "admin");
export const isAdmin = (p: Principal | null) => hasRole(p, "admin");
