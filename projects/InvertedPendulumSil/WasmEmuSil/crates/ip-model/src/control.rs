//! エネルギー法スイングアップと LQR の操作量計算。ファームウェア (Rust 版) と検証が共有する。

use crate::dynamics::{pendulum_energy, State};
use crate::math;
use crate::params::{Derived, Params};

/// エネルギー法スイングアップの調整ゲイン。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwingUpGains {
    pub energy_gain: f64,
    pub acceleration_amplitude: f64,
    pub position_gain: f64,
    pub velocity_gain: f64,
    pub energy_offset: f64,
    pub acceleration_limit: f64,
    pub voltage_limit: f64,
}

impl Default for SwingUpGains {
    fn default() -> Self {
        Self {
            energy_gain: 80.0,
            acceleration_amplitude: 3.0,
            position_gain: 5.0,
            velocity_gain: 3.0,
            energy_offset: 0.05,
            acceleration_limit: 5.0,
            voltage_limit: 18.0,
        }
    }
}

/// エネルギー法によるスイングアップ電圧。
/// dE/dt = -m·l·cosθ·θ̇·ẍ なので ẍ ∝ (E - E*)·sgn(cosθ·θ̇) とすれば E は目標へ単調に近づく。
pub fn swing_up_voltage(p: &Params, d: &Derived, s: &State, g: &SwingUpGains) -> f64 {
    let energy = pendulum_energy(p, d, s);
    let mut sign = math::signum(s.theta_dot * math::cos(s.theta));
    if sign == 0.0 {
        sign = 1.0;
    }
    let mut accel = g.acceleration_amplitude
        * math::clamp(g.energy_gain * (energy - g.energy_offset), -1.0, 1.0)
        * sign
        - g.position_gain * s.x
        - g.velocity_gain * s.x_dot;
    accel = math::clamp(accel, -g.acceleration_limit, g.acceleration_limit);

    // 加速度 → 推力 → 電圧 (逆起電力と摩擦の前向き補償を含む)
    let volts = ((d.equivalent_cart_mass + p.pendulum_mass) * accel
        + (d.back_emf_damping + p.cart_viscous_friction) * s.x_dot
        + p.cart_coulomb_friction * math::tanh(s.x_dot / 0.01))
        / d.force_gain;
    math::clamp(volts, -g.voltage_limit, g.voltage_limit)
}

/// LQR の操作量 u = -K (x - x_ref)。θ は正規化済みであること。
pub fn lqr_voltage(k: &[f64; 4], x_err: f64, theta: f64, x_dot: f64, theta_dot: f64) -> f64 {
    -(k[0] * x_err + k[1] * theta + k[2] * x_dot + k[3] * theta_dot)
}
