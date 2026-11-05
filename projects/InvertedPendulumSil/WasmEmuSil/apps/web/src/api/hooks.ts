// TanStack Query のフック。キーは URL をそのまま使う (無効化が素直になる)。
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { del, get, post, put } from "./client";
import type {
  ApiKey,
  Artifact,
  AuditEntry,
  CatalogItem,
  ClientInfo,
  Launch,
  ManifestAdmin,
  RevisionDetail,
  TicketResponse,
  User,
  ValidationReport,
} from "./types";

type Items<T> = { items: T[] };

export const useCatalog = () => useQuery({ queryKey: ["/api/v1/catalog"], queryFn: () => get<Items<CatalogItem>>("/api/v1/catalog") });
export const useMyLaunches = () => useQuery({ queryKey: ["/api/v1/launches/mine"], queryFn: () => get<Items<Launch>>("/api/v1/launches/mine"), refetchInterval: 10_000 });
export const usePublicManifest = (id: string, rev: number | "default") =>
  useQuery({ queryKey: ["/api/v1/manifests", id, rev], queryFn: () => get<import("./types").ManifestBody>(`/api/v1/manifests/${encodeURIComponent(id)}?rev=${rev}`) });

export const useArtifacts = () => useQuery({ queryKey: ["/api/admin/artifacts"], queryFn: () => get<Items<Artifact>>("/api/admin/artifacts") });
export const useManifests = () => useQuery({ queryKey: ["/api/admin/manifests"], queryFn: () => get<Items<ManifestAdmin>>("/api/admin/manifests") });
export const useManifest = (id: string) => useQuery({ queryKey: ["/api/admin/manifests", id], queryFn: () => get<ManifestAdmin>(`/api/admin/manifests/${encodeURIComponent(id)}`) });
export const useRevision = (id: string, rev: number) =>
  useQuery({ queryKey: ["/api/admin/manifests", id, rev], queryFn: () => get<RevisionDetail>(`/api/admin/manifests/${encodeURIComponent(id)}/revisions/${rev}`) });
export const useUsers = () => useQuery({ queryKey: ["/api/admin/users"], queryFn: () => get<Items<User>>("/api/admin/users") });
export const useGroups = () => useQuery({ queryKey: ["/api/admin/groups"], queryFn: () => get<Items<string>>("/api/admin/groups") });
export const useApiKeys = () => useQuery({ queryKey: ["/api/admin/api-keys"], queryFn: () => get<Items<ApiKey>>("/api/admin/api-keys") });
export const useAudit = () => useQuery({ queryKey: ["/api/admin/audit"], queryFn: () => get<Items<AuditEntry>>("/api/admin/audit?limit=300"), refetchInterval: 15_000 });
export const useLaunches = () => useQuery({ queryKey: ["/api/admin/launches"], queryFn: () => get<Items<Launch>>("/api/admin/launches?limit=300"), refetchInterval: 15_000 });
export const useClients = () => useQuery({ queryKey: ["/api/admin/clients"], queryFn: () => get<Items<ClientInfo>>("/api/admin/clients"), refetchInterval: 15_000 });

/// 成功したら関連するクエリを無効化する mutation のヘルパ。
export function useInvalidatingMutation<TArgs, TResult>(fn: (args: TArgs) => Promise<TResult>, keys: unknown[][]) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: fn,
    onSuccess: () => keys.forEach((k) => qc.invalidateQueries({ queryKey: k })),
  });
}

export const manifestKeys = (id: string) => [["/api/admin/manifests"], ["/api/admin/manifests", id], ["/api/v1/catalog"]];

export const createTicket = (manifestId: string, revision: number | null) => post<TicketResponse>("/api/v1/launch-tickets", { manifestId, revision });
export const validateAdhoc = (body: unknown) => post<{ report: ValidationReport; body: unknown }>("/api/admin/validate", body);
export { del, get, post, put };
