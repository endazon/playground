import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useCatalog } from "../../api/hooks";
import { ErrorBox, Loading, fmtTime } from "../../components/common";
import type { CatalogItem } from "../../api/types";

/** 検索語 (名前 / id / 説明) で絞る */
export function filterCatalog(items: CatalogItem[], q: string): CatalogItem[] {
  const s = q.trim().toLowerCase();
  if (!s) return items;
  return items.filter((i) => [i.id, i.name, i.description].some((t) => t.toLowerCase().includes(s)));
}

export function CatalogPage() {
  const { data, error, isLoading } = useCatalog();
  const [q, setQ] = useState("");
  const items = useMemo(() => filterCatalog(data?.items ?? [], q), [data, q]);
  return (
    <div>
      <h1>カタログ</h1>
      <input className="search" placeholder="検索 (名前 / id / 説明)" value={q} onChange={(e) => setQ(e.target.value)} />
      <ErrorBox error={error} />
      {isLoading && <Loading />}
      {data && items.length === 0 && <p className="muted">公開された構成はありません。</p>}
      <div className="cards">
        {items.map((m) => (
          <Link key={m.id} to={`/m/${encodeURIComponent(m.id)}`} className="card catalog-card">
            <h2>{m.name}</h2>
            <p className="muted mono">{m.id}</p>
            <p>{m.description}</p>
            <p className="muted">
              既定 rev {m.defaultRevision ?? "-"} / {m.revisions.length} revision / 更新 {fmtTime(m.updatedAt)}
            </p>
            {m.requiresCapabilities.length > 0 ? <span className="pill warn">🔐 OS アクセスあり ({m.requiresCapabilities.length})</span> : <span className="pill">サンドボックス内</span>}
          </Link>
        ))}
      </div>
    </div>
  );
}
