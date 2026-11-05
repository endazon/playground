//! 理論遅延余裕: 閉ループが発散しないループ遅延の上限を数値シミュレーション + 二分探索で求める。
//!
//! NetworkedSil の `Ip.Shared.Control.DelayMarginAnalyzer` (C#) の移植。構造・定数・判定はすべて同じにしてある
//! (Docs/10-delay-comparison.md で NetworkedSil の表と突き合わせるため)。
//!
//! 解析的な位相余裕ではなく、実装と同じ構造 —— 周期 Ts のサンプル、差分 + LPF の速度推定、
//! ZOH の電圧保持、一定遅延 —— を 1ms 刻みで線形モデルのまま模擬し、発散するかどうかで二分探索する。
//! 電圧の飽和・量子化・摩擦の非線形は含まない (線形化モデル (A, B) だけを使う)。

use crate::lqr::{self, Linearized};
use crate::matrix::{self, Mat4, Vec4, N};
use crate::params::Params;

/// 積分の刻み [s] (C# 版の `StepSeconds`)。
pub const STEP_SECONDS: f64 = 0.001;
/// 模擬するステップ数 (24 s)。
pub const SIMULATION_STEPS: usize = 24_000;
/// 二分探索の上限 [ms]。
pub const MAX_DELAY_MS: u32 = 600;
/// NetworkedSil の既定の速度推定 LPF カットオフ [Hz] (`ControlOptions.VelocityCutoffHz`)。
pub const DEFAULT_CUTOFF_HZ: f64 = 25.0;

/// C# の `Math.Round` (既定は偶数丸め) と同じ丸め。
fn round_half_even(x: f64) -> f64 {
    libm::rint(x)
}

/// 遅延 [ms] を積分ステップ数へ。`STEP_SECONDS` を変えても遅延の意味が変わらないようにする (C# 版と同じ)。
fn delay_to_steps(delay_ms: u32) -> usize {
    round_half_even(delay_ms as f64 / (STEP_SECONDS * 1000.0)) as usize
}

/// 遅延解析の前計算 (1ms の ZOH 離散化と速度推定のゲイン)。
#[derive(Debug, Clone, Copy)]
pub struct Analyzer {
    ad: Mat4,
    bd: Vec4,
    k: Vec4,
    sub_steps: usize,
    lpf: f64,
}

impl Analyzer {
    /// `a`, `b`: 連続時間モデル。`k`: 状態フィードバックゲイン (u = -K x)。
    /// `ts`: 帰還周期 [s]。`cutoff_hz`: 速度推定 LPF のカットオフ [Hz]。
    pub fn new(a: &Mat4, b: &Vec4, k: &Vec4, ts: f64, cutoff_hz: f64) -> Self {
        let (ad, bd) = matrix::zero_order_hold(a, b, STEP_SECONDS);
        let sub = round_half_even(ts / STEP_SECONDS) as i64;
        let sub_steps = if sub < 1 { 1 } else { sub as usize };
        let lpf = crate::estimator::low_pass_gain(cutoff_hz, sub_steps as f64 * STEP_SECONDS);
        Self {
            ad,
            bd,
            k: *k,
            sub_steps,
            lpf,
        }
    }

    /// 一定のループ遅延 `delay_ms` (`MAX_DELAY_MS` 以下) で閉ループが不安定か。
    /// 発散 (|θ| が 1e3 を超える / 非有限) するか、後半 1/3 の |θ| のピークが中盤 1/3 のピークの 98% を超えて
    /// 減衰していない (かつ 1e-4 rad より大きい) なら不安定とみなす (C# 版と同じ判定)。
    pub fn is_unstable(&self, delay_ms: u32) -> bool {
        assert!(delay_ms <= MAX_DELAY_MS, "delay_ms は {MAX_DELAY_MS} 以下");
        let delay_steps = delay_to_steps(delay_ms);
        // 遅延キュー: (適用ステップ, 電圧)。適用ステップは単調に増えるので固定長のリングで足りる
        let mut queue = Ring::new();
        let mut u = 0.0;
        let mut x: Vec4 = [0.0, 0.01, 0.0, 0.0];
        let (mut prev_x, mut prev_th, mut vx, mut vth) = (0.0, 0.01, 0.0, 0.0);
        let (mut first_half_peak, mut second_half_peak): (f64, f64) = (0.0, 0.0);
        let dt = self.sub_steps as f64 * STEP_SECONDS;
        let k = &self.k;

        for step in 0..SIMULATION_STEPS {
            if step % self.sub_steps == 0 {
                vx += self.lpf * ((x[0] - prev_x) / dt - vx);
                vth += self.lpf * ((x[1] - prev_th) / dt - vth);
                prev_x = x[0];
                prev_th = x[1];
                queue.push(
                    step + delay_steps,
                    -(k[0] * x[0] + k[1] * x[1] + k[2] * vx + k[3] * vth),
                );
            }

            while let Some(v) = queue.pop_due(step) {
                u = v;
            }

            let mut nx = matrix::mul_vec(&self.ad, &x);
            for (i, v) in nx.iter_mut().enumerate().take(N) {
                *v += self.bd[i] * u;
            }
            x = nx;

            let magnitude = crate::math::fabs(x[1]);
            // NaN も不安定に含める (`!(m < 1e3)`)
            if magnitude.is_nan() || magnitude >= 1e3 {
                return true;
            }
            if step > SIMULATION_STEPS / 3 && step <= 2 * SIMULATION_STEPS / 3 {
                first_half_peak = first_half_peak.max(magnitude);
            } else if step > 2 * SIMULATION_STEPS / 3 {
                second_half_peak = second_half_peak.max(magnitude);
            }
        }

        // 発散はしていないが減衰もしていない (振幅が保たれている) 場合も不安定とみなす
        second_half_peak > first_half_peak * 0.98 && second_half_peak > 1e-4
    }

