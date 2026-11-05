//! plant-cartpole — 台車 + 振子 + DC モータ (逆起電力) + メカストッパ。1ms RK4 (ip-model)。
//!
//! 任意の `pushes` パラメータで、NetworkedSil の `VirtualPlant.Push` 相当の周期外乱
//! (振子軸トルクのパルス、向きを交互に反転) を部品の中で発生できる (M6 の遅延比較, D62)。
//! 指定しなければ従来どおり外乱は入力ポート (`force-dist` / `torque-dist`) だけ。

use ip_model::dynamics::{pendulum_energy, rk4_step, Disturbance, State};
use ip_model::{Derived, Params};
use ip_part_sdk::{num, num_nested, scalar, scalar_default, PartDesc, PartImpl};

const IN_VOLTAGE: usize = 0;
const IN_FORCE_DIST: usize = 1;
const IN_TORQUE_DIST: usize = 2;

pub struct Plant {
    params: Params,
    derived: Derived,
    initial: State,
    state: State,
    inputs: [f64; 3],
    hit_stopper: bool,
    /// 積分の内部刻み [s]。ステップ dt がこれより長ければ分割する
    substep_s: f64,
    /// 周期外乱の予定 (任意)
    pushes: Option<Pushes>,
    /// init / reset からの経過時間 [ns] (周期外乱の基準)
    t_ns: i64,
}

/// 周期外乱: `firstAtMs` から `everyMs` ごとに `torqueNm` を `durationMs` だけ振子軸に加える。
/// 向きは 1 回目が +x 側へ倒す向き (`torqueNm` の符号どおり) で、`alternate` なら毎回反転する。
/// ステップ開始時刻が [開始, 開始 + durationMs) に入るステップでトルクを加える
/// (NetworkedSil の `Push` は 1ms ステップの先頭で外乱の終了を判定するので、これと同じ区間になる)。
#[derive(Debug, Clone, Copy)]
struct Pushes {
    torque: f64,
    duration_ns: i64,
    first_ns: i64,
    every_ns: i64,
    /// 0 なら無制限
    count: u64,
    alternate: bool,
}

impl Pushes {
    fn from_json(v: &serde_json::Value) -> Result<Option<Self>, String> {
        let Some(p) = v.get("pushes") else {
            return Ok(None);
        };
        if p.is_null() {
            return Ok(None);
        }
        if !p.is_object() {
            return Err("pushes はオブジェクト".into());
        }
        let torque = num(p, "torqueNm", 0.3);
        let duration_ms = num(p, "durationMs", 50.0);
        let first_ms = num(p, "firstAtMs", 2000.0);
        let every_ms = num(p, "everyMs", 2000.0);
        let count = num(p, "count", 0.0);
        let alternate = p.get("alternate").and_then(|a| a.as_bool()).unwrap_or(true);
        if !(torque.is_finite()
            && duration_ms > 0.0
            && first_ms >= 0.0
            && every_ms >= duration_ms
            && count >= 0.0)
        {
            return Err(
                "pushes: torqueNm は有限, durationMs > 0, firstAtMs >= 0, everyMs >= durationMs, count >= 0"
                    .into(),
            );
        }
        Ok(Some(Self {
            torque,
            duration_ns: (duration_ms * 1e6).round() as i64,
            first_ns: (first_ms * 1e6).round() as i64,
            every_ns: (every_ms * 1e6).round() as i64,
            count: count as u64,
            alternate,
        }))
    }

    /// ステップ開始時刻 `t_ns` に加えるトルク [Nm]。
    fn torque_at(&self, t_ns: i64) -> f64 {
        if t_ns < self.first_ns {
            return 0.0;
        }
        let since = t_ns - self.first_ns;
        let n = since / self.every_ns;
        if self.count > 0 && n as u64 >= self.count {
            return 0.0;
        }
        if since - n * self.every_ns >= self.duration_ns {
            return 0.0;
        }
        if self.alternate && n % 2 == 1 {
            -self.torque
        } else {
            self.torque
        }
    }
}

fn params_from_json(v: &serde_json::Value) -> Params {
    let d = Params::default();
    Params {
        cart_mass: num(v, "cartMass", d.cart_mass),
        rotor_inertia: num(v, "rotorInertia", d.rotor_inertia),
        pulley_radius: num_nested(v, &["motor", "pulleyRadius"], d.pulley_radius),
        torque_constant: num_nested(v, &["motor", "kt"], d.torque_constant),
        back_emf_constant: num_nested(v, &["motor", "ke"], d.back_emf_constant),
        motor_resistance: num_nested(v, &["motor", "resistance"], d.motor_resistance),
        max_voltage: num(v, "maxVoltage", d.max_voltage),
        cart_viscous_friction: num(v, "cartViscousFriction", d.cart_viscous_friction),
        cart_coulomb_friction: num(v, "cartCoulombFriction", d.cart_coulomb_friction),
        pendulum_mass: num(v, "poleMass", d.pendulum_mass),
        pendulum_length: num(v, "pendulumLength", d.pendulum_length),
        pivot_viscous_friction: num(v, "pivotFriction", d.pivot_viscous_friction),
        rail_stroke: num(v, "railStroke", d.rail_stroke),
        stop_stiffness: num(v, "stopperStiffness", d.stop_stiffness),
        stop_damping: num(v, "stopperDamping", d.stop_damping),
    }
}

