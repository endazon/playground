import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter, Navigate, Outlet, Route, Routes, useLocation } from "react-router-dom";
import { AuthProvider, canViewAdmin, useAuth } from "./auth/AuthContext";
import { AdminLayout, Layout } from "./components/Layout";
import { Loading } from "./components/common";
import { LoginPage } from "./pages/LoginPage";
import { CatalogPage } from "./pages/portal/CatalogPage";
import { ManifestDetailPage } from "./pages/portal/ManifestDetailPage";
import { MyLaunchesPage } from "./pages/portal/MyLaunchesPage";
import { ChangePasswordPage } from "./pages/portal/ChangePasswordPage";
import { ArtifactsPage } from "./pages/admin/ArtifactsPage";
import { ManifestsPage } from "./pages/admin/ManifestsPage";
import { ManifestAdminPage } from "./pages/admin/ManifestAdminPage";
import { RevisionEditorPage } from "./pages/admin/RevisionEditorPage";
import { ApprovalsPage } from "./pages/admin/ApprovalsPage";
import { UsersPage } from "./pages/admin/UsersPage";
import { ClientsPage } from "./pages/admin/ClientsPage";
import { AuditPage } from "./pages/admin/AuditPage";
import "./styles.css";

const qc = new QueryClient({ defaultOptions: { queries: { retry: 1, staleTime: 5_000 } } });

function RequireAuth() {
  const { principal, loading } = useAuth();
  const loc = useLocation();
  if (loading) return <Loading />;
  if (!principal) return <Navigate to="/login" state={{ from: loc.pathname }} replace />;
  return <Outlet />;
}

function RequireAdmin() {
  const { principal } = useAuth();
  if (!canViewAdmin(principal)) return <Navigate to="/" replace />;
  return <AdminLayout />;
}

export function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route element={<RequireAuth />}>
        <Route element={<Layout />}>
          <Route index element={<CatalogPage />} />
          <Route path="/m/:id" element={<ManifestDetailPage />} />
          <Route path="/me/launches" element={<MyLaunchesPage />} />
          <Route path="/me/password" element={<ChangePasswordPage />} />
          <Route path="/admin" element={<RequireAdmin />}>
            <Route index element={<Navigate to="/admin/manifests" replace />} />
            <Route path="artifacts" element={<ArtifactsPage />} />
            <Route path="manifests" element={<ManifestsPage />} />
            <Route path="manifests/:id" element={<ManifestAdminPage />} />
            <Route path="manifests/:id/rev/:rev" element={<RevisionEditorPage />} />
            <Route path="approvals" element={<ApprovalsPage />} />
            <Route path="users" element={<UsersPage />} />
            <Route path="clients" element={<ClientsPage />} />
            <Route path="audit" element={<AuditPage />} />
          </Route>
        </Route>
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={qc}>
      <AuthProvider>
        <BrowserRouter>
          <App />
        </BrowserRouter>
      </AuthProvider>
    </QueryClientProvider>
  </React.StrictMode>,
);
