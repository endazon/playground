import { useForm } from "react-hook-form";
import { useNavigate } from "react-router-dom";
import { useState } from "react";
import { post } from "../../api/client";
import { useAuth } from "../../auth/AuthContext";
import { ErrorBox } from "../../components/common";

interface Form {
  currentPassword: string;
  newPassword: string;
  confirm: string;
}

export function ChangePasswordPage() {
  const { refresh } = useAuth();
  const nav = useNavigate();
  const [error, setError] = useState<unknown>(null);
  const { register, handleSubmit, watch, formState } = useForm<Form>();
  const onSubmit = handleSubmit(async (f) => {
    setError(null);
    if (f.newPassword !== f.confirm) {
      setError(new Error("確認用のパスワードが一致しません"));
      return;
    }
    if (f.newPassword !== f.currentPassword && f.newPassword.length < 10) {
      setError(new Error("新しいパスワードは 10 文字以上にしてください (今と同じものを選ぶなら長さは問いません)"));
      return;
    }
    try {
      await post<{ mustChangePassword: boolean }>("/api/auth/password", {
        currentPassword: f.currentPassword,
        newPassword: f.newPassword,
      });
      await refresh();
      nav("/");
    } catch (e) {
      setError(e);
    }
  });
  return (
    <div className="login">
      <h1>パスワード変更</h1>
      <form onSubmit={onSubmit} className="card">
        <label>
          現在のパスワード
          <input type="password" autoComplete="current-password" {...register("currentPassword", { required: true })} />
        </label>
        <label>
          新しいパスワード (10 文字以上。今と同じものも選べます)
          <input
            type="password"
            autoComplete="new-password"
            {...register("newPassword", {
              required: true,
              // 今と同じパスワードを選び直すときは長さを問わない (初期パスワードを使い続ける用)。
              // そのかわり変更待ちは解けず、ログインのたびにこの画面に戻る
              validate: (v) => v === watch("currentPassword") || v.length >= 10,
            })}
          />
        </label>
        <p className="muted">
          今と同じパスワードを選ぶと、変更待ちのままになります (ログインのたびにこの画面が出ます)。
        </p>
        <label>
          確認
          <input type="password" autoComplete="new-password" {...register("confirm", { required: true, validate: (v) => v === watch("newPassword") })} />
        </label>
        <ErrorBox error={error} />
        <button type="submit" disabled={formState.isSubmitting}>
          変更
        </button>
      </form>
    </div>
  );
}
