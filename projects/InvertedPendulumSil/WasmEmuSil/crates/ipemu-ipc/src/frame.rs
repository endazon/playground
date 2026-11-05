//! バイナリフレーム。先頭 1 バイトがチャネル種別、残りが MessagePack (rmp-serde、名前付きフィールド)。
//!
//! TS 側は `@msgpack/msgpack` の `decode(bytes.subarray(1))` で同じ構造が得られる。
//! 64bit 整数は JS の number に落ちるが、仮想時刻 [ns] なら 2^53 ns ≈ 104 日で足りる。

use crate::methods::{PacingMode, RunState};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    /// 既定 50Hz のスナップショット
    State = 0x01,
    /// `scope.subscribe` の高レート値
    Scope = 0x02,
}

impl FrameKind {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x01 => Some(FrameKind::State),
            0x02 => Some(FrameKind::Scope),
            _ => None,
        }
    }
}

/// スケジューラの実時間統計 (state フレームに同梱)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SchedulerStats {
    /// 直近 1 秒の実時間倍率 (仮想時間 / 壁時計)。fast では大きな値
    pub realtime_ratio: f64,
    /// 1 ステップあたりの壁時計 [µs] (直近の移動平均)
    pub step_wall_us: f64,
    /// realtime で maxSlip を超えて追いつきを諦めた累積 [ms]
    pub slip_ms_total: f64,
    /// スリップの回数
    pub slips: u32,
    /// 1 ステップが stepBudgetMs を超えた回数
    pub step_overruns: u32,
}

/// `state` フレーム。`values` は `session.describe` の `snapshotPorts` と同じ順。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StateFrame {
    #[ts(type = "number")]
    pub t_ns: i64,
    #[ts(type = "number")]
    pub tick: u64,
    pub epoch: u32,
    pub state: RunState,
    pub pacing: PacingMode,
    pub speed: f64,
    pub values: Vec<f64>,
    pub stats: SchedulerStats,
}

/// `scope` フレーム。`channels[i][k]` は `ports[i]` の `t0Ns + k * periodNs` 時点の値。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScopeFrame {
    pub subscription_id: u32,
    pub epoch: u32,
    #[ts(type = "number")]
    pub t0_ns: i64,
    #[ts(type = "number")]
    pub period_ns: i64,
    pub channels: Vec<Vec<f64>>,
}

/// 種別バイト + MessagePack にエンコードする。
pub fn encode_frame<T: Serialize>(
    kind: FrameKind,
    payload: &T,
) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    let mut out = vec![kind as u8];
    let mut ser = rmp_serde::Serializer::new(&mut out).with_struct_map();
    payload.serialize(&mut ser)?;
    Ok(out)
}

/// 先頭バイトを見て payload を復号する。種別が合わなければ None。
pub fn decode_frame<T: DeserializeOwned>(kind: FrameKind, bytes: &[u8]) -> Option<T> {
    let (&k, rest) = bytes.split_first()?;
    if FrameKind::from_byte(k)? != kind {
        return None;
    }
    rmp_serde::from_slice(rest).ok()
}
