//! 物理パラメータと派生量。

/// 重力加速度 [m/s^2]
pub const GRAVITY: f64 = 9.81;

/// 台車型倒立振子の物理パラメータ。既定値は NetworkedSil の `PendulumParameters` と同じ。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// 台車質量 [kg]
    pub cart_mass: f64,
    /// モータロータ慣性 [kg m^2]
    pub rotor_inertia: f64,
    /// プーリ半径 [m]
    pub pulley_radius: f64,
    /// トルク定数 [Nm/A]
    pub torque_constant: f64,
    /// 逆起電力定数 [Vs/rad]
    pub back_emf_constant: f64,
    /// 巻線抵抗 [ohm]
    pub motor_resistance: f64,
    /// 電源電圧 = 指令電圧の飽和値 [V]
    pub max_voltage: f64,
    /// 台車の粘性摩擦 [Ns/m]
    pub cart_viscous_friction: f64,
    /// 台車のクーロン摩擦 [N]
    pub cart_coulomb_friction: f64,
    /// 振子質量 [kg]
    pub pendulum_mass: f64,
    /// 振子全長 [m] (重心は L/2)
    pub pendulum_length: f64,
    /// 振子軸の粘性摩擦 [Nms/rad]
    pub pivot_viscous_friction: f64,
    /// レール片側ストローク [m]
    pub rail_stroke: f64,
    /// メカストッパ剛性 [N/m]
    pub stop_stiffness: f64,
    /// メカストッパ減衰 [Ns/m]
    pub stop_damping: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            cart_mass: 0.6,
            rotor_inertia: 2e-5,
            pulley_radius: 0.015,
            torque_constant: 0.05,
            back_emf_constant: 0.05,
            motor_resistance: 2.0,
            max_voltage: 24.0,
            cart_viscous_friction: 1.0,
            cart_coulomb_friction: 0.4,
            pendulum_mass: 0.15,
            pendulum_length: 0.6,
            pivot_viscous_friction: 8e-4,
            rail_stroke: 0.5,
            stop_stiffness: 2e4,
            stop_damping: 150.0,
        }
    }
}

impl Params {
    /// 摩擦をすべて 0 にした保存系 (エネルギー保存の性質テスト用)。
    pub fn frictionless(self) -> Self {
        Self {
            cart_viscous_friction: 0.0,
            cart_coulomb_friction: 0.0,
            pivot_viscous_friction: 0.0,
            back_emf_constant: 0.0,
            ..self
        }
    }

    pub fn derive(&self) -> Derived {
        Derived::from(self)
    }
}

/// 積分ループの内側で毎回割り算をしないための派生量。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Derived {
    /// 軸から重心までの距離 [m]
    pub half_length: f64,
    /// 軸まわりの慣性モーメント [kg m^2]
    pub pivot_inertia: f64,
    /// ロータ慣性を並進換算して加えた等価台車質量 [kg]
    pub equivalent_cart_mass: f64,
    /// 電圧→推力ゲイン [N/V]
    pub force_gain: f64,
    /// 逆起電力による速度比例抗力 [Ns/m]
    pub back_emf_damping: f64,
}

impl From<&Params> for Derived {
    fn from(p: &Params) -> Self {
        let half_length = p.pendulum_length / 2.0;
        let center_inertia = p.pendulum_mass * p.pendulum_length * p.pendulum_length / 12.0;
        Self {
            half_length,
            pivot_inertia: center_inertia + p.pendulum_mass * half_length * half_length,
            equivalent_cart_mass: p.cart_mass
                + p.rotor_inertia / (p.pulley_radius * p.pulley_radius),
            force_gain: p.torque_constant / (p.motor_resistance * p.pulley_radius),
            back_emf_damping: p.torque_constant * p.back_emf_constant
                / (p.motor_resistance * p.pulley_radius * p.pulley_radius),
        }
    }
}
