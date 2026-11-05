//! 要求 (シェル / CLI → エンジン) のメソッドと、その params / result。
//!
//! [`RequestBody`] は `{"method": "...", "params": {...}}` の形で (de)serialize される。
//! 対応する result は [`ResultBody`] に同じタグで並べ、TS 側はこの 2 つから
//! `method → (params, result)` の対応表を型レベルで引く (`apps/ui-kit/src/ipc/client.ts`)。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

// ---------------------------------------------------------------------------
// 共通の小さな型
// ---------------------------------------------------------------------------

/// params を持たないメソッド用の空オブジェクト。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Empty {}

/// ペーシングの種別 (manifest の `scheduler.pacing` と同じ語)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum PacingMode {
    /// 壁時計に追従する。`speed` 倍速
    #[default]
    Realtime,
    /// 待たずに回す
    Fast,
    /// `run.step` でだけ進む
    Step,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PacingConfig {
    pub mode: PacingMode,
    /// realtime のときの倍率 (1.0 = 等速)。他のモードでは無視
    pub speed: f64,
}

impl Default for PacingConfig {
    fn default() -> Self {
        Self {
            mode: PacingMode::Realtime,
            speed: 1.0,
        }
    }
}

/// 実行状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum RunState {
    /// 組み立て済みで停止中 (開始前、または stop 後)
    #[default]
    Idle,
    Running,
    /// 部品 / FW の step が失敗した。`run.reset` で復帰する
    Fault,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RunStatus {
    pub state: RunState,
    pub pacing: PacingConfig,
    /// 仮想時刻 [ns]
    #[ts(type = "number")]
    pub t_ns: i64,
    #[ts(type = "number")]
    pub tick: u64,
    /// `run.reset` のたびに 1 増える。UI はこれが変わったらスコープを捨てる
    pub epoch: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fault: Option<String>,
}

/// パラメータ上書き `instance.key = value`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ParamOverride {
    pub instance: String,
    pub key: String,
    pub value: serde_json::Value,
}

// ---------------------------------------------------------------------------
// session.launch
// ---------------------------------------------------------------------------

