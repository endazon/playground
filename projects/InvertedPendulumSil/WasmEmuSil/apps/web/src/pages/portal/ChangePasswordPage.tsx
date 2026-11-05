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
    try {
      await post("/api/auth/password", { currentPassword: f.currentPassword, newPassword: f.newPassword });
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
          新しいパスワード (10 文字以上)
          <input type="password" autoComplete="new-password" {...register("newPassword", { required: true, minLength: 10 })} />
        </label>
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
