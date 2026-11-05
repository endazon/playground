// このファイルは `cargo test -p ipemu-ipc` (scripts/gen-ipc-types.sh) が生成する。手で編集しない。
export type { BootstrapProgress } from "./BootstrapProgress";
export type { BootstrapStage } from "./BootstrapStage";
export type { CapabilityState } from "./CapabilityState";
export type { CatalogEntry } from "./CatalogEntry";
export type { CatalogLocalResult } from "./CatalogLocalResult";
export type { ConsentAnswerParams } from "./ConsentAnswerParams";
export type { ConsentRequest } from "./ConsentRequest";
export type { ConsentState } from "./ConsentState";
export type { Credential } from "./Credential";
export type { CredentialKind } from "./CredentialKind";
export type { DescribeResult } from "./DescribeResult";
export type { DiagnosticInfo } from "./DiagnosticInfo";
export type { Discovery } from "./Discovery";
export type { Empty } from "./Empty";
export type { EngineInfo } from "./EngineInfo";
export type { EventKind } from "./EventKind";
export type { EventNotice } from "./EventNotice";
export type { FirmwareInfo } from "./FirmwareInfo";
export type { FirmwareLoadParams } from "./FirmwareLoadParams";
export type { FirmwareLoadResult } from "./FirmwareLoadResult";
export type { FirmwareWatchParams } from "./FirmwareWatchParams";
export type { LaunchParams } from "./LaunchParams";
export type { LaunchSource } from "./LaunchSource";
export type { Level } from "./Level";
export type { ManifestRefreshResult } from "./ManifestRefreshResult";
export type { ManifestSummary } from "./ManifestSummary";
export type { Notification } from "./Notification";
export type { NotificationBody } from "./NotificationBody";
export type { Overrides } from "./Overrides";
export type { PacingConfig } from "./PacingConfig";
export type { PacingMode } from "./PacingMode";
export type { ParamOverride } from "./ParamOverride";
export type { ParamSetResult } from "./ParamSetResult";
export type { PartInfo } from "./PartInfo";
export type { PortInfo } from "./PortInfo";
export type { Request } from "./Request";
export type { RequestBody } from "./RequestBody";
export type { RequestId } from "./RequestId";
export type { Response } from "./Response";
export type { ResultBody } from "./ResultBody";
export type { RpcError } from "./RpcError";
export type { RunState } from "./RunState";
export type { RunStatus } from "./RunStatus";
export type { SchedulerStats } from "./SchedulerStats";
export type { ScopeFrame } from "./ScopeFrame";
export type { ScopeSubscribeParams } from "./ScopeSubscribeParams";
export type { ScopeSubscribeResult } from "./ScopeSubscribeResult";
export type { SessionDescription } from "./SessionDescription";
export type { StateFrame } from "./StateFrame";
export type { StepParams } from "./StepParams";
export type { UiBundleInfo } from "./UiBundleInfo";
export type { WireInfo } from "./WireInfo";

/** JSON-RPC エラーコード (crates/ipemu-ipc/src/error.rs と同じ値) */
export const codes = {
  PARSE_ERROR: -32700,
  INVALID_REQUEST: -32600,
  METHOD_NOT_FOUND: -32601,
  INVALID_PARAMS: -32602,
  INTERNAL_ERROR: -32603,
  ENGINE_ERROR: -32000,
  NO_SESSION: -32001,
  RESTART_REQUIRED: -32002,
  NOT_IMPLEMENTED: -32003,
  UNAUTHORIZED: -32004,
  NOT_FOUND: -32005,
  INVALID_STATE: -32006,
} as const;

export const IPC_VERSION = 1;
/** バイナリフレームの先頭バイト */
export const FRAME_STATE = 0x01;
export const FRAME_SCOPE = 0x02;
/** 要求メソッド名の一覧 (RequestBody::METHODS) */
export const METHODS = [
  "session.launch",
  "session.describe",
  "catalog.local",
  "run.start",
  "run.stop",
  "run.reset",
  "run.step",
  "run.setPacing",
  "param.set",
  "firmware.load",
  "firmware.watch",
  "scope.subscribe",
  "consent.answer",
  "manifest.refresh",
] as const;
