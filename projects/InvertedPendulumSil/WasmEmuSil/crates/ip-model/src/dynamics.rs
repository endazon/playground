//! 非線形運動方程式と RK4 積分。

use crate::math;
use crate::params::{Derived, Params, GRAVITY};

/// プラント状態 [x, θ, ẋ, θ̇]。θ=0 が倒立、正の向きは +x 側へ倒れる向き。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct State {
    pub x: f64,
    pub theta: f64,
    pub x_dot: f64,
    pub theta_dot: f64,
}

impl State {
    pub const fn new(x: f64, theta: f64, x_dot: f64, theta_dot: f64) -> Self {
        Self {
            x,
            theta,
            x_dot,
            theta_dot,
        }
    }
    pub fn hanging() -> Self {
        Self::new(0.0, core::f64::consts::PI, 0.0, 0.0)
    }
    pub fn near_upright(theta_offset: f64) -> Self {
        Self::new(0.0, theta_offset, 0.0, 0.0)
    }
    #[inline]
    fn add(self, o: State) -> State {
        State::new(
            self.x + o.x,
            self.theta + o.theta,
            self.x_dot + o.x_dot,
            self.theta_dot + o.theta_dot,
        )
    }
    #[inline]
    fn scale(self, k: f64) -> State {
        State::new(
            self.x * k,
            self.theta * k,
            self.x_dot * k,
            self.theta_dot * k,
        )
    }
    pub fn to_array(self) -> [f64; 4] {
        [self.x, self.theta, self.x_dot, self.theta_dot]
    }
}

/// 外乱入力。`force` は台車への水平力 [N]、`torque` は振子軸まわりのトルク [Nm]。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Disturbance {
    pub force: f64,
    pub torque: f64,
}

impl Disturbance {
    pub const NONE: Disturbance = Disturbance {
        force: 0.0,
        torque: 0.0,
    };
}

/// 状態微分 ds/dt。
pub fn derivative(p: &Params, d: &Derived, s: &State, volts: f64, dist: &Disturbance) -> State {
    let c = math::cos(s.theta);
    let sn = math::sin(s.theta);

    // 電圧→推力。逆起電力ぶんは速度に比例する抗力として現れる。
    let mut force = d.force_gain * volts
        - (d.back_emf_damping + p.cart_viscous_friction) * s.x_dot
        - p.cart_coulomb_friction * math::tanh(s.x_dot / 0.01)
        + dist.force;

    // メカストッパ: ばね + ダンパ。押し戻す向きの力しか出さない。
    let penetration = if s.x > p.rail_stroke {
        s.x - p.rail_stroke
    } else if s.x < -p.rail_stroke {
        s.x + p.rail_stroke
    } else {
        0.0
    };
    if penetration != 0.0 {
        let mut stop_force = -p.stop_stiffness * penetration - p.stop_damping * s.x_dot;
        if stop_force * penetration > 0.0 {
            stop_force = 0.0;
        }
        force += stop_force;
    }

    let a11 = d.equivalent_cart_mass + p.pendulum_mass;
    let a12 = p.pendulum_mass * d.half_length * c;
    let det = a11 * d.pivot_inertia - a12 * a12;

    let b1 = force + p.pendulum_mass * d.half_length * sn * s.theta_dot * s.theta_dot;
    let b2 = p.pendulum_mass * GRAVITY * d.half_length * sn
        - p.pivot_viscous_friction * s.theta_dot
        + dist.torque;

    State::new(
        s.x_dot,
        s.theta_dot,
        (b1 * d.pivot_inertia - a12 * b2) / det,
        (a11 * b2 - a12 * b1) / det,
    )
}

/// 刻み h [s] の RK4 で 1 ステップ進める。
pub fn rk4_step(
    p: &Params,
    d: &Derived,
    s: &State,
    volts: f64,
    dist: &Disturbance,
    h: f64,
) -> State {
    let k1 = derivative(p, d, s, volts, dist);
    let k2 = derivative(p, d, &s.add(k1.scale(h / 2.0)), volts, dist);
    let k3 = derivative(p, d, &s.add(k2.scale(h / 2.0)), volts, dist);
    let k4 = derivative(p, d, &s.add(k3.scale(h)), volts, dist);
    s.add(k1.add(k2.add(k3).scale(2.0)).add(k4).scale(h / 6.0))
}

/// 倒立静止を 0 とした振子の力学エネルギー [J] (スイングアップの評価量)。
pub fn pendulum_energy(p: &Params, d: &Derived, s: &State) -> f64 {
    0.5 * d.pivot_inertia * s.theta_dot * s.theta_dot
        + p.pendulum_mass * GRAVITY * d.half_length * (math::cos(s.theta) - 1.0)
}

/// 系全体の力学エネルギー [J]。運動エネルギー (台車 + 振子の並進・回転) + 振子の位置エネルギー。
/// 摩擦 0・電圧 0・外乱 0・ストッパ非接触なら保存する (性質テストに使う)。
pub fn total_energy(p: &Params, d: &Derived, s: &State) -> f64 {
    let c = math::cos(s.theta);
    let sn = math::sin(s.theta);
    // 重心速度: (ẋ + l cosθ θ̇, -l sinθ θ̇) ... θ=0 が倒立 (上向き) で +x 側へ倒れる定義
    let vx = s.x_dot + d.half_length * c * s.theta_dot;
    let vy = -d.half_length * sn * s.theta_dot;
    let center_inertia = p.pendulum_mass * p.pendulum_length * p.pendulum_length / 12.0;
    let kinetic = 0.5 * d.equivalent_cart_mass * s.x_dot * s.x_dot
        + 0.5 * p.pendulum_mass * (vx * vx + vy * vy)
        + 0.5 * center_inertia * s.theta_dot * s.theta_dot;
    let potential = p.pendulum_mass * GRAVITY * d.half_length * c;
    kinetic + potential
}
