//! motor-driver — PWM デューティ → 端子電圧。ドライブ保護 (E-STOP / リミット / ハートビート断 / 無効中の指令拒否)。
//!
//! NetworkedSil の VirtualPlant が持っていた「ドライブ側の保護」を独立した部品にしたもの。
//! 保護は MCU (ファームウェア) に依存せず、この部品だけで電圧を切る。

use ip_part_sdk::{num, scalar, scalar_default, LogLevel, PartDesc, PartImpl};

const IN_DUTY: usize = 0;
const IN_ENABLE: usize = 1;
const IN_HEARTBEAT: usize = 2;
const IN_ESTOP: usize = 3;
const IN_POSITION: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    None = 0,
    EStop = 1,
    Limit = 2,
    HeartbeatTimeout = 3,
}

pub struct Driver {
    bus_voltage: f64,
    timeout_ns: i64,
    limit_m: f64,
    inputs: [f64; 5],
    enabled: bool,
    fault: Fault,
    prev_enable: bool,
    last_heartbeat: f64,
    since_heartbeat_ns: i64,
    heartbeat_seen: bool,
    voltage: f64,
    reject_count: u64,
}

impl Driver {
    fn trip(&mut self, f: Fault) {
        if self.fault == Fault::None {
            ip_part_sdk::log(LogLevel::Warn, &format!("ドライブ遮断: {f:?}"));
        }
        self.fault = f;
        self.enabled = false;
        self.voltage = 0.0;
    }
}

impl PartImpl for Driver {
    fn describe() -> PartDesc {
        PartDesc {
            name: "motor-driver".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            inputs: vec![
                scalar("duty", "1", "PWM デューティ -1..1"),
                scalar(
                    "enable",
                    "1",
                    "ドライブ有効要求 (>0.5)。立ち上がりで FAULT を解除して再有効化",
                ),
                scalar(
                    "heartbeat",
                    "1",
                    "MCU の生存信号。値が timeout 以内に変化しなければ遮断",
                ),
                scalar("estop-in", "1", "非常停止入力 (>0.5 で遮断)"),
                scalar_default(
                    "position",
                    "m",
                    0.0,
                    "台車位置 (|x| > limitM で遮断)。未配線なら 0",
                ),
            ],
            outputs: vec![
                scalar("voltage", "V", "端子電圧 (無効なら 0)"),
                scalar("fault", "1", "0=なし 1=E-STOP 2=リミット 3=ハートビート断"),
                scalar("drive-enabled", "1", "有効なら 1"),
                scalar("rejected", "count", "無効中に受けた非ゼロ指令の数"),
            ],
            params: Vec::new(),
            deterministic: true,
            doc:
                "PWM→電圧と保護。無効化後は enable の立ち上がり (現場側の復帰操作) まで有効化しない"
                    .into(),
        }
    }

    fn init(v: &serde_json::Value) -> Result<Self, String> {
        let bus_voltage = num(v, "busVoltage", 24.0);
        let timeout_ms = num(v, "commandTimeoutMs", 500.0);
        let limit_m = num(v, "limitM", f64::INFINITY);
        if !(bus_voltage > 0.0) || !(timeout_ms > 0.0) {
            return Err("busVoltage, commandTimeoutMs は正".into());
        }
        Ok(Self {
            bus_voltage,
            timeout_ns: (timeout_ms * 1e6) as i64,
            limit_m,
            inputs: [0.0; 5],
            enabled: false,
            fault: Fault::None,
            prev_enable: false,
            last_heartbeat: 0.0,
            since_heartbeat_ns: 0,
            heartbeat_seen: false,
            voltage: 0.0,
            reject_count: 0,
        })
    }

    fn reset(&mut self) {
        self.inputs = [0.0; 5];
        self.enabled = false;
        self.fault = Fault::None;
        self.prev_enable = false;
        self.last_heartbeat = 0.0;
        self.since_heartbeat_ns = 0;
        self.heartbeat_seen = false;
        self.voltage = 0.0;
        self.reject_count = 0;
    }

    fn set_inputs(&mut self, values: &[f64]) {
        for (i, v) in values.iter().take(5).enumerate() {
            self.inputs[i] = if v.is_finite() { *v } else { 0.0 };
        }
    }

    fn step(&mut self, dt_ns: i64) -> Result<(), String> {
        let enable = self.inputs[IN_ENABLE] > 0.5;
        let estop = self.inputs[IN_ESTOP] > 0.5;

        // E-STOP は常に最優先
        if estop {
            self.trip(Fault::EStop);
        }
        // メカリミット
        if self.inputs[IN_POSITION].abs() > self.limit_m {
            self.trip(Fault::Limit);
        }
        // ハートビート監視 (有効中のみ)
        if self.enabled {
            if self.inputs[IN_HEARTBEAT] != self.last_heartbeat || !self.heartbeat_seen {
                self.last_heartbeat = self.inputs[IN_HEARTBEAT];
                self.heartbeat_seen = true;
                self.since_heartbeat_ns = 0;
            } else {
                self.since_heartbeat_ns += dt_ns;
                if self.since_heartbeat_ns > self.timeout_ns {
                    self.trip(Fault::HeartbeatTimeout);
                }
            }
        }

        // enable の立ち上がりで復帰 (E-STOP が解除されていれば)
        if enable && !self.prev_enable && !estop {
            if self.fault != Fault::None {
                ip_part_sdk::log(LogLevel::Info, "enable の立ち上がりで FAULT を解除");
            }
            self.fault = Fault::None;
            self.enabled = true;
            self.heartbeat_seen = false;
            self.since_heartbeat_ns = 0;
        }
        if !enable && self.enabled {
            self.enabled = false;
            self.voltage = 0.0;
        }
        self.prev_enable = enable;

        if self.enabled && self.fault == Fault::None {
            let duty = self.inputs[IN_DUTY].clamp(-1.0, 1.0);
            self.voltage = duty * self.bus_voltage;
        } else {
            if self.inputs[IN_DUTY] != 0.0 {
                self.reject_count += 1;
            }
            self.voltage = 0.0;
        }
        Ok(())
    }

    fn get_outputs(&self, out: &mut Vec<f64>) {
        out.extend_from_slice(&[
            self.voltage,
            self.fault as u8 as f64,
            if self.enabled { 1.0 } else { 0.0 },
            self.reject_count as f64,
        ]);
    }
}

ip_part_sdk::part_entry!(Driver);
