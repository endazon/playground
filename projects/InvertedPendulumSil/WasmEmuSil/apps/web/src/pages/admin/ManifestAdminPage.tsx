import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { post, put } from "../../api/client";
import { manifestKeys, useGroups, useInvalidatingMutation, useManifest } from "../../api/hooks";
import { canApprove, canEdit, useAuth } from "../../auth/AuthContext";
import { ErrorBox, Loading, StateBadge, fmtTime } from "../../components/common";
import type { RevisionDetail } from "../../api/types";

export function ManifestAdminPage() {
  const { id = "" } = useParams();
  const { principal } = useAuth();
  const nav = useNavigate();
  const { data: m, error, isLoading } = useManifest(id);
  const groups = useGroups();
  const [vis, setVis] = useState<"all" | "groups">("all");
  const [visGroups, setVisGroups] = useState("");
  useEffect(() => {
    if (m) {
      setVis(m.visibility);
      setVisGroups(m.visibilityGroups.join(", "));
    }
  }, [m]);

  const keys = manifestKeys(id);
  const setVisibility = useInvalidatingMutation(() => put(`/api/admin/manifests/${id}/visibility`, { visibility: vis, groups: visGroups.split(",").map((s) => s.trim()).filter(Boolean) }), keys);
  const setDefault = useInvalidatingMutation((revision: number) => put(`/api/admin/manifests/${id}/default-revision`, { revision }), keys);
  const newRevision = useInvalidatingMutation((fromRevision: number | null) => post<RevisionDetail>(`/api/admin/manifests/${id}/revisions`, { fromRevision }), keys);

  if (isLoading) return <Loading />;
  if (!m) return <ErrorBox error={error ?? new Error("構成がありません")} />;
  return (
    <div>
      <h1>
        {m.name} <span className="muted mono">{m.id}</span>
      </h1>
      <p>{m.description}</p>

      <section className="card">
        <h2>revisions</h2>
        <table>
          <thead>
            <tr>
              <th>rev</th>
              <th>状態</th>
              <th>検証</th>
              <th>編集者</th>
              <th>承認</th>
              <th>公開</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {m.revisions.map((r) => (
              <tr key={r.revision} className={m.defaultRevision === r.revision ? "hot" : ""}>
                <td>
                  <Link to={`/admin/manifests/${encodeURIComponent(id)}/rev/${r.revision}`}>rev {r.revision}</Link>
                  {m.defaultRevision === r.revision && <span className="pill">カタログの既定</span>}
                </td>
                <td>
                  <StateBadge state={r.state} />
                </td>
                <td>{r.validation ? (r.validation.ok ? <span className="ok">OK</span> : <span className="ng">エラー {r.validation.errors.length}</span>) : "-"}</td>
                <td>
                  {r.updatedBy} <span className="muted">{fmtTime(r.updatedAt)}</span>
                </td>
                <td>{r.approvedBy ? `${r.approvedBy} (${fmtTime(r.approvedAt)})` : "-"}</td>
                <td>{r.publishedBy ? `${r.publishedBy} (${fmtTime(r.publishedAt)}) keyId ${r.keyId}` : "-"}</td>
                <td className="row">
                  {canEdit(principal) && (
                    <button className="small" onClick={() => newRevision.mutate(r.revision, { onSuccess: (d) => nav(`/admin/manifests/${encodeURIComponent(id)}/rev/${d.revision}`) })}>
                      これを元に下書き
                    </button>
                  )}
                  {canApprove(principal) && r.state === "published" && m.defaultRevision !== r.revision && (
                    <button className="small" onClick={() => setDefault.mutate(r.revision)}>
                      既定にする (巻き戻し / 切替)
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <ErrorBox error={newRevision.error ?? setDefault.error} />
      </section>

      <section className="card">
        <h2>公開範囲</h2>
        <p className="muted">公開範囲と既定 revision は manifest 本文ではなくサーバのメタデータです (署名のやり直しが不要)。変更は approver 以上。</p>
        <div className="row wrap">
          <label>
            <input type="radio" checked={vis === "all"} onChange={() => setVis("all")} disabled={!canApprove(principal)} /> 全利用者
          </label>
          <label>
            <input type="radio" checked={vis === "groups"} onChange={() => setVis("groups")} disabled={!canApprove(principal)} /> グループ
          </label>
          <input placeholder="lab, control-team" value={visGroups} onChange={(e) => setVisGroups(e.target.value)} disabled={vis !== "groups" || !canApprove(principal)} list="groups" />
          <datalist id="groups">{(groups.data?.items ?? []).map((g) => <option key={g} value={g} />)}</datalist>
          {canApprove(principal) && (
            <button onClick={() => setVisibility.mutate(undefined)} disabled={setVisibility.isPending}>
              保存
            </button>
          )}
        </div>
        <ErrorBox error={setVisibility.error} />
      </section>
    </div>
  );
}
