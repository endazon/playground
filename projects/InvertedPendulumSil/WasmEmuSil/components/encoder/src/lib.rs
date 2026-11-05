//! encoder — 連続量 (角度 / 位置) を量子化して累積カウントにする。

use ip_part_sdk::{num, scalar, PartDesc, PartImpl};

pub struct Encoder {
    counts_per_unit: f64,
    angular: bool,
    input: f64,
    count: i64,
    prev_count: i64,
    offset_count: i64,
}

impl PartImpl for Encoder {
    fn describe() -> PartDesc {
        PartDesc {
            name: "encoder".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            inputs: vec![scalar("input", "", "角度 [rad] (mode=angular) または位置 [m] (mode=linear)。単位は mode で決まるので検証対象外")],
            outputs: vec![
                scalar("count", "count", "累積カウント (32bit でラップ)"),
                scalar("delta", "count", "直近ステップのカウント変化"),
            ],
            params: Vec::new(),
            deterministic: true,
            doc: "ppr と mode で量子化。NetworkedSil の ReadEncoders と同じ丸め".into(),
        }
    }

    fn init(v: &serde_json::Value) -> Result<Self, String> {
        let ppr = num(v, "ppr", 4096.0);
        if !(ppr >= 1.0) {
            return Err("ppr は 1 以上".into());
        }
        let mode = v.get("mode").and_then(|m| m.as_str()).unwrap_or("angular");
        let counts_per_unit = match mode {
            "angular" => ppr / (2.0 * core::f64::consts::PI),
            "linear" => {
                let m_per_rev = num(v, "metersPerRev", 0.0);
                if !(m_per_rev > 0.0) {
                    return Err("mode=linear には metersPerRev が必要".into());
                }
                ppr / m_per_rev
            }
            other => return Err(format!("未知の mode `{other}`")),
        };
        let offset_count = num(v, "offsetCount", 0.0) as i64;
        Ok(Self {
            counts_per_unit,
            angular: mode == "angular",
            input: 0.0,
            count: offset_count,
            prev_count: offset_count,
            offset_count,
        })
    }

    fn reset(&mut self) {
        self.input = 0.0;
        self.count = self.offset_count;
        self.prev_count = self.offset_count;
    }

    fn set_inputs(&mut self, values: &[f64]) {
        if let Some(v) = values.first() {
            self.input = if v.is_finite() { *v } else { self.input };
        }
    }

    fn step(&mut self, _dt_ns: i64) -> Result<(), String> {
        self.prev_count = self.count;
        let _ = self.angular;
        let raw =
            ip_model::math::round(self.input * self.counts_per_unit) as i64 + self.offset_count;
        // 32bit ラップ (実機のカウンタ幅)
        self.count = (raw as i32) as i64;
        Ok(())
    }

    fn get_outputs(&self, out: &mut Vec<f64>) {
        let delta = (self.count as i32).wrapping_sub(self.prev_count as i32) as f64;
        out.extend_from_slice(&[self.count as f64, delta]);
    }
}

ip_part_sdk::part_entry!(Encoder);
