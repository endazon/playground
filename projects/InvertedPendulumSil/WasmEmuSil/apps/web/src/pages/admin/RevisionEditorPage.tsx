import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { del, post, put } from "../../api/client";
import { manifestKeys, useInvalidatingMutation, useRevision, validateAdhoc } from "../../api/hooks";
import { canApprove, canEdit, useAuth } from "../../auth/AuthContext";
import { CapabilityList } from "../../components/CapabilityList";
import { Diagnostics } from "../../components/Diagnostics";
import { JsonEditor } from "../../components/JsonEditor";
import { ErrorBox, Loading, StateBadge, fmtTime } from "../../components/common";
import { TopologyView } from "../../components/TopologyView";
import type { ManifestBody, RevisionDetail, ValidationReport } from "../../api/types";

/** エディタの文字列を manifest として読む。壊れていれば null */
export function parseBody(text: string): ManifestBody | null {
  try {
    const v = JSON.parse(text);
    return v && typeof v === "object" && Array.isArray(v.components) ? (v as ManifestBody) : null;
  } catch {
    return null;
  }
}

export function RevisionEditorPage() {
  const { id = "", rev = "1" } = useParams();
  const revision = Number(rev);
  const { principal } = useAuth();
  const { data, error, isLoading } = useRevision(id, revision);
  const [text, setText] = useState("");
  const [dirty, setDirty] = useState(false);
  const [report, setReport] = useState<ValidationReport | null>(null);
  const [reason, setReason] = useState("");
  const [tab, setTab] = useState<"json" | "topology" | "signed">("json");
  const [selected, setSelected] = useState<string | null>(null);

  useEffect(() => {
    if (data && !dirty) {
      setText(JSON.stringify(data.body, null, 2));
      setReport(data.validation);
    }
  }, [data, dirty]);

  const parsed = useMemo(() => parseBody(text), [text]);
  const editable = data?.state === "draft" && canEdit(principal);
  const keys = [...manifestKeys(id), ["/api/admin/manifests", id, revision]];
  const base = `/api/admin/manifests/${encodeURIComponent(id)}/revisions/${revision}`;

  const save = useInvalidatingMutation(async () => {
    const body = JSON.parse(text);
    const r = await put<RevisionDetail>(base, { body });
    setDirty(false);
    return r;
  }, keys);
  const validate = useInvalidatingMutation(async () => {
    const r = await validateAdhoc(JSON.parse(text));
    setReport(r.report);
    return r;
  }, []);
  const approve = useInvalidatingMutation(() => post<RevisionDetail>(`${base}/approve`, { reason }), keys);
  const publish = useInvalidatingMutation(() => post<RevisionDetail>(`${base}/publish`), keys);
  const remove = useInvalidatingMutation(() => del(base), keys);

  if (isLoading) return <Loading />;
  if (!data) return <ErrorBox error={error ?? new Error("revision がありません")} />;
  const caps = (parsed ?? data.body).components.filter((c) => c.capabilities).map((c) => ({ instance: c.instance, capabilities: c.capabilities! }));
  const selfEdited = data.updatedBy === principal?.name;

  return (
    <div>
      <h1>
        <Link to={`/admin/manifests/${encodeURIComponent(id)}`}>{id}</Link> rev {revision} <StateBadge state={data.state} />
      </h1>
      <p className="muted">
        作成 {data.createdBy} {fmtTime(data.createdAt)} / 最終編集 {data.updatedBy} {fmtTime(data.updatedAt)}
        {data.approvedBy && ` / 承認 ${data.approvedBy} ${fmtTime(data.approvedAt)} (capabilitiesHash ${data.capabilitiesHash?.slice(0, 12)}…)`}
        {data.publishedBy && ` / 公開 ${data.publishedBy} ${fmtTime(data.publishedAt)} keyId ${data.keyId}`}
      </p>

      <div className="toolbar row wrap">
        <button onClick={() => validate.mutate(undefined)} disabled={!parsed || validate.isPending}>
          検証
        </button>
        {editable && (
          <>
            <button className="primary" onClick={() => save.mutate(undefined)} disabled={!parsed || !dirty || save.isPending}>
              保存 {dirty ? "*" : ""}
            </button>
            <button
              className="danger"
              onClick={() => {
                if (confirm(`rev ${revision} の下書きを削除しますか?`)) remove.mutate(undefined);
              }}
            >
              下書きを削除
            </button>
          </>
        )}
        <div className="spacer" />
        {data.state === "draft" && canApprove(principal) && (
          <>
            <input placeholder="承認理由" value={reason} onChange={(e) => setReason(e.target.value)} />
            <button onClick={() => approve.mutate(undefined)} disabled={approve.isPending || dirty || selfEdited} title={selfEdited ? "編集した本人は承認できません (editor ≠ approver)" : ""}>
              承認
            </button>
          </>
        )}
        {data.state === "approved" && canApprove(principal) && (
          <button className="primary" onClick={() => publish.mutate(undefined)} disabled={publish.isPending}>
            公開 (署名)
          </button>
        )}
      </div>
      {selfEdited && data.state === "draft" && canApprove(principal) && <p className="muted">この下書きはあなたが最後に編集したため、別の承認者が承認する必要があります。</p>}
      <ErrorBox error={save.error ?? validate.error ?? approve.error ?? publish.error ?? remove.error} />
      {remove.isSuccess && <p className="ok">削除しました。</p>}

      <div className="tabs">
        <button className={tab === "json" ? "active" : ""} onClick={() => setTab("json")}>
          JSON
        </button>
        <button className={tab === "topology" ? "active" : ""} onClick={() => setTab("topology")}>
          トポロジ
        </button>
        {data.signedBody && (
          <button className={tab === "signed" ? "active" : ""} onClick={() => setTab("signed")}>
            署名付き全文
          </button>
        )}
      </div>
      {tab === "json" && (
        <JsonEditor
          value={text}
          readOnly={!editable}
          onChange={(v) => {
            setText(v);
            setDirty(true);
          }}
        />
      )}
      {tab === "topology" &&
        (parsed ? (
          <div className="topology-wrap">
            <TopologyView components={parsed.components} wires={parsed.wires} descs={report?.descs} highlight={selected} onSelect={setSelected} />
          </div>
        ) : (
          <p className="ng">JSON が壊れています。</p>
        ))}
      {tab === "signed" && data.signedBody && <JsonEditor value={JSON.stringify(data.signedBody, null, 2)} readOnly height="50vh" />}

      <div className="row wrap top">
        <section className="card grow">
          <h2>検証結果</h2>
          <Diagnostics report={report} />
        </section>
        <section className="card grow">
          <h2>capability 宣言 (承認の対象)</h2>
          <CapabilityList entries={caps} />
        </section>
      </div>
    </div>
  );
}
