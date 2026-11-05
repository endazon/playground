import type { EventNotice } from "@ipemu/ipc-types";

export interface EventLogProps {
  events: EventNotice[];
  max?: number;
}

/** イベント / ログの一覧 (新しいものが下) */
export function EventLog({ events, max = 200 }: EventLogProps) {
  const list = events.slice(-max);
  return (
    <div className="ipk-log" role="log">
      {list.length === 0 && <span className="sub">(イベントなし)</span>}
      {list.map((e, i) => (
        <div key={i} className={e.level === "error" ? "error" : e.level === "warn" ? "warn" : ""}>
          <span className="sub">[{(e.tNs * 1e-9).toFixed(3)}s {e.kind}]</span> {e.instance ? `${e.instance}: ` : ""}
          {e.message}
        </div>
      ))}
    </div>
  );
}
