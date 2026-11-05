//! `builtin:cpu-virtual` — 仮想 MCU。周辺をポートとして持ち、注入されたファームウェアを `tickHz` で呼ぶ。

use crate::desc::{PartDesc, PortDesc};
use crate::engine::Part;
use crate::hal::{Firmware, HalRegs, LogLevel, Peripherals};
use serde::Deserialize;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TimingModel {
    /// 理想 (実行時間 0)
    #[default]
    Zero,
    /// tick ごとに固定の実行時間
    Fixed {
        #[serde(rename = "fixedNs")]
        fixed_ns: i64,
    },
    /// wasm の fuel 消費量 × nsPerFuel
    Fuel {
        #[serde(rename = "nsPerFuel")]
        ns_per_fuel: f64,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PeripheralsJson {
    #[serde(default = "d2")]
    enc: usize,
    #[serde(default = "d1")]
    pwm: usize,
    #[serde(default)]
    dac: usize,
    #[serde(default = "d2")]
    gpio_in: usize,
    #[serde(default = "d2")]
    gpio_out: usize,
    #[serde(default = "d1")]
    uart: usize,
}
fn d1() -> usize {
    1
}
fn d2() -> usize {
    2
}
impl Default for PeripheralsJson {
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuVirtualParams {
    #[serde(default = "default_tick_hz")]
    pub tick_hz: f64,
    #[serde(default)]
    pub timing_model: TimingModel,
    #[serde(default)]
    peripherals: PeripheralsJson,
}
fn default_tick_hz() -> f64 {
    1000.0
}

impl CpuVirtualParams {
    pub fn from_json(v: &serde_json::Value) -> anyhow::Result<Self> {
        let p: CpuVirtualParams = serde_json::from_value(v.clone())?;
        anyhow::ensure!(
            p.tick_hz > 0.0 && p.tick_hz.is_finite(),
            "tickHz は正の有限値"
        );
        anyhow::ensure!(
            p.peripherals.enc <= 16
                && p.peripherals.pwm <= 16
                && p.peripherals.gpio_in <= 32
                && p.peripherals.gpio_out <= 32,
            "周辺の個数が多すぎます"
        );
        Ok(p)
    }
    pub fn peripherals(&self) -> Peripherals {
        Peripherals {
            enc: self.peripherals.enc,
            pwm: self.peripherals.pwm,
            dac: self.peripherals.dac,
            gpio_in: self.peripherals.gpio_in,
            gpio_out: self.peripherals.gpio_out,
            uart: self.peripherals.uart,
        }
    }
    pub fn tick_period_ns(&self) -> i64 {
        (1e9 / self.tick_hz).round() as i64
    }
}

/// 実行時間モデルで遅らせる出力 (適用時刻付き)。
struct PendingOutputs {
    apply_at_ns: i64,
    pwm: Vec<f32>,
    dac: Vec<f32>,
    gpio_out: Vec<bool>,
}

pub struct CpuVirtual {
    desc: PartDesc,
    params: CpuVirtualParams,
    per: Peripherals,
    firmware: Option<Box<dyn Firmware>>,
    fw_cfg_json: String,
    regs: HalRegs,
    t_ns: i64,
    next_tick_ns: i64,
    tick_count: u64,
    overruns: u64,
    last_exec_ns: i64,
    pending: VecDeque<PendingOutputs>,
    // 入出力ポートの像
    inputs: Vec<f64>,
    out_pwm: Vec<f32>,
    out_dac: Vec<f32>,
    out_gpio: Vec<bool>,
    logs: Vec<(LogLevel, String)>,
    fault: bool,
}

impl CpuVirtual {
    pub fn describe_for(p: &CpuVirtualParams) -> PartDesc {
        let per = p.peripherals();
        let mut inputs = Vec::new();
        for i in 0..per.enc {
            inputs.push(PortDesc::scalar(
                &format!("enc{i}"),
                "count",
                "エンコーダ累積カウント (整数)",
            ));
        }
        for i in 0..per.gpio_in {
            inputs.push(PortDesc::scalar(
                &format!("gpio-in{i}"),
                "1",
                "デジタル入力 (>0.5 で H)",
            ));
        }
        let mut outputs = Vec::new();
        for i in 0..per.pwm {
            outputs.push(PortDesc::scalar(
                &format!("pwm{i}"),
                "1",
                "PWM デューティ -1..1",
            ));
        }
        for i in 0..per.dac {
            outputs.push(PortDesc::scalar(&format!("dac{i}"), "1", "DAC 出力"));
        }
        for i in 0..per.gpio_out {
            outputs.push(PortDesc::scalar(
                &format!("gpio-out{i}"),
                "1",
                "デジタル出力 (0/1)",
            ));
        }
        outputs.push(PortDesc::scalar(
            "exec-ns",
            "ns",
            "直近 tick の仮想実行時間",
        ));
        outputs.push(PortDesc::scalar(
            "overruns",
            "count",
            "実行時間が tick 周期を超えた回数",
        ));
        outputs.push(PortDesc::scalar("tick-count", "count", "実行した tick 数"));
        PartDesc {
            name: "builtin:cpu-virtual".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            inputs,
            outputs,
            params: Vec::new(),
            deterministic: true,
            doc: "仮想 MCU。HAL 付きの関数呼び出しモデルで注入ファームウェアを周期的に呼ぶ".into(),
        }
    }

    pub fn new(params: CpuVirtualParams, fw_cfg_json: String) -> Self {
        let per = params.peripherals();
        let desc = Self::describe_for(&params);
        let n_in = desc.inputs.len();
        Self {
            desc,
            per,
            regs: HalRegs::new(&per),
            firmware: None,
            fw_cfg_json,
            t_ns: 0,
            next_tick_ns: 0,
            tick_count: 0,
            overruns: 0,
            last_exec_ns: 0,
            pending: VecDeque::new(),
            inputs: vec![0.0; n_in],
            out_pwm: vec![0.0; per.pwm],
            out_dac: vec![0.0; per.dac],
            out_gpio: vec![false; per.gpio_out],
            logs: Vec::new(),
            fault: false,
            params,
        }
    }

    /// ファームウェアを装着して init する。実行中の差し替え (ホット) にも使う。
    pub fn attach_firmware(&mut self, mut fw: Box<dyn Firmware>) -> anyhow::Result<()> {
        let abi = fw.abi_version()?;
        anyhow::ensure!(abi == 1, "ファームウェア ABI {abi} は未対応 (対応: 1)");
        self.regs.micros = (self.t_ns / 1000) as u64;
        fw.init(&mut self.regs, &self.fw_cfg_json)?;
        self.collect_logs();
        self.firmware = Some(fw);
        self.fault = false;
        Ok(())
    }

    pub fn firmware_flavor(&self) -> Option<&'static str> {
        self.firmware.as_ref().map(|f| f.flavor())
    }
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }
    pub fn overruns(&self) -> u64 {
        self.overruns
    }

    fn collect_logs(&mut self) {
        self.logs.append(&mut self.regs.logs);
        self.regs.traces.clear();
    }

    fn load_inputs_to_regs(&mut self) {
        let mut k = 0;
        for i in 0..self.per.enc {
            // f64 で運ばれる整数カウント。32bit でラップする
            let v = self.inputs[k].round();
            self.regs.enc[i] = (v as i64) as i32;
            k += 1;
        }
        for i in 0..self.per.gpio_in {
            self.regs.gpio_in[i] = self.inputs[k] > 0.5;
            k += 1;
        }
    }

    fn exec_time_ns(&self, stats: &crate::hal::TickStats) -> i64 {
        match self.params.timing_model {
            TimingModel::Zero => 0,
            TimingModel::Fixed { fixed_ns } => fixed_ns.max(0),
            TimingModel::Fuel { ns_per_fuel } => match stats.fuel {
                Some(f) => (f as f64 * ns_per_fuel).round() as i64,
                None => 0,
            },
        }
    }

    fn apply_pending(&mut self) {
        while let Some(p) = self.pending.front() {
            if p.apply_at_ns > self.t_ns {
                break;
            }
            let p = self.pending.pop_front().unwrap();
            self.out_pwm = p.pwm;
            self.out_dac = p.dac;
            self.out_gpio = p.gpio_out;
        }
    }
}

impl Part for CpuVirtual {
    fn describe(&self) -> &PartDesc {
        &self.desc
    }

    fn init(&mut self, _params: &serde_json::Value) -> anyhow::Result<()> {
        // パラメータはコンストラクタで解釈済み。ファームウェアは attach_firmware で装着する
        Ok(())
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        self.t_ns = 0;
        self.next_tick_ns = 0;
        self.tick_count = 0;
        self.overruns = 0;
        self.last_exec_ns = 0;
        self.pending.clear();
        self.regs = HalRegs::new(&self.per);
        self.out_pwm.iter_mut().for_each(|v| *v = 0.0);
        self.out_dac.iter_mut().for_each(|v| *v = 0.0);
        self.out_gpio.iter_mut().for_each(|v| *v = false);
        self.fault = false;
        if let Some(fw) = self.firmware.as_mut() {
            fw.init(&mut self.regs, &self.fw_cfg_json)?;
        }
        self.collect_logs();
        Ok(())
    }

    fn set_inputs(&mut self, values: &[f64]) -> anyhow::Result<()> {
        anyhow::ensure!(values.len() == self.inputs.len(), "入力数が一致しません");
        self.inputs.copy_from_slice(values);
        Ok(())
    }

    fn step(&mut self, dt_ns: i64) -> anyhow::Result<()> {
        let period = self.params.tick_period_ns();
        let end = self.t_ns + dt_ns;
        // borrow を単純にするためファームウェアを一旦取り出す
        let mut fw = self.firmware.take();
        let mut result = Ok(());
        // このステップ内に来る tick をすべて処理する (tick 周期 < ステップのときは複数回)
        while self.next_tick_ns < end {
            self.t_ns = self.next_tick_ns;
            self.apply_pending();
            if let Some(f) = fw.as_mut() {
                if !self.fault {
                    self.load_inputs_to_regs();
                    self.regs.micros = (self.t_ns / 1000) as u64;
                    match f.tick(&mut self.regs) {
                        Ok(stats) => {
                            let exec = self.exec_time_ns(&stats);
                            self.last_exec_ns = exec;
                            if exec > period {
                                self.overruns += 1;
                            }
                            self.pending.push_back(PendingOutputs {
                                apply_at_ns: self.t_ns + exec,
                                pwm: self.regs.pwm.clone(),
                                dac: self.regs.dac.clone(),
                                gpio_out: self.regs.gpio_out.clone(),
                            });
                            self.collect_logs();
                        }
                        Err(e) => {
                            self.fault = true;
                            self.collect_logs();
                            self.logs
                                .push((LogLevel::Error, format!("firmware.tick が失敗: {e:#}")));
                            result = Err(e.context("ファームウェアの tick が trap"));
                            break;
                        }
                    }
                }
            }
            self.tick_count += 1;
            self.next_tick_ns += period;
        }
        self.firmware = fw;
        result?;
        self.t_ns = end;
        self.apply_pending();
        Ok(())
    }

    fn get_outputs(&mut self, out: &mut [f64]) -> anyhow::Result<()> {
        let mut k = 0;
        for v in &self.out_pwm {
            out[k] = *v as f64;
            k += 1;
        }
        for v in &self.out_dac {
            out[k] = *v as f64;
            k += 1;
        }
        for v in &self.out_gpio {
            out[k] = if *v { 1.0 } else { 0.0 };
            k += 1;
        }
        out[k] = self.last_exec_ns as f64;
        out[k + 1] = self.overruns as f64;
        out[k + 2] = self.tick_count as f64;
        Ok(())
    }

    fn drain_logs(&mut self) -> Vec<(LogLevel, String)> {
        std::mem::take(&mut self.logs)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}
