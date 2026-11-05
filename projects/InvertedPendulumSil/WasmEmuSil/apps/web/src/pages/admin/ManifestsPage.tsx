import { useForm } from "react-hook-form";
import { Link, useNavigate } from "react-router-dom";
import { post } from "../../api/client";
import { useInvalidatingMutation, useManifests } from "../../api/hooks";
import { canEdit, useAuth } from "../../auth/AuthContext";
import { ErrorBox, Loading, StateBadge, fmtTime } from "../../components/common";
import type { ManifestAdmin } from "../../api/types";

export function ManifestsPage() {
  const { principal } = useAuth();
  const { data, error, isLoading } = useManifests();
  const nav = useNavigate();
  const { register, handleSubmit } = useForm<{ id: string }>();
  const create = useInvalidatingMutation((id: string) => post<ManifestAdmin>("/api/admin/manifests", { id }), [["/api/admin/manifests"]]);
  return (
    <div>
      <h1>構成 (Manifests)</h1>
      {canEdit(principal) && (
        <form className="card row" onSubmit={handleSubmit((f) => create.mutate(f.id, { onSuccess: (m) => nav(`/admin/manifests/${m.id}/rev/1`) }))}>
          <label>
            新しい構成の id
            <input placeholder="cartpole-lqr-default" {...register("id", { required: true, pattern: /^[a-z0-9][a-z0-9-]{2,63}$/ })} />
          </label>
          <button type="submit" disabled={create.isPending}>
            下書きを作成
          </button>
          <ErrorBox error={create.error} />
        </form>
      )}
      <ErrorBox error={error} />
      {isLoading && <Loading />}
      <table>
        <thead>
          <tr>
            <th>id</th>
            <th>名前</th>
            <th>公開範囲</th>
            <th>既定 rev</th>
            <th>revisions</th>
            <th>更新</th>
          </tr>
        </thead>
        <tbody>
          {(data?.items ?? []).map((m) => (
            <tr key={m.id}>
              <td className="mono">
                <Link to={`/admin/manifests/${encodeURIComponent(m.id)}`}>{m.id}</Link>
              </td>
              <td>{m.name}</td>
              <td>{m.visibility === "all" ? "全利用者" : `グループ: ${m.visibilityGroups.join(", ") || "(なし)"}`}</td>
              <td>{m.defaultRevision ?? "-"}</td>
              <td>
                {m.revisions.map((r) => (
                  <Link key={r.revision} to={`/admin/manifests/${encodeURIComponent(m.id)}/rev/${r.revision}`} className="revlink">
                    {r.revision} <StateBadge state={r.state} />
                  </Link>
                ))}
              </td>
              <td>{fmtTime(m.updatedAt)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
