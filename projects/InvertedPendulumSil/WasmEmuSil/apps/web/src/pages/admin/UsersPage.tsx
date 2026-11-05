import { useState } from "react";
import { useForm } from "react-hook-form";
import { post, put } from "../../api/client";
import { useInvalidatingMutation, useUsers } from "../../api/hooks";
import { ErrorBox, Loading, fmtTime } from "../../components/common";
import { ALL_ROLES, type Role, type User } from "../../api/types";

interface CreateForm {
  username: string;
  password: string;
  displayName: string;
  roles: Role[];
  groups: string;
}

function RoleEditor({ user }: { user: User }) {
  const [roles, setRoles] = useState<Role[]>(user.roles);
  const [groups, setGroups] = useState(user.groups.join(", "));
  const [pw, setPw] = useState("");
  const keys = [["/api/admin/users"], ["/api/admin/groups"]];
  const update = useInvalidatingMutation((body: Partial<{ roles: Role[]; groups: string[]; disabled: boolean }>) => put(`/api/admin/users/${user.id}`, body), keys);
  const reset = useInvalidatingMutation(() => post(`/api/admin/users/${user.id}/password`, { password: pw }), keys);
  return (
    <div className="row wrap">
      {ALL_ROLES.map((r) => (
        <label key={r} className="inline">
          <input type="checkbox" checked={roles.includes(r)} onChange={(e) => setRoles(e.target.checked ? [...roles, r] : roles.filter((x) => x !== r))} /> {r}
        </label>
      ))}
      <input value={groups} onChange={(e) => setGroups(e.target.value)} placeholder="groups (comma)" />
      <button className="small" onClick={() => update.mutate({ roles, groups: groups.split(",").map((s) => s.trim()).filter(Boolean) })}>
        保存
      </button>
      <button className="small" onClick={() => update.mutate({ disabled: !user.disabled })}>
        {user.disabled ? "有効化" : "無効化"}
      </button>
      <input type="password" value={pw} onChange={(e) => setPw(e.target.value)} placeholder="新パスワード" autoComplete="new-password" />
      <button className="small" onClick={() => reset.mutate(undefined, { onSuccess: () => setPw("") })} disabled={pw.length < 10}>
        再設定
      </button>
      <ErrorBox error={update.error ?? reset.error} />
    </div>
  );
}

export function UsersPage() {
  const { data, error, isLoading } = useUsers();
  const { register, handleSubmit, reset } = useForm<CreateForm>({ defaultValues: { roles: ["user"] } });
  const create = useInvalidatingMutation((f: CreateForm) => post("/api/admin/users", { ...f, groups: f.groups.split(",").map((s) => s.trim()).filter(Boolean) }), [["/api/admin/users"], ["/api/admin/groups"]]);
  return (
    <div>
      <h1>ユーザとロール</h1>
      <p className="muted">
        ロール: user (ポータルと起動) / viewer (管理画面の閲覧) / editor (下書き・部品登録) / approver (承認・公開・公開範囲) / admin (ユーザ・API キー)。<strong>編集と承認は別人</strong>。
      </p>
      <form className="card row wrap" onSubmit={handleSubmit((f) => create.mutate(f, { onSuccess: () => reset() }))}>
        <input placeholder="username" {...register("username", { required: true })} />
        <input placeholder="表示名" {...register("displayName")} />
        <input type="password" placeholder="初期パスワード (10 文字以上)" autoComplete="new-password" {...register("password", { required: true, minLength: 10 })} />
        {ALL_ROLES.map((r) => (
          <label key={r} className="inline">
            <input type="checkbox" value={r} {...register("roles")} /> {r}
          </label>
        ))}
        <input placeholder="groups (comma)" {...register("groups")} />
        <button type="submit" disabled={create.isPending}>
          作成
        </button>
        <ErrorBox error={create.error} />
      </form>
      <ErrorBox error={error} />
      {isLoading && <Loading />}
      <table>
        <thead>
          <tr>
            <th>ユーザ</th>
            <th>状態</th>
            <th>ロール / グループ</th>
          </tr>
        </thead>
        <tbody>
          {(data?.items ?? []).map((u) => (
            <tr key={u.id} className={u.disabled ? "muted" : ""}>
              <td>
                <strong>{u.username}</strong> {u.displayName && <span className="muted">({u.displayName})</span>}
                <div className="muted">作成 {fmtTime(u.createdAt)}</div>
              </td>
              <td>
                {u.disabled ? "無効" : "有効"}
                {u.mustChangePassword && <div className="muted">初回変更待ち</div>}
              </td>
              <td>
                <RoleEditor user={u} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
