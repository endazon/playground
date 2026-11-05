import { useForm } from "react-hook-form";
import { useLocation, useNavigate } from "react-router-dom";
import { useState } from "react";
import { useAuth } from "../auth/AuthContext";
import { ErrorBox } from "../components/common";

interface Form {
  username: string;
  password: string;
}

export function LoginPage() {
  const { login } = useAuth();
  const nav = useNavigate();
  const loc = useLocation() as { state?: { from?: string } };
  const [error, setError] = useState<unknown>(null);
  const { register, handleSubmit, formState } = useForm<Form>();
  const onSubmit = handleSubmit(async (f) => {
    setError(null);
    try {
      const p = await login(f.username, f.password);
      nav(p.mustChangePassword ? "/me/password" : (loc.state?.from ?? "/"), { replace: true });
    } catch (e) {
      setError(e);
    }
  });
  return (
    <div className="login">
      <h1>ipemu ポータル</h1>
      <p className="muted">ログインして、カタログから構成を選び、エミュレータを起動します。</p>
      <form onSubmit={onSubmit} className="card">
        <label>
          ユーザ名
          <input autoComplete="username" {...register("username", { required: true })} />
        </label>
        <label>
          パスワード
          <input type="password" autoComplete="current-password" {...register("password", { required: true })} />
        </label>
        <ErrorBox error={error} />
        <button type="submit" disabled={formState.isSubmitting}>
          ログイン
        </button>
      </form>
    </div>
  );
}
