//! 構成情報 (manifest) の型。`schemas/emulator-manifest.schema.json` と対応する。

use crate::capability::{Approval, Capabilities};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub manifest_version: u32,
    pub id: String,
    pub revision: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub scheduler: SchedulerConfig,
    #[serde(default)]
    pub artifacts: BTreeMap<String, ArtifactRef>,
    pub components: Vec<ComponentSpec>,
    #[serde(default)]
    pub wires: Vec<Wire>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<serde_json::Value>,
    /// 承認 (capability 宣言のハッシュを含む)。V12 で照合する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval: Option<Approval>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Pacing {
    #[default]
    Realtime,
    Fast,
    Step,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerConfig {
    pub base_step_ns: i64,
    #[serde(default)]
    pub pacing: Pacing,
    #[serde(default = "one_f64")]
    pub speed: f64,
    #[serde(default = "default_max_slip")]
    pub max_slip_ms: u32,
    #[serde(default = "default_snapshot_divider")]
    pub snapshot_divider: u32,
    #[serde(default = "default_step_budget")]
    pub step_budget_ms: u32,
}

fn one_f64() -> f64 {
    1.0
}
fn default_max_slip() -> u32 {
    200
}
fn default_snapshot_divider() -> u32 {
    20
}
fn default_step_budget() -> u32 {
    50
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactKind {
    #[default]
    Part,
    Firmware,
    Ui,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    /// 内容ハッシュ。開発用のローカル manifest では省略できる (検証で警告)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// manifest ファイルからの相対パス (ローカル開発用)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default)]
    pub kind: ArtifactKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentSpec {
    pub instance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default = "one_u32")]
    pub rate_divider: u32,
    #[serde(default = "empty_object")]
    pub params: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firmware: Option<FirmwareSlot>,
    /// instance 単位の capability 宣言 (04-manifest.md)。builtin:cpu-virtual ではファームウェアに適用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,
}

fn one_u32() -> u32 {
    1
}
fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(Default::default())
}

impl ComponentSpec {
    pub fn is_builtin(&self) -> bool {
        self.kind.is_some()
    }
    /// 宣言がなければ空 (= サンドボックス既定)。
    pub fn capabilities(&self) -> Capabilities {
        self.capabilities.clone().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareSlot {
    pub slot: String,
    pub world: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default)]
    pub allow_override: Vec<String>,
    #[serde(default = "empty_object")]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wire {
    pub from: String,
    pub to: String,
    #[serde(default = "one_u8")]
    pub latency: u8,
    #[serde(default = "one_f64")]
    pub gain: f64,
    #[serde(default)]
    pub offset: f64,
}

fn one_u8() -> u8 {
    1
}

/// `instance.port` を分解する。
pub fn split_port_ref(s: &str) -> Option<(&str, &str)> {
    let (a, b) = s.split_once('.')?;
    if a.is_empty() || b.is_empty() {
        return None;
    }
    Some((a, b))
}

impl Manifest {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
    pub fn component(&self, instance: &str) -> Option<&ComponentSpec> {
        self.components.iter().find(|c| c.instance == instance)
    }
}
