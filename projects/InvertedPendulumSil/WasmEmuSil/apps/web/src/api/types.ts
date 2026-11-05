// サーバ API の型 (crates/ipemu-server の JSON と対応)

export type Role = "user" | "viewer" | "editor" | "approver" | "admin";
export const ALL_ROLES: Role[] = ["user", "viewer", "editor", "approver", "admin"];

export interface Principal {
  kind: "user" | "api_key";
  id: string;
  name: string;
  roles: Role[];
  groups: string[];
  allowedManifests: string[];
  credential: "session" | "ticket" | "api_key";
  mustChangePassword: boolean;
}

export interface LoginResponse {
  token: string;
  expiresAt: string;
  principal: Principal;
}

export interface CatalogRevision {
  revision: number;
  state: "draft" | "approved" | "published";
  name: string;
  publishedAt: string | null;
  approvedBy: string | null;
  signed: boolean;
}

export interface CapabilityEntry {
  instance: string;
  capabilities: Capabilities;
}

export interface Capabilities {
  net?: { tcpConnect?: string[]; tcpListen?: string[]; udp?: string[] };
  fs?: { path: string; mode: "ro" | "rw" }[];
  clock?: boolean;
  random?: boolean;
  hostExt?: string[];
}

export interface CatalogItem {
  id: string;
  name: string;
  description: string;
  defaultRevision: number | null;
  revisions: CatalogRevision[];
  requiresCapabilities: CapabilityEntry[];
  updatedAt: string;
}

export interface ManifestBody {
  manifestVersion: 1;
  id: string;
  revision: number;
  name: string;
  description?: string;
  scheduler: { baseStepNs: number; pacing?: string; speed?: number; snapshotDivider?: number; maxSlipMs?: number };
  artifacts: Record<string, { sha256?: string; url?: string; path?: string; size?: number; kind?: "part" | "firmware" | "ui" }>;
  components: ComponentSpec[];
  wires: Wire[];
  ui?: { bundle?: string; views?: string[] };
  approval?: { approvedBy: string; approvedAt: string; reason?: string; capabilitiesHash: string };
  signature?: { alg: string; keyId: string; value: string };
}

export interface ComponentSpec {
  instance: string;
  artifact?: string;
  kind?: string;
  rateDivider?: number;
  params?: Record<string, unknown>;
  firmware?: { slot: string; world: string; default?: string; allowOverride?: string[]; params?: Record<string, unknown> };
  capabilities?: Capabilities;
}

export interface Wire {
  from: string;
  to: string;
  latency?: 0 | 1;
  gain?: number;
  offset?: number;
}

export interface PortDesc {
  name: string;
  kind: "scalar" | "bytes";
  unit: string;
  default: number;
  doc: string;
}

export interface PartDesc {
  name: string;
  version: string;
  inputs: PortDesc[];
  outputs: PortDesc[];
  params: { name: string; schema_json: string; doc: string }[];
  deterministic: boolean;
  doc: string;
}

export interface DiagItem {
  rule: string;
  message: string;
}

export interface ValidationReport {
  ok: boolean;
  errors: DiagItem[];
  warnings: DiagItem[];
  descs: { instance: string; desc: PartDesc }[];
  validatedAt: string;
}

export interface RevisionSummary {
  manifestId: string;
  revision: number;
  state: "draft" | "approved" | "published";
  bodySha256: string;
  createdBy: string;
  updatedBy: string;
  createdAt: string;
  updatedAt: string;
  approvedBy: string | null;
  approvedAt: string | null;
  approvalReason: string | null;
  capabilitiesHash: string | null;
  publishedBy: string | null;
  publishedAt: string | null;
  keyId: string | null;
  validation: ValidationReport | null;
}

export interface RevisionDetail extends RevisionSummary {
  body: ManifestBody;
  signedBody: ManifestBody | null;
}

export interface ManifestAdmin {
  id: string;
  name: string;
  description: string;
  visibility: "all" | "groups";
  visibilityGroups: string[];
  defaultRevision: number | null;
  createdBy: string;
  createdAt: string;
  updatedAt: string;
  revisions: RevisionSummary[];
}

export interface Artifact {
  id: string;
  name: string;
  version: string;
  ref: string;
  kind: "part" | "firmware" | "ui";
  sha256: string;
  size: number;
  contentType: string;
  world: string;
  describe: PartDesc | null;
  wasiImports: string[];
  imports: string[];
  uploadedBy: string;
  createdAt: string;
  url: string;
}

export interface User {
  id: string;
  username: string;
  displayName: string;
  roles: Role[];
  groups: string[];
  mustChangePassword: boolean;
  disabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ApiKey {
  id: string;
  name: string;
  prefix: string;
  allowedManifests: string[];
  createdBy: string;
  createdAt: string;
  revokedAt: string | null;
  lastUsedAt: string | null;
  key?: string; // 作成時のみ
}

export interface Launch {
  id: string;
  manifestId: string;
  revision: number;
  principalKind: string;
  principalName: string;
  credentialKind: string;
  clientInfo: Record<string, unknown> | null;
  startedAt: string;
  endedAt: string | null;
  outcome: string | null;
  detail: Record<string, unknown> | null;
}

export interface ClientInfo {
  id: string;
  clientId: string;
  principalKind: string;
  principalName: string;
  engineVersion: string;
  witVersions: string[];
  os: string;
  arch: string;
  firstSeen: string;
  lastSeen: string;
  helloCount: number;
  running: { manifestId: string; revision: number; startedAt: string }[];
}

export interface AuditEntry {
  id: string;
  at: string;
  actorId: string;
  actorName: string;
  action: string;
  target: string;
  detail: Record<string, unknown> | null;
}

export interface TicketResponse {
  ticket: string;
  expiresAt: string;
  launchUrl: string;
}
