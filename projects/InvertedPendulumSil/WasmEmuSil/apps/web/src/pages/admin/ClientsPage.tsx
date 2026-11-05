import { useState } from "react";
import { useForm } from "react-hook-form";
import { del, post } from "../../api/client";
import { useApiKeys, useClients, useInvalidatingMutation } from "../../api/hooks";
import { isAdmin, useAuth } from "../../auth/AuthContext";
import { ErrorBox, Loading, fmtTime } from "../../components/common";
import type { ApiKey } from "../../api/types";

export function ClientsPage() {
  const { principal } = useAuth();
  const clients = useClients();
  const keys = useApiKeys();
  const [created, setCreated] = useState<ApiKey | null>(null);
  const { register, handleSubmit, reset } = useForm<{ name: string; allowed: string }>({ defaultValues: { allowed: "*" } });
  const create = useInvalidatingMutation((f: { name: string; allowed: string }) => post<ApiKey>("/api/admin/api-keys", { name: f.name, allowedManifests: f.allowed.split(",").map((s) => s.trim()).filter(Boolean) }), [["/api/admin/api-keys"]]);
  const revoke = useInvalidatingMutation((id: string) => del(`/api/admin/api-keys/${id}`), [["/api/admin/api-keys"]]);
  return (
    <div>
      <h1>クライアント</h1>
      <h2>hello してきたエンジン</h2>
      <ErrorBox error={clients.error} />
      {clients.isLoading && <Loading />}
      <table>
        <thead>
          <tr>
            <th>主体</th>
            <th>エンジン</th>
            <th>OS / arch</th>
            <th>WIT</th>
            <th>最終接続</th>
            <th>起動中</th>
          </tr>
        </thead>
        <tbody>
          {(clients.data?.items ?? []).map((c) => (
            <tr key={c.id}>
              <td>
                {c.principalName} <span className="muted">({c.principalKind})</span>
              </td>
              <td className="mono">{c.engineVersion}</td>
              <td className="mono">
                {c.os}/{c.arch}
              </td>
              <td className="mono">{c.witVersions.join(", ")}</td>
              <td>
                {fmtTime(c.lastSeen)} <span className="muted">({c.helloCount} 回, 初回 {fmtTime(c.firstSeen)})</span>
              </td>
              <td className="mono">{c.running.map((r) => `${r.manifestId}@${r.revision}`).join(", ") || "-"}</td>
            </tr>
          ))}
        </tbody>
      </table>

      {isAdmin(principal) && (
        <>
          <h2>クライアント API キー (headless / CI 用)</h2>
          <p className="muted">
            エンジンは <code>ipemu-engine run --server … --api-key … --launch id@rev</code> で起動します。キーは作成時に 1 度だけ表示されます。
          </p>
          <form className="card row wrap" onSubmit={handleSubmit((f) => create.mutate(f, { onSuccess: (k) => (setCreated(k), reset()) }))}>
            <input placeholder="名前 (例 ci-linux)" {...register("name", { required: true })} />
            <input placeholder="取得を許す構成 id (comma)。* で全部" {...register("allowed")} />
            <button type="submit" disabled={create.isPending}>
              発行
            </button>
            <ErrorBox error={create.error} />
          </form>
          {created && (
            <div className="banner">
              <strong>{created.name}</strong> のキー (この画面を離れると二度と表示されません):
              <pre className="mono select-all">{created.key}</pre>
            </div>
          )}
          <ErrorBox error={keys.error ?? revoke.error} />
          <table>
            <thead>
              <tr>
                <th>名前</th>
                <th>prefix</th>
                <th>構成</th>
                <th>作成</th>
                <th>最終使用</th>
                <th>状態</th>
              </tr>
            </thead>
            <tbody>
              {(keys.data?.items ?? []).map((k) => (
                <tr key={k.id} className={k.revokedAt ? "muted" : ""}>
                  <td>{k.name}</td>
                  <td className="mono">{k.prefix}…</td>
                  <td className="mono">{k.allowedManifests.join(", ")}</td>
                  <td>
                    {fmtTime(k.createdAt)} <span className="muted">{k.createdBy}</span>
                  </td>
                  <td>{fmtTime(k.lastUsedAt)}</td>
                  <td>
                    {k.revokedAt ? (
                      `失効 ${fmtTime(k.revokedAt)}`
                    ) : (
                      <button className="small danger" onClick={() => confirm(`${k.name} を失効しますか?`) && revoke.mutate(k.id)}>
                        失効
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </div>
  );
}
