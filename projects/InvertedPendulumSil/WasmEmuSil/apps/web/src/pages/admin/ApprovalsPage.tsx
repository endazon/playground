import { Link } from "react-router-dom";
import { useManifests } from "../../api/hooks";
import { CapabilityList } from "../../components/CapabilityList";
import { ErrorBox, Loading, StateBadge, fmtTime } from "../../components/common";
import { useRevision } from "../../api/hooks";

function DraftCaps({ id, rev }: { id: string; rev: number }) {
  const { data } = useRevision(id, rev);
  if (!data) return <Loading />;
  const caps = data.body.components.filter((c) => c.capabilities).map((c) => ({ instance: c.instance, capabilities: c.capabilities! }));
  return <CapabilityList entries={caps} />;
}

/** capability 承認: 未公開 (draft / approved) の revision が宣言する capability を instance ごとに一覧する。 */
export function ApprovalsPage() {
  const { data, error, isLoading } = useManifests();
  const pending = (data?.items ?? []).flatMap((m) => m.revisions.filter((r) => r.state !== "published").map((r) => ({ m, r })));
  const published = (data?.items ?? []).flatMap((m) => m.revisions.filter((r) => r.state === "published").map((r) => ({ m, r })));
  return (
    <div>
      <h1>capability 承認</h1>
      <p className="muted">承認は本文 (capability 宣言を含む) のハッシュに紐づきます。承認後に本文を変えると新しい下書きになります。編集した本人は承認できません。</p>
      <ErrorBox error={error} />
      {isLoading && <Loading />}
      <h2>承認待ち / 公開待ち</h2>
      {pending.length === 0 && <p className="muted">ありません。</p>}
      {pending.map(({ m, r }) => (
        <section key={`${m.id}@${r.revision}`} className="card">
          <h3>
            <Link to={`/admin/manifests/${encodeURIComponent(m.id)}/rev/${r.revision}`}>
              {m.id} rev {r.revision}
            </Link>{" "}
            <StateBadge state={r.state} /> <span className="muted">最終編集 {r.updatedBy} {fmtTime(r.updatedAt)}</span>
            {r.validation && !r.validation.ok && <span className="ng"> 検証エラーあり</span>}
          </h3>
          <DraftCaps id={m.id} rev={r.revision} />
          {r.approvedBy && (
            <p className="muted">
              承認済み: {r.approvedBy} ({fmtTime(r.approvedAt)}) {r.approvalReason}
            </p>
          )}
        </section>
      ))}
      <h2>公開済み</h2>
      <table>
        <thead>
          <tr>
            <th>構成</th>
            <th>rev</th>
            <th>承認者</th>
            <th>理由</th>
            <th>capabilitiesHash</th>
            <th>公開</th>
          </tr>
        </thead>
        <tbody>
          {published.map(({ m, r }) => (
            <tr key={`${m.id}@${r.revision}`}>
              <td className="mono">
                <Link to={`/admin/manifests/${encodeURIComponent(m.id)}/rev/${r.revision}`}>{m.id}</Link>
              </td>
              <td>{r.revision}</td>
              <td>
                {r.approvedBy} <span className="muted">{fmtTime(r.approvedAt)}</span>
              </td>
              <td>{r.approvalReason}</td>
              <td className="mono" title={r.capabilitiesHash ?? ""}>
                {r.capabilitiesHash?.slice(0, 16)}…
              </td>
              <td>
                {r.publishedBy} <span className="muted">{fmtTime(r.publishedAt)}</span>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
