import { useEffect, useMemo, useState } from "react";
import { useParams } from "react-router-dom";
import { createTicket, useCatalog, usePublicManifest } from "../../api/hooks";
import { useAuth } from "../../auth/AuthContext";
import { CapabilityList } from "../../components/CapabilityList";
import { ErrorBox, Loading, StateBadge, fmtTime } from "../../components/common";
import { TopologyView } from "../../components/TopologyView";
import { currentServer, hasShellBridge, launchManifest, type LaunchResult } from "../../launch";
import type { ManifestBody } from "../../api/types";

/** パラメータの要約: 部品ごとに主要な数値パラメータを数個 */
export function summarizeParams(m: ManifestBody): { instance: string; items: string[] }[] {
  return m.components.map((c) => {
    const items: string[] = [];
    const p = c.params ?? {};
    for (const [k, v] of Object.entries(p)) {
      if (typeof v === "number" || typeof v === "string" || typeof v === "boolean") items.push(`${k}=${v}`);
      else if (v && typeof v === "object" && !Array.isArray(v)) items.push(`${k}={…${Object.keys(v).length}}`);
      if (items.length >= 6) break;
    }
    if (c.firmware) items.unshift(`firmware ${c.firmware.default ?? "(なし)"} / slot ${c.firmware.slot}`);
    return { instance: c.instance, items };
  });
}

export function ManifestDetailPage() {
  const { id = "" } = useParams();
  const { token } = useAuth();
  const catalog = useCatalog();
  const item = catalog.data?.items.find((i) => i.id === id);
  const [rev, setRev] = useState<number | "default">("default");
  const manifest = usePublicManifest(id, rev);
  const [result, setResult] = useState<LaunchResult | null>(null);
  const [launchError, setLaunchError] = useState<unknown>(null);
  const [showInstallHint, setShowInstallHint] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);

  // 通常ブラウザで ipemu:// に遷移してもページが見えたままなら、クライアント未インストールの可能性が高い
  useEffect(() => {
    if (result?.mode !== "protocol") return;
    const t = setTimeout(() => {
      if (document.visibilityState === "visible") setShowInstallHint(true);
    }, 2500);
    return () => clearTimeout(t);
  }, [result]);

  const caps = useMemo(() => {
    const m = manifest.data;
    if (!m) return item?.requiresCapabilities ?? [];
    return m.components.filter((c) => c.capabilities).map((c) => ({ instance: c.instance, capabilities: c.capabilities! }));
  }, [manifest.data, item]);

  const doLaunch = async () => {
    setLaunchError(null);
    setShowInstallHint(false);
    try {
      const r = await launchManifest(id, rev, { server: currentServer(), token: token ?? "", createTicket });
      setResult(r);
    } catch (e) {
      setLaunchError(e);
    }
  };

  if (catalog.isLoading) return <Loading />;
  if (!item) return <ErrorBox error={catalog.error ?? new Error("この構成は公開されていません")} />;
  const selRev = rev === "default" ? item.revisions.find((r) => r.revision === item.defaultRevision) : item.revisions.find((r) => r.revision === rev);
  return (
    <div className="detail">
      <h1>{item.name}</h1>
      <p className="muted mono">{item.id}</p>
      <p>{item.description}</p>

      <section className="card launch-card">
        <div className="row">
          <label>
            revision
            <select value={String(rev)} onChange={(e) => setRev(e.target.value === "default" ? "default" : Number(e.target.value))}>
              <option value="default">既定 (rev {item.defaultRevision ?? "-"})</option>
              {item.revisions.map((r) => (
                <option key={r.revision} value={r.revision}>
                  rev {r.revision} ({r.state}
                  {r.signed ? "" : "・署名なし"})
                </option>
              ))}
            </select>
          </label>
          {selRev && (
            <span>
              <StateBadge state={selRev.state} /> {selRev.publishedAt ? `公開 ${fmtTime(selRev.publishedAt)}` : "未公開 (検証用。署名なしで配信されます)"}
            </span>
          )}
          <div className="spacer" />
          <button className="primary" onClick={() => void doLaunch()} disabled={!token}>
            ▶ 起動
          </button>
        </div>
        <p className="muted">{hasShellBridge() ? "シェル内: エンジンに直接 session.launch を送ります。" : "通常のブラウザ: 単回チケット (60 秒) を発行して ipemu:// リンクでクライアントを起動します。"}</p>
        <ErrorBox error={launchError} />
        {result?.mode === "bridge" && <p className="ok">起動要求をエンジンに送りました。シミュレータのタブが開きます。</p>}
        {result?.mode === "protocol" && (
          <div>
            <p className="ok">
              クライアントを起動しています… 開かない場合は <a href={result.url}>こちら</a> (チケットは {fmtTime(result.expiresAt)} まで・1 回限り)。
            </p>
            {showInstallHint && (
              <div className="banner">
                クライアント (ipemu シェル) がインストールされていないようです。管理者から配布された zip を展開し、一度起動して <code>ipemu://</code> を登録してから、もう一度「起動」を押してください。
              </div>
            )}
          </div>
        )}
      </section>

      <section>
        <h2>要求する OS アクセス (capability)</h2>
        <CapabilityList entries={caps} />
        {manifest.data?.approval && (
          <p className="muted">
            承認: {manifest.data.approval.approvedBy} ({fmtTime(manifest.data.approval.approvedAt)}){manifest.data.approval.reason ? ` — ${manifest.data.approval.reason}` : ""}
          </p>
        )}
      </section>

      <section>
        <h2>トポロジ</h2>
        <ErrorBox error={manifest.error} />
        {manifest.data && (
          <div className="topology-wrap">
            <TopologyView components={manifest.data.components} wires={manifest.data.wires} highlight={selected} onSelect={setSelected} />
          </div>
        )}
      </section>

      {manifest.data && (
        <section>
          <h2>パラメータの要約</h2>
          <table>
            <tbody>
              {summarizeParams(manifest.data).map((row) => (
                <tr key={row.instance} className={selected === row.instance ? "hot" : ""} onClick={() => setSelected(row.instance)}>
                  <th>{row.instance}</th>
                  <td className="mono">{row.items.join("  ")}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="muted">
            scheduler: baseStep {manifest.data.scheduler.baseStepNs} ns, pacing {manifest.data.scheduler.pacing ?? "realtime"}
            {manifest.data.signature ? ` / 署名 keyId ${manifest.data.signature.keyId}` : " / 署名なし"}
          </p>
        </section>
      )}
    </div>
  );
}
