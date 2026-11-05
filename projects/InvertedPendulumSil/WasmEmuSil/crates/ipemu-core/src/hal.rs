//! 仮想 MCU の HAL レジスタ像と、ファームウェアの抽象。
//!
//! ファームウェアの実体 (wasm / native) は `ipemu-wasm` が与える。ここでは
//! 「tick の前に入力を書き、tick の後に出力を読む」ためのレジスタ像だけを定義する。

use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn from_u32(v: u32) -> Self {
        match v {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            _ => LogLevel::Error,
        }
    }
}

/// 周辺の個数。manifest の `params.peripherals`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Peripherals {
    pub enc: usize,
    pub pwm: usize,
    pub dac: usize,
    pub gpio_in: usize,
    pub gpio_out: usize,
    pub uart: usize,
}

impl Default for Peripherals {
    fn default() -> Self {
        Self {
            enc: 2,
            pwm: 1,
            dac: 0,
            gpio_in: 2,
            gpio_out: 2,
            uart: 1,
        }
    }
}

/// ファームウェアから見える周辺の状態。1 tick ごとにホストが入力を書き、出力を読む。
#[derive(Debug, Default)]
pub struct HalRegs {
    /// 仮想時刻 [µs]
    pub micros: u64,
    pub enc: Vec<i32>,
    pub pwm: Vec<f32>,
    pub dac: Vec<f32>,
    pub gpio_in: Vec<bool>,
    pub gpio_out: Vec<bool>,
    pub uart_rx: Vec<VecDeque<u8>>,
    pub uart_tx: Vec<VecDeque<u8>>,
    /// tick 中に出たログ (ホストが回収して空にする)
    pub logs: Vec<(LogLevel, String)>,
    /// 数値トレース
    pub traces: Vec<(u32, f64)>,
}

impl HalRegs {
    pub const UART_QUEUE_CAPACITY: usize = 4096;

    pub fn new(p: &Peripherals) -> Self {
        Self {
            micros: 0,
            enc: vec![0; p.enc],
            pwm: vec![0.0; p.pwm],
            dac: vec![0.0; p.dac],
            gpio_in: vec![false; p.gpio_in],
            gpio_out: vec![false; p.gpio_out],
            uart_rx: (0..p.uart).map(|_| VecDeque::new()).collect(),
            uart_tx: (0..p.uart).map(|_| VecDeque::new()).collect(),
            logs: Vec::new(),
            traces: Vec::new(),
        }
    }

    // ---- HAL 関数の実体。wasm の import / native の vtable はここへ委譲する ----
    pub fn hal_micros(&self) -> u64 {
        self.micros
    }
    pub fn hal_enc_read(&self, ch: u32) -> i32 {
        self.enc.get(ch as usize).copied().unwrap_or(0)
    }
    pub fn hal_pwm_write(&mut self, ch: u32, duty: f32) {
        if let Some(v) = self.pwm.get_mut(ch as usize) {
            *v = if duty.is_nan() {
                0.0
            } else {
                duty.clamp(-1.0, 1.0)
            };
        }
    }
    pub fn hal_dac_write(&mut self, ch: u32, value: f32) {
        if let Some(v) = self.dac.get_mut(ch as usize) {
            *v = if value.is_nan() { 0.0 } else { value };
        }
    }
    pub fn hal_gpio_read(&self, pin: u32) -> bool {
        self.gpio_in.get(pin as usize).copied().unwrap_or(false)
    }
    pub fn hal_gpio_write(&mut self, pin: u32, level: bool) {
        if let Some(v) = self.gpio_out.get_mut(pin as usize) {
            *v = level;
        }
    }
    pub fn hal_uart_write(&mut self, ch: u32, data: &[u8]) -> u32 {
        let Some(q) = self.uart_tx.get_mut(ch as usize) else {
            return 0;
        };
        let room = Self::UART_QUEUE_CAPACITY.saturating_sub(q.len());
        let n = data.len().min(room);
        q.extend(&data[..n]);
        n as u32
    }
    pub fn hal_uart_read(&mut self, ch: u32, max: u32) -> Vec<u8> {
        let Some(q) = self.uart_rx.get_mut(ch as usize) else {
            return Vec::new();
        };
        let n = (max as usize).min(q.len());
        q.drain(..n).collect()
    }
    pub fn hal_log(&mut self, level: LogLevel, msg: &str) {
        if self.logs.len() < 1024 {
            self.logs.push((level, msg.to_owned()));
        }
    }
    pub fn hal_trace(&mut self, id: u32, value: f64) {
        if self.traces.len() < 4096 {
            self.traces.push((id, value));
        }
    }
}

/// 1 回の tick の実行統計。
#[derive(Debug, Clone, Copy, Default)]
pub struct TickStats {
    /// wasm の fuel 消費量 (fuel 計測が有効なときだけ Some)
    pub fuel: Option<u64>,
}

/// 1 呼び出しあたりの fuel 上限 (決定的なウォッチドッグ)。None は無制限 (壁時計の epoch だけが歯止め)。
///
/// 上限を超えた呼び出しは仮想時刻・壁時計によらず同じ命令位置で trap する (D57)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FuelLimits {
    /// `tick` と `irq` の 1 回あたり
    pub per_tick: Option<u64>,
    /// `abi-version` と `init` の 1 回あたり (cfg の解釈やゲイン計算で tick より重いことがある)
    pub per_init: Option<u64>,
}

impl FuelLimits {
    pub fn is_unlimited(&self) -> bool {
        self.per_tick.is_none() && self.per_init.is_none()
    }
}

/// 注入されるファームウェアの抽象。wasm 形も native 形も同じ呼び出しで扱う。
pub trait Firmware {
    fn abi_version(&mut self) -> anyhow::Result<u32>;
    fn init(&mut self, regs: &mut HalRegs, cfg_json: &str) -> anyhow::Result<()>;
    fn tick(&mut self, regs: &mut HalRegs) -> anyhow::Result<TickStats>;
    fn irq(&mut self, regs: &mut HalRegs, id: u32) -> anyhow::Result<()>;
    /// 形の表示名 ("wasm" / "native")
    fn flavor(&self) -> &'static str;
    /// ホット差替が可能か (wasm は true、native は false)
    fn hot_reloadable(&self) -> bool {
        false
    }
    /// fuel を計測する形か (wasm で計測が有効なとき true)。false なら `TickStats::fuel` は常に None
    fn meters_fuel(&self) -> bool {
        false
    }
    /// 呼び出しごとの fuel 上限を設定する。`abi_version` / `init` より前に呼ぶ。fuel を計測しない形では何もしない
    fn set_fuel_limits(&mut self, _limits: FuelLimits) {}
}