/// 資格情報。M3 でサーバ配信が入るまでエンジンは `Server` を受け付けない (NOT_IMPLEMENTED)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum Credential {
    /// ポータルにログインした利用者のセッショントークン (シェルのブリッジ経由)
    SessionToken { token: String },
    /// `ipemu://` リンクに載る単回チケット。エンジンが短命トークンに交換する
    Ticket { ticket: String },
    /// headless / CI 用
    ApiKey { key: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CredentialKind {
    User,
    ApiKey,
    None,
}

/// 構成の取得元。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum LaunchSource {
    /// ローカルの manifest ファイル (開発用。`path` 参照・署名なし)
    #[serde(rename_all = "camelCase")]
    Local {
        manifest_path: String,
        /// sha256 / 署名のない manifest を許す (`--no-signature-check`)
        #[serde(default)]
        allow_unhashed: bool,
    },
    /// サーバのカタログ (M3 の bootstrap: 取得 → 署名検証 → キャッシュ)
    #[serde(rename_all = "camelCase")]
    Server {
        server: String,
        manifest_id: String,
        /// None なら既定 revision
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        revision: Option<u32>,
        /// None はキャッシュからのオフライン起動 (`offline: true`) のときだけ許される
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        credential: Option<Credential>,
        /// サーバに問い合わせず、キャッシュ (ローカルカタログ) から起動する
        #[serde(default)]
        offline: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LaunchParams {
    pub source: LaunchSource,
    /// slot → `wasm:<path>` | `native:<path>` | `artifact:<name@ver>` | `default`
    #[serde(default)]
    pub firmware: BTreeMap<String, String>,
    #[serde(default)]
    pub params: Vec<ParamOverride>,
    /// 省略時は manifest の scheduler.pacing / speed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub pacing: Option<PacingConfig>,
    /// 組み立て後すぐ run.start 相当を行う
    #[serde(default)]
    pub auto_start: bool,
}

// ---------------------------------------------------------------------------
// session.describe
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PortInfo {
    pub name: String,
    /// `scalar` | `bytes`
    pub kind: String,
    pub unit: String,
    pub default: f64,
    pub doc: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FirmwareInfo {
    pub slot: String,
    pub world: String,
    /// 装着中の形。`wasm` | `native`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub flavor: Option<String>,
    /// 装着中の指定 (`default` / `wasm:<path>` / `native:<path>`)
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default: Option<String>,
    pub allow_override: Vec<String>,
    /// `firmware.watch` で監視中のパス
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub watching: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PartInfo {
    pub instance: String,
    /// `part` (wasm 部品) | `builtin:cpu-virtual` | `builtin:snapshot`
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub artifact: Option<String>,
    /// describe() の name / version
    pub name: String,
    pub version: String,
    pub inputs: Vec<PortInfo>,
    pub outputs: Vec<PortInfo>,
    pub params: serde_json::Value,
    pub rate_divider: u32,
    pub deterministic: bool,
    pub doc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub firmware: Option<FirmwareInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WireInfo {
    pub from: String,
    pub to: String,
    pub latency: u8,
    pub gain: f64,
    pub offset: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ConsentState {
    Granted,
    Denied,
    Pending,
    /// 宣言なし、または同意が不要な種類
    NotRequired,
}

/// 部品が宣言した capability と同意の状態 (M5 で実装。M2 は宣言の列挙だけ)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CapabilityState {
    pub instance: String,
    /// 例 `net.tcpConnect:192.168.10.5:502`
    pub capability: String,
    pub consent: ConsentState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub artifact_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ManifestSummary {
    pub id: String,
    pub revision: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    pub source: LaunchSource,
    #[ts(type = "number")]
    pub base_step_ns: i64,
    pub snapshot_divider: u32,
}

/// UI バンドル (manifest の `ui.bundle`)。アーティファクトは単一ファイル (tar.gz)。
/// シェルがキャッシュへ展開して `app://ui/` で配信する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UiBundleInfo {
    pub artifact: String,
    /// tar.gz の絶対パス (エンジンのキャッシュ、またはローカル manifest の path)
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub views: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EngineInfo {
    pub version: String,
    pub ipc_version: u32,
    pub pid: u32,
    /// `--debug-friendly` (または デバッガ検出) で追いつきを止めている
    pub debug_friendly: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Overrides {
    pub firmware: BTreeMap<String, String>,
    pub params: Vec<ParamOverride>,
}

/// `session.describe` / `session.launch` の結果。セッションがないときは `None`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SessionDescription {
    pub manifest: ManifestSummary,
    pub parts: Vec<PartInfo>,
    pub wires: Vec<WireInfo>,
    /// `state` フレームの `values` と同じ順
    pub snapshot_ports: Vec<String>,
    pub run: RunStatus,
    pub capabilities: Vec<CapabilityState>,
    pub overrides: Overrides,
    pub credential: CredentialKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ui: Option<UiBundleInfo>,
    /// キャッシュ構成でのオフライン起動
    pub offline: bool,
    pub warnings: Vec<DiagnosticInfo>,
    pub engine: EngineInfo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DescribeResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub session: Option<SessionDescription>,
    pub engine: EngineInfo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DiagnosticInfo {
    /// `error` | `warning`
    pub severity: String,
    pub rule: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// catalog.local
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CatalogEntry {
    pub id: String,
    pub revision: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub description: Option<String>,
    /// 取得元サーバ (ローカル manifest なら None)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub server: Option<String>,
    pub source: LaunchSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub last_launched: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CatalogLocalResult {
    pub entries: Vec<CatalogEntry>,
}

// ---------------------------------------------------------------------------
// run.*
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StepParams {
    /// 進める baseStep 数 (1 以上)
    #[ts(type = "number")]
    pub n: u64,
}

// ---------------------------------------------------------------------------
// param.set
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ParamSetResult {
    /// 部品を `init` し直した (内部状態は初期化される)
    pub reinitialized: bool,
    /// 組み込み部品など、再起動 (再 launch) しないと反映できない
    pub restart_required: bool,
    pub params: serde_json::Value,
}

// ---------------------------------------------------------------------------
// firmware.load / firmware.watch
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FirmwareLoadParams {
    pub slot: String,
    /// `wasm:<path>` | `native:<path>` | `artifact:<name@ver>` | `default`
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FirmwareLoadResult {
    /// true: native 形。エンジンを再起動して同じ launch (上書き込み) をやり直す必要がある
    pub restart_required: bool,
    /// true: wasm をホットで差し替えた (プラント状態は保持)
    pub hot: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub flavor: Option<String>,
    /// 再起動後に渡すべき上書き一式
    pub overrides: Overrides,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FirmwareWatchParams {
    pub slot: String,
    /// 監視する `wasm:<path>` / `native:<path>`。None で監視を止める
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source: Option<String>,
}

// ---------------------------------------------------------------------------
// scope.subscribe
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScopeSubscribeParams {
    /// `instance.port` (出力ポート)。空なら購読解除
    pub ports: Vec<String>,
    /// 希望サンプルレート。baseStep の整数倍に丸める
    pub rate_hz: f64,
    /// フレームをまとめる時間幅 [s] (例 0.05)。この間隔で `scope` フレームが届く
    pub window_s: f64,
    /// 既存の購読を置き換えるときに指定
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub subscription_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScopeSubscribeResult {
    pub subscription_id: u32,
    pub ports: Vec<String>,
    /// 実際のサンプル周期 [ns]
    #[ts(type = "number")]
    pub period_ns: i64,
}

// ---------------------------------------------------------------------------
// consent.answer / manifest.refresh
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConsentAnswerParams {
    pub request_id: String,
    pub grant: bool,
    /// 記憶する (consent store へ書く)。false は「今回だけ」
    pub remember: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ManifestRefreshResult {
    /// 取得し直した manifest が現在のものと異なる
    pub changed: bool,
    pub revision: u32,
    pub diagnostics: Vec<DiagnosticInfo>,
}

// ---------------------------------------------------------------------------
// 要求本文 / 結果本文
// ---------------------------------------------------------------------------

/// 要求の本文。`method` でタグ付けし、`params` に本体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "method", content = "params")]
#[ts(export)]
pub enum RequestBody {
    #[serde(rename = "session.launch")]
    SessionLaunch(LaunchParams),
    #[serde(rename = "session.describe")]
    SessionDescribe(Empty),
    #[serde(rename = "catalog.local")]
    CatalogLocal(Empty),
    #[serde(rename = "run.start")]
    RunStart(Empty),
    #[serde(rename = "run.stop")]
    RunStop(Empty),
    #[serde(rename = "run.reset")]
    RunReset(Empty),
    #[serde(rename = "run.step")]
    RunStep(StepParams),
    #[serde(rename = "run.setPacing")]
    RunSetPacing(PacingConfig),
    #[serde(rename = "param.set")]
    ParamSet(ParamOverride),
    #[serde(rename = "firmware.load")]
    FirmwareLoad(FirmwareLoadParams),
    #[serde(rename = "firmware.watch")]
    FirmwareWatch(FirmwareWatchParams),
    #[serde(rename = "scope.subscribe")]
    ScopeSubscribe(ScopeSubscribeParams),
    #[serde(rename = "consent.answer")]
    ConsentAnswer(ConsentAnswerParams),
    #[serde(rename = "manifest.refresh")]
    ManifestRefresh(Empty),
}

impl RequestBody {
    /// メソッド名の一覧 (未知メソッドの判定と、TS 側の網羅チェックに使う)。
    pub const METHODS: &'static [&'static str] = &[
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
    ];

    pub fn method(&self) -> &'static str {
        match self {
            RequestBody::SessionLaunch(_) => "session.launch",
            RequestBody::SessionDescribe(_) => "session.describe",
            RequestBody::CatalogLocal(_) => "catalog.local",
            RequestBody::RunStart(_) => "run.start",
            RequestBody::RunStop(_) => "run.stop",
            RequestBody::RunReset(_) => "run.reset",
            RequestBody::RunStep(_) => "run.step",
            RequestBody::RunSetPacing(_) => "run.setPacing",
            RequestBody::ParamSet(_) => "param.set",
            RequestBody::FirmwareLoad(_) => "firmware.load",
            RequestBody::FirmwareWatch(_) => "firmware.watch",
            RequestBody::ScopeSubscribe(_) => "scope.subscribe",
            RequestBody::ConsentAnswer(_) => "consent.answer",
            RequestBody::ManifestRefresh(_) => "manifest.refresh",
        }
    }
}

/// 各メソッドの result 型。ワイヤには `Response.result` として本体だけが載る。
/// この enum は TS に `method → result` の対応を伝えるためだけにある。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "method", content = "result")]
#[ts(export)]
pub enum ResultBody {
    #[serde(rename = "session.launch")]
    SessionLaunch(SessionDescription),
    #[serde(rename = "session.describe")]
    SessionDescribe(DescribeResult),
    #[serde(rename = "catalog.local")]
    CatalogLocal(CatalogLocalResult),
    #[serde(rename = "run.start")]
    RunStart(RunStatus),
    #[serde(rename = "run.stop")]
    RunStop(RunStatus),
    #[serde(rename = "run.reset")]
    RunReset(RunStatus),
    #[serde(rename = "run.step")]
    RunStep(RunStatus),
    #[serde(rename = "run.setPacing")]
    RunSetPacing(RunStatus),
    #[serde(rename = "param.set")]
    ParamSet(ParamSetResult),
    #[serde(rename = "firmware.load")]
    FirmwareLoad(FirmwareLoadResult),
    #[serde(rename = "firmware.watch")]
    FirmwareWatch(Empty),
    #[serde(rename = "scope.subscribe")]
    ScopeSubscribe(ScopeSubscribeResult),
    #[serde(rename = "consent.answer")]
    ConsentAnswer(Empty),
    #[serde(rename = "manifest.refresh")]
    ManifestRefresh(ManifestRefreshResult),
}
