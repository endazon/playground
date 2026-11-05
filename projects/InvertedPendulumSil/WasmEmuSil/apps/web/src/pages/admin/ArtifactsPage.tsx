import { useState } from "react";
import { useForm } from "react-hook-form";
import { api } from "../../api/client";
import { useArtifacts, useInvalidatingMutation } from "../../api/hooks";
import { canEdit, useAuth } from "../../auth/AuthContext";
import { ErrorBox, Loading, fmtBytes, fmtTime } from "../../components/common";
import type { Artifact } from "../../api/types";

interface UploadForm {
  name: string;
  version: string;
  kind: "part" | "firmware" | "ui";
  file: FileList;
}

export function ArtifactsPage() {
  const { principal } = useAuth();
  const { data, error, isLoading } = useArtifacts();
  const [open, setOpen] = useState<Artifact | null>(null);
  const { register, handleSubmit, reset } = useForm<UploadForm>({ defaultValues: { kind: "part" } });
  const upload = useInvalidatingMutation(async (f: UploadForm) => {
    const form = new FormData();
    form.set("name", f.name);
    form.set("version", f.version);
    form.set("kind", f.kind);
    form.set("file", f.file[0]);
    return api<Artifact>("/api/admin/artifacts", { method: "POST", form });
  }, [["/api/admin/artifacts"]]);

  return (
    <div>
      <h1>部品 (Artifacts)</h1>
      <p className="muted">wasm をアップロードすると、サーバが WIT の import と describe() を読んで保存します。同名同版の再登録はできません (イミュータブル)。UI バンドルは単一ファイルの tar.gz。</p>
      {canEdit(principal) && (
        <form
          className="card row wrap"
          onSubmit={handleSubmit((f) => {
            upload.mutate(f, { onSuccess: () => reset({ kind: f.kind, name: "", version: "" }) });
          })}
        >
          <label>
            name
            <input placeholder="plant-cartpole" {...register("name", { required: true, pattern: /^[a-z0-9][a-z0-9-]*$/ })} />
          </label>
          <label>
            version
            <input placeholder="0.1.0" {...register("version", { required: true })} />
          </label>
          <label>
            kind
            <select {...register("kind")}>
              <option value="part">part</option>
              <option value="firmware">firmware</option>
              <option value="ui">ui (tar.gz)</option>
            </select>
          </label>
          <label>
            file
            <input type="file" accept=".wasm,.tar.gz,.tgz" {...register("file", { required: true })} />
          </label>
          <button type="submit" disabled={upload.isPending}>
            {upload.isPending ? "検査中…" : "登録"}
          </button>
          <ErrorBox error={upload.error} />
        </form>
      )}
      <ErrorBox error={error} />
      {isLoading && <Loading />}
      <table>
        <thead>
          <tr>
            <th>ref</th>
            <th>kind</th>
            <th>world</th>
            <th>wasi import</th>
            <th>size</th>
            <th>sha256</th>
            <th>登録</th>
          </tr>
        </thead>
        <tbody>
          {(data?.items ?? []).map((a) => (
            <tr key={a.id} onClick={() => setOpen(open?.id === a.id ? null : a)} className={open?.id === a.id ? "hot" : ""}>
              <td className="mono">{a.ref}</td>
              <td>{a.kind}</td>
              <td className="mono">{a.world}</td>
              <td className="mono">{a.wasiImports.length ? a.wasiImports.join(", ") : <span className="muted">なし</span>}</td>
              <td>{fmtBytes(a.size)}</td>
              <td className="mono" title={a.sha256}>
                {a.sha256.slice(0, 12)}…
              </td>
              <td>
                {fmtTime(a.createdAt)} <span className="muted">{a.uploadedBy}</span>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {open && (
        <section className="card">
          <h2>
            {open.ref} <span className="muted">({open.kind})</span>
          </h2>
          <p className="mono">imports: {open.imports.join(", ") || "(なし)"}</p>
          {open.describe ? (
            <div className="row wrap">
              <div>
                <h3>入力</h3>
                <ul>
                  {open.describe.inputs.map((p) => (
                    <li key={p.name}>
                      <code>{p.name}</code> [{p.unit || "-"}] {p.doc}
                    </li>
                  ))}
                </ul>
              </div>
              <div>
                <h3>出力</h3>
                <ul>
                  {open.describe.outputs.map((p) => (
                    <li key={p.name}>
                      <code>{p.name}</code> [{p.unit || "-"}] {p.doc}
                    </li>
                  ))}
                </ul>
              </div>
              <div>
                <h3>パラメータ</h3>
                <ul>
                  {open.describe.params.map((p) => (
                    <li key={p.name}>
                      <code>{p.name}</code> {p.doc}
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          ) : (
            <p className="muted">describe はありません ({open.kind === "part" ? "wasi を import する部品の describe は M5 で" : open.kind})。</p>
          )}
        </section>
      )}
    </div>
  );
}
