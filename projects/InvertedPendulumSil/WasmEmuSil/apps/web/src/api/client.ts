// fetch の薄いラッパ。認証は Bearer のみ (Cookie を使わないので CSRF 対策が不要)。
// トークンは sessionStorage に置く (タブを閉じると消える。シェル経由でエンジンに渡すのもこの値)。

const TOKEN_KEY = "ipemu.token";

export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message);
  }
}

export function getToken(): string | null {
  try {
    return sessionStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

export function setToken(token: string | null): void {
  try {
    if (token) sessionStorage.setItem(TOKEN_KEY, token);
    else sessionStorage.removeItem(TOKEN_KEY);
  } catch {
    /* プライベートモードなどで保存できない場合はメモリ上だけで動く */
  }
}

export interface RequestOptions {
  method?: string;
  body?: unknown;
  form?: FormData;
  headers?: Record<string, string>;
  signal?: AbortSignal;
}

export async function api<T>(path: string, opts: RequestOptions = {}): Promise<T> {
  const headers: Record<string, string> = { ...(opts.headers ?? {}) };
  const token = getToken();
  if (token) headers["Authorization"] = `Bearer ${token}`;
  let body: BodyInit | undefined;
  if (opts.form) {
    body = opts.form;
  } else if (opts.body !== undefined) {
    headers["Content-Type"] = "application/json";
    body = JSON.stringify(opts.body);
  }
  const res = await fetch(path, { method: opts.method ?? (body ? "POST" : "GET"), headers, body, signal: opts.signal });
  const text = await res.text();
  let json: unknown = null;
  try {
    json = text ? JSON.parse(text) : null;
  } catch {
    json = null;
  }
  if (!res.ok) {
    const err = (json as { error?: { code?: string; message?: string } } | null)?.error;
    if (res.status === 401) setToken(null);
    throw new ApiError(res.status, err?.code ?? "http_error", err?.message ?? `HTTP ${res.status}`);
  }
  return json as T;
}

export const get = <T>(path: string) => api<T>(path);
export const post = <T>(path: string, body?: unknown) => api<T>(path, { method: "POST", body: body ?? {} });
export const put = <T>(path: string, body: unknown) => api<T>(path, { method: "PUT", body });
export const del = <T>(path: string) => api<T>(path, { method: "DELETE" });
