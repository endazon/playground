//! 位置差分 + 1 次ローパスによる速度推定。

use crate::math;

/// 1 次ローパスの離散ゲイン 1 - exp(-2π fc dt)。
pub fn low_pass_gain(cutoff_hz: f64, dt: f64) -> f64 {
    1.0 - math::exp(-2.0 * core::f64::consts::PI * cutoff_hz * dt)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VelocityEstimator {
    pub cutoff_hz: f64,
    pub value: f64,
    pub last_input: f64,
    pub initialized: bool,
}

impl VelocityEstimator {
    pub const fn new(cutoff_hz: f64) -> Self {
        Self {
            cutoff_hz,
            value: 0.0,
            last_input: 0.0,
            initialized: false,
        }
    }
    pub fn reset(&mut self, input: f64) {
        self.last_input = input;
        self.value = 0.0;
        self.initialized = true;
    }
    /// 新しい計測値で更新して推定速度を返す。dt<=0 のサンプルは無視する。
    pub fn update(&mut self, input: f64, dt: f64) -> f64 {
        if !self.initialized {
            self.reset(input);
            return self.value;
        }
        if dt > 0.0 {
            let a = low_pass_gain(self.cutoff_hz, dt);
            self.value += a * ((input - self.last_input) / dt - self.value);
        }
        self.last_input = input;
        self.value
    }
}
