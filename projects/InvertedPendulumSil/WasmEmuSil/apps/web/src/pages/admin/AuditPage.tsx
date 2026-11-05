import { useState } from "react";
import { useAudit, useLaunches } from "../../api/hooks";
import { ErrorBox, Loading, fmtTime } from "../../components/common";

export function AuditPage() {
  const audit = useAudit();
  const launches = useLaunches();
  const [tab, setTab] = useState<"audit" | "launches">("audit");
  return (
    <div>
      <h1>監査</h1>
      <div className="tabs">
        <button className={tab === "audit" ? "active" : ""} onClick={() => setTab("audit")}>
          変更操作
        </button>
        <button className={tab === "launches" ? "active" : ""} onClick={() => setTab("launches")}>
          起動 / 終了 / FAULT
        </button>
      </div>
      {tab === "audit" && (
        <>
          <ErrorBox error={audit.error} />
          {audit.isLoading && <Loading />}
          <table>
            <thead>
              <tr>
                <th>いつ</th>
                <th>誰が</th>
                <th>何を</th>
                <th>対象</th>
                <th>詳細</th>
              </tr>
            </thead>
            <tbody>
              {(audit.data?.items ?? []).map((a) => (
                <tr key={a.id}>
                  <td>{fmtTime(a.at)}</td>
                  <td>{a.actorName}</td>
                  <td className="mono">{a.action}</td>
                  <td className="mono">{a.target}</td>
                  <td className="mono small">{a.detail ? JSON.stringify(a.detail) : ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
      {tab === "launches" && (
        <>
          <ErrorBox error={launches.error} />
          {launches.isLoading && <Loading />}
          <table>
            <thead>
              <tr>
                <th>開始</th>
                <th>誰が</th>
                <th>構成</th>
                <th>資格情報</th>
                <th>端末</th>
                <th>結果</th>
                <th>終了</th>
                <th>詳細</th>
              </tr>
            </thead>
            <tbody>
              {(launches.data?.items ?? []).map((l) => (
                <tr key={l.id} className={l.outcome === "fault" ? "ng" : ""}>
                  <td>{fmtTime(l.startedAt)}</td>
                  <td>
                    {l.principalName} <span className="muted">({l.principalKind})</span>
                  </td>
                  <td className="mono">
                    {l.manifestId}@{l.revision}
                  </td>
                  <td>{l.credentialKind}</td>
                  <td className="mono">{l.clientInfo ? `${l.clientInfo.os ?? ""}/${l.clientInfo.arch ?? ""} ${l.clientInfo.engineVersion ?? ""}` : "-"}</td>
                  <td>{l.outcome ?? "-"}</td>
                  <td>{fmtTime(l.endedAt)}</td>
                  <td className="mono small">{l.detail ? JSON.stringify(l.detail) : ""}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </div>
  );
}
