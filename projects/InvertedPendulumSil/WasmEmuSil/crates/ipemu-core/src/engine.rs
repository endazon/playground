//! スケジューラ。仮想時間を固定ステップで進め、部品を実行順に step し、配線に従って転記する。
//!
//! 単一スレッドで使う。壁時計へのペーシングは呼び出し側 (CLI / エンジン本体) の責務。

use crate::desc::PartDesc;
use crate::graph::Graph;
use thiserror::Error;

/// 部品の抽象。wasm 部品も組み込み部品も同じ契約で呼ぶ (WIT の `part` と同型)。
pub trait Part {
    fn describe(&self) -> &PartDesc;
    fn init(&mut self, params: &serde_json::Value) -> anyhow::Result<()>;
    fn reset(&mut self) -> anyhow::Result<()>;
    /// describe().inputs の順で全スカラー入力を一括設定する
    fn set_inputs(&mut self, values: &[f64]) -> anyhow::Result<()>;
    fn step(&mut self, dt_ns: i64) -> anyhow::Result<()>;
    /// describe().outputs の順で全スカラー出力を `out` に書く (長さは呼び出し側が確保)
    fn get_outputs(&mut self, out: &mut [f64]) -> anyhow::Result<()>;
    /// 部品が出したログを回収する (任意)
    fn drain_logs(&mut self) -> Vec<(crate::hal::LogLevel, String)> {
        Vec::new()
    }
    /// 具象型へ降りるための入口 (任意)。エンジンが組み込み部品を直接操作するときに使う
    /// (例: `builtin:cpu-virtual` のファームウェアをホット差替する)。wasm 部品は None のままでよい。
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        None
    }
}

#[derive(Debug, Error)]
pub enum StepError {
    // `{source:#}`: 原因の連鎖 (例: fuel 上限の値) まで FAULT の理由に載せる
    #[error("部品 `{instance}` の step が失敗しました (仮想時刻 {t_ns} ns): {source:#}")]
    Part {
        instance: String,
        t_ns: i64,
        source: anyhow::Error,
    },
}

/// 実行中のエミュレータ。
pub struct Engine {
    graph: Graph,
    parts: Vec<Box<dyn Part>>,
    base_step_ns: i64,
    t_ns: i64,
    tick: u64,
    /// 各部品の入力バッファ (今ステップで step に渡す値)
    inputs: Vec<Vec<f64>>,
    /// latency 1 の配線が次ステップ向けに書く先
    next_inputs: Vec<Vec<f64>>,
    /// 各部品の最新出力
    outputs: Vec<Vec<f64>>,
    logs: Vec<(String, crate::hal::LogLevel, String)>,
}

impl Engine {
    /// 部品は `graph.instances` と同じ順で渡す。`init` 済みであること。
    pub fn new(graph: Graph, parts: Vec<Box<dyn Part>>, base_step_ns: i64) -> anyhow::Result<Self> {
        anyhow::ensure!(
            parts.len() == graph.instances.len(),
            "部品数が manifest と一致しません"
        );
        anyhow::ensure!(base_step_ns > 0, "baseStepNs は正である必要があります");
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for d in &graph.descs {
            inputs.push(d.inputs.iter().map(|p| p.default).collect::<Vec<f64>>());
            outputs.push(d.outputs.iter().map(|p| p.default).collect::<Vec<f64>>());
        }
        let next_inputs = inputs.clone();
        let mut e = Self {
            graph,
            parts,
            base_step_ns,
            t_ns: 0,
            tick: 0,
            inputs,
            next_inputs,
            outputs,
            logs: Vec::new(),
        };
        // 初期出力を読んでおく (配線の初期値)
        for i in 0..e.parts.len() {
            let (parts, outputs) = (&mut e.parts, &mut e.outputs);
            parts[i].get_outputs(&mut outputs[i])?;
        }
        e.propagate_all_latency1();
        e.inputs = e.next_inputs.clone();
        Ok(e)
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }
    pub fn time_ns(&self) -> i64 {
        self.t_ns
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn base_step_ns(&self) -> i64 {
        self.base_step_ns
    }
    pub fn part_mut(&mut self, index: usize) -> &mut dyn Part {
        self.parts[index].as_mut()
    }
    /// 部品の最新出力。
    pub fn output(&self, inst: usize, port: usize) -> f64 {
        self.outputs[inst][port]
    }
    /// `instance.port` で最新出力を引く。
    pub fn output_by_ref(&self, port_ref: &str) -> Option<f64> {
        let (i, p) = self.graph.resolve_output(port_ref)?;
        Some(self.outputs[i][p])
    }
    /// 部品のログを回収する (instance, level, message)。
    pub fn drain_logs(&mut self) -> Vec<(String, crate::hal::LogLevel, String)> {
        std::mem::take(&mut self.logs)
    }

    /// 1 ステップ進める。
    pub fn step(&mut self) -> Result<(), StepError> {
        let n = self.parts.len();
        let dt = self.base_step_ns;
        for k in 0..self.graph.order.len() {
            let i = self.graph.order[k];
            if !self.tick.is_multiple_of(self.graph.rate_dividers[i] as u64) {
                continue;
            }
            let part = &mut self.parts[i];
            let run = (|| -> anyhow::Result<()> {
                part.set_inputs(&self.inputs[i])?;
                part.step(dt * self.graph.rate_dividers[i] as i64)?;
                part.get_outputs(&mut self.outputs[i])?;
                Ok(())
            })();
            if let Err(source) = run {
                return Err(StepError::Part {
                    instance: self.graph.instances[i].clone(),
                    t_ns: self.t_ns,
                    source,
                });
            }
            for (lvl, msg) in part.drain_logs() {
                self.logs.push((self.graph.instances[i].clone(), lvl, msg));
            }
            // latency 0: 同一ステップで後続の入力へ
            for w in self
                .graph
                .wires
                .iter()
                .filter(|w| w.from_inst == i && w.latency == 0)
            {
                let v = self.outputs[i][w.from_port] * w.gain + w.offset;
                self.inputs[w.to_inst][w.to_port] = v;
                self.next_inputs[w.to_inst][w.to_port] = v;
            }
        }
        debug_assert_eq!(n, self.parts.len());
        self.propagate_all_latency1();
        std::mem::swap(&mut self.inputs, &mut self.next_inputs);
        // next_inputs は「今の inputs」のコピーから始める (未配線の入力は保持される)
        for i in 0..n {
            self.next_inputs[i].clone_from(&self.inputs[i]);
        }
        self.t_ns += dt;
        self.tick += 1;
        Ok(())
    }

    fn propagate_all_latency1(&mut self) {
        for w in self.graph.wires.iter().filter(|w| w.latency == 1) {
            self.next_inputs[w.to_inst][w.to_port] =
                self.outputs[w.from_inst][w.from_port] * w.gain + w.offset;
        }
    }

    /// 全部品を reset して時刻を 0 に戻す (Epoch を進める操作)。
    pub fn reset(&mut self) -> anyhow::Result<()> {
        for p in &mut self.parts {
            p.reset()?;
        }
        self.t_ns = 0;
        self.tick = 0;
        for (i, d) in self.graph.descs.iter().enumerate() {
            for (j, port) in d.inputs.iter().enumerate() {
                self.inputs[i][j] = port.default;
                self.next_inputs[i][j] = port.default;
            }
            self.parts[i].get_outputs(&mut self.outputs[i])?;
        }
        self.propagate_all_latency1();
        self.inputs = self.next_inputs.clone();
        Ok(())
    }
}
