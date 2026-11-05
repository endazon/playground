import type { ReactNode } from "react";
import { ApiError } from "../api/client";

export function ErrorBox({ error }: { error: unknown }) {
  if (!error) return null;
  const msg = error instanceof ApiError ? `${error.message} (${error.status})` : error instanceof Error ? error.message : String(error);
  return (
    <div className="error" role="alert">
      {msg}
    </div>
  );
}

export function Loading({ children = "読み込み中…" }: { children?: ReactNode }) {
  return <p className="muted">{children}</p>;
}

export function fmtTime(s: string | null | undefined): string {
  if (!s) return "-";
  const d = new Date(s);
  return isNaN(d.getTime()) ? s : d.toLocaleString();
}

export function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(2)} MB`;
}

export function StateBadge({ state }: { state: string }) {
  return <span className={`badge badge-${state}`}>{state}</span>;
}