impl PartImpl for Plant {
    fn describe() -> PartDesc {
        PartDesc {
            name: "plant-cartpole".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            inputs: vec![
                scalar("voltage", "V", "モータ端子電圧 (maxVoltage で飽和)"),
                scalar("force-dist", "N", "台車への外乱力"),
                scalar("torque-dist", "N*m", "振子軸まわりの外乱トルク"),
            ],
            outputs: vec![
                scalar("x", "m", "台車位置"),
                scalar_default(
                    "theta",
                    "rad",
                    core::f64::consts::PI,
                    "振子角 (0 = 倒立, 正 = +x 側へ倒れる)。正規化しない累積角",
                ),
                scalar("x-dot", "m/s", "台車速度"),
                scalar("theta-dot", "rad/s", "振子角速度"),
                scalar("hit-stopper", "1", "メカストッパ接触中なら 1"),
                scalar("energy", "J", "倒立静止を 0 とした振子エネルギー"),
                scalar("applied-voltage", "V", "飽和後の印加電圧"),
            ],
            params: Vec::new(),
            deterministic: true,
            doc: "台車型倒立振子の非線形モデル。NetworkedSil の CartPoleDynamics と同じ式".into(),
        }
    }

    fn init(v: &serde_json::Value) -> Result<Self, String> {
        let params = params_from_json(v);
        if !(params.pendulum_length > 0.0 && params.cart_mass > 0.0 && params.pendulum_mass > 0.0) {
            return Err("質量・長さは正である必要があります".into());
        }
        let initial = State::new(
            num_nested(v, &["initial", "x"], 0.0),
            num_nested(v, &["initial", "theta"], core::f64::consts::PI),
            num_nested(v, &["initial", "xDot"], 0.0),
            num_nested(v, &["initial", "thetaDot"], 0.0),
        );
        let substep_s = num(v, "substepSeconds", 0.001);
        if !(substep_s > 0.0 && substep_s <= 0.01) {
            return Err("substepSeconds は (0, 0.01] の範囲".into());
        }
        let pushes = Pushes::from_json(v)?;
        Ok(Self {
            derived: params.derive(),
            params,
            initial,
            state: initial,
            inputs: [0.0; 3],
            hit_stopper: false,
            substep_s,
            pushes,
            t_ns: 0,
        })
    }

    fn reset(&mut self) {
        self.state = self.initial;
        self.inputs = [0.0; 3];
        self.hit_stopper = false;
        self.t_ns = 0;
    }

    fn set_inputs(&mut self, values: &[f64]) {
        for (i, v) in values.iter().take(3).enumerate() {
            self.inputs[i] = if v.is_finite() { *v } else { 0.0 };
        }
    }

    fn step(&mut self, dt_ns: i64) -> Result<(), String> {
        if dt_ns <= 0 {
            return Err("dt は正".into());
        }
        let dt = dt_ns as f64 * 1e-9;
        let volts = ip_model::math::clamp(
            self.inputs[IN_VOLTAGE],
            -self.params.max_voltage,
            self.params.max_voltage,
        );
        let pushed = self.pushes.map_or(0.0, |p| p.torque_at(self.t_ns));
        let dist = Disturbance {
            force: self.inputs[IN_FORCE_DIST],
            torque: self.inputs[IN_TORQUE_DIST] + pushed,
        };
        // dt を内部刻み以下の等分に割る (dt=1ms, substep=1ms なら 1 回)
        let n = (dt / self.substep_s - 1e-9).ceil().max(1.0) as usize;
        let h = dt / n as f64;
        for _ in 0..n {
            self.state = rk4_step(&self.params, &self.derived, &self.state, volts, &dist, h);
        }
        if !self.state.to_array().iter().all(|v| v.is_finite()) {
            return Err("状態が発散しました (NaN/Inf)".into());
        }
        self.hit_stopper = self.state.x.abs() > self.params.rail_stroke;
        self.t_ns += dt_ns;
        Ok(())
    }

    fn get_outputs(&self, out: &mut Vec<f64>) {
        let s = &self.state;
        out.extend_from_slice(&[
            s.x,
            s.theta,
            s.x_dot,
            s.theta_dot,
            if self.hit_stopper { 1.0 } else { 0.0 },
            pendulum_energy(&self.params, &self.derived, s),
            ip_model::math::clamp(
                self.inputs[IN_VOLTAGE],
                -self.params.max_voltage,
                self.params.max_voltage,
            ),
        ]);
    }

    fn snapshot(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(32);
        for f in self.state.to_array() {
            v.extend_from_slice(&f.to_le_bytes());
        }
        v
    }

    fn restore(&mut self, state: &[u8]) -> Result<(), String> {
        if state.len() != 32 {
            return Err("snapshot の長さが不正".into());
        }
        let mut f = [0.0; 4];
        for (i, chunk) in state.chunks_exact(8).enumerate() {
            f[i] = f64::from_le_bytes(chunk.try_into().unwrap());
        }
        self.state = State::new(f[0], f[1], f[2], f[3]);
        Ok(())
    }
}

ip_part_sdk::part_entry!(Plant);
