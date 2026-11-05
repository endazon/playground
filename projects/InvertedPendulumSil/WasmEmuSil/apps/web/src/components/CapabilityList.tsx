import type { Capabilities, CapabilityEntry } from "../api/types";

/** capability 宣言を 1 行ずつの文字列にする (クライアントの同意キーと同じ `net.tcpConnect:host:port` 形) */
export function capabilityLines(caps: Capabilities | undefined): string[] {
  if (!caps) return [];
  const out: string[] = [];
  for (const h of caps.net?.tcpConnect ?? []) out.push(`net.tcpConnect:${h}`);
  for (const h of caps.net?.tcpListen ?? []) out.push(`net.tcpListen:${h}`);
  for (const h of caps.net?.udp ?? []) out.push(`net.udp:${h}`);
  for (const f of caps.fs ?? []) out.push(`fs.${f.mode}:${f.path}`);
  if (caps.clock) out.push("clock (wasi:clocks — 決定性の保証外)");
  if (caps.random) out.push("random (wasi:random — 決定性の保証外)");
  for (const h of caps.hostExt ?? []) out.push(`hostExt:${h}`);
  return out;
}

export function CapabilityList({ entries }: { entries: CapabilityEntry[] }) {
  if (entries.length === 0) return <p className="muted">OS アクセスの要求はありません (純粋な部品だけで構成されています)。</p>;
  return (
    <ul className="caps">
      {entries.map((e) => (
        <li key={e.instance}>
          <strong>{e.instance}</strong>
          <ul>
            {capabilityLines(e.capabilities).map((l) => (
              <li key={l}>
                <code>{l}</code>
              </li>
            ))}
          </ul>
        </li>
      ))}
    </ul>
  );
}
