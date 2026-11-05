import type { ValidationReport } from "../api/types";

export function Diagnostics({ report }: { report: ValidationReport | null | undefined }) {
  if (!report) return <p className="muted">まだ検証していません。</p>;
  return (
    <div className="diagnostics">
      <p className={report.ok ? "ok" : "ng"}>
        {report.ok ? "検証 OK" : `エラー ${report.errors.length} 件`}
        {report.warnings.length > 0 ? ` / 警告 ${report.warnings.length} 件` : ""}
        <span className="muted"> ({new Date(report.validatedAt).toLocaleString()})</span>
      </p>
      <ul>
        {report.errors.map((e, i) => (
          <li key={`e${i}`} className="diag-error">
            <code>{e.rule}</code> {e.message}
          </li>
        ))}
        {report.warnings.map((w, i) => (
          <li key={`w${i}`} className="diag-warn">
            <code>{w.rule}</code> {w.message}
          </li>
        ))}
      </ul>
    </div>
  );
}