    /// 倒立を維持できる最大のループ遅延 [ms] (0〜`MAX_DELAY_MS` の二分探索)。
    /// 0 なら遅延ゼロでも安定化できない。
    pub fn margin_ms(&self) -> u32 {
        if self.is_unstable(0) {
            return 0;
        }
        if !self.is_unstable(MAX_DELAY_MS) {
            return MAX_DELAY_MS;
        }
        let (mut lo, mut hi) = (0u32, MAX_DELAY_MS);
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if self.is_unstable(mid) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        lo
    }
}

/// `DelayMarginAnalyzer.Compute` と同じ入口。
pub fn compute(a: &Mat4, b: &Vec4, k: &Vec4, ts: f64, cutoff_hz: f64) -> u32 {
    Analyzer::new(a, b, k, ts, cutoff_hz).margin_ms()
}

/// 設計結果 (NetworkedSil の `ControllerCore.CreateDesign` が返す `DesignInfo` 相当)。
#[derive(Debug, Clone, Copy)]
pub struct DelayDesign {
    /// 離散 LQR ゲイン (周期 Ts で ZOH 離散化したモデルに対する設計)
    pub k: Vec4,
    /// 倒立点の不安定極 [rad/s]
    pub unstable_pole: f64,
    /// 理論遅延余裕 [ms]
    pub margin_ms: u32,
    pub period_ms: u32,
}

/// 周期 `period_ms` で既定の重み ([`lqr::DEFAULT_STATE_WEIGHTS`], [`lqr::DEFAULT_INPUT_WEIGHT`]) の離散 LQR を設計し、
/// その閉ループの理論遅延余裕を求める。NetworkedSil の `ControllerCore.CreateDesign` と同じ経路。
pub fn design(p: &Params, period_ms: u32, cutoff_hz: f64) -> Option<DelayDesign> {
    let ts = period_ms as f64 / 1000.0;
    let (sol, lin, _, _): (_, Linearized, _, _) = lqr::design_gains(
        p,
        ts,
        &lqr::DEFAULT_STATE_WEIGHTS,
        lqr::DEFAULT_INPUT_WEIGHT,
    )?;
    Some(DelayDesign {
        k: sol.k,
        unstable_pole: lin.unstable_pole,
        margin_ms: compute(&lin.a, &lin.b, &sol.k, ts, cutoff_hz),
        period_ms,
    })
}

/// 遅延キュー。保留できるのは高々 `MAX_DELAY_MS + 1` 件 (サンプルは 1 ステップに 1 件以下、
/// 遅延は `MAX_DELAY_MS` ステップ以下) なので、`no_std` でも使える固定長のリングにする。
struct Ring {
    buf: [(usize, f64); RING_LEN],
    head: usize,
    len: usize,
}

const RING_LEN: usize = MAX_DELAY_MS as usize + 2;

impl Ring {
    fn new() -> Self {
        Self {
            buf: [(0, 0.0); RING_LEN],
            head: 0,
            len: 0,
        }
    }
    fn push(&mut self, at: usize, u: f64) {
        assert!(self.len < RING_LEN, "遅延キューがあふれた");
        self.buf[(self.head + self.len) % RING_LEN] = (at, u);
        self.len += 1;
    }
    /// 先頭が `step` までに適用されるなら取り出す。
    fn pop_due(&mut self, step: usize) -> Option<f64> {
        if self.len == 0 || self.buf[self.head].0 > step {
            return None;
        }
        let v = self.buf[self.head].1;
        self.head = (self.head + 1) % RING_LEN;
        self.len -= 1;
        Some(v)
    }
}
