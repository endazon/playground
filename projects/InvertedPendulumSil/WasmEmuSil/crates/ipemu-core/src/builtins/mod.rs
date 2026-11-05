//! 組み込み部品。エンジンの内部構造 (ファームウェアローダ・スケジューラ) に触る必要があるものだけ。

pub mod cpu_virtual;

pub use cpu_virtual::{CpuVirtual, CpuVirtualParams, TimingModel};

/// 組み込み部品の名前一覧 (manifest 検証で使う)。
pub const BUILTIN_KINDS: &[&str] = &["builtin:cpu-virtual", "builtin:snapshot"];

/// 組み込み部品の記述を返す (ポートはパラメータで決まる)。
pub fn describe_builtin(kind: &str, params: &serde_json::Value) -> anyhow::Result<crate::PartDesc> {
    match kind {
        "builtin:cpu-virtual" => Ok(CpuVirtual::describe_for(&CpuVirtualParams::from_json(
            params,
        )?)),
        "builtin:snapshot" => Ok(crate::PartDesc {
            name: "builtin:snapshot".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            doc: "指定ポートをスナップショット / 記録に載せる (ポートは持たない)".into(),
            ..Default::default()
        }),
        other => anyhow::bail!("未知の組み込み部品 `{other}`"),
    }
}
