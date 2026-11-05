import { useMyLaunches } from "../../api/hooks";
import { ErrorBox, Loading, fmtTime } from "../../components/common";

export function MyLaunchesPage() {
  const { data, error, isLoading } = useMyLaunches();
  return (
    <div>
      <h1>自分の起動履歴</h1>
      <ErrorBox error={error} />
      {isLoading && <Loading />}
      <table>
        <thead>
          <tr>
            <th>開始</th>
            <th>構成</th>
            <th>rev</th>
            <th>資格情報</th>
            <th>端末</th>
            <th>結果</th>
            <th>終了</th>
          </tr>
        </thead>
        <tbody>
          {(data?.items ?? []).map((l) => (
            <tr key={l.id}>
              <td>{fmtTime(l.startedAt)}</td>
              <td className="mono">{l.manifestId}</td>
              <td>{l.revision}</td>
              <td>{l.credentialKind}</td>
              <td className="mono">{l.clientInfo ? `${l.clientInfo.os ?? ""}/${l.clientInfo.arch ?? ""} ${l.clientInfo.engineVersion ?? ""}` : "-"}</td>
              <td>{l.outcome ?? "-"}</td>
              <td>{fmtTime(l.endedAt)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {data && data.items.length === 0 && <p className="muted">まだ起動していません。</p>}
    </div>
  );
}
