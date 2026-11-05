//! fw-balance (Rust 版) — 仮想 MCU で走る倒立振子の制御ファームウェア。
//!
//! NetworkedSil の `ControllerCore` を、通信を持たない MCU のファームウェアとして書き直したもの。
//! HAL (`ip_hal`) だけに依存し、同じソースが wasm (配布) とネイティブ共有ライブラリ (デバッグ) になる。
//!
//! 周辺の割当 (manifest の cpu-virtual と対応):
//!   enc0 = 台車, enc1 = 振子, pwm0 = デューティ, gpio-out0 = ドライブ有効, gpio-out1 = ハートビート,
//!   gpio-in0 = ドライブ FAULT, gpio-in1 = ドライブ有効応答, uart0 = オペレータ操作 ('s' 'b' 'x' 'c')

mod cfg;

use ip_hal::LogLevel;
use ip_model::control::{lqr_voltage, swing_up_voltage, SwingUpGains};
use ip_model::dynamics::State;
use ip_model::estimator::VelocityEstimator;
use ip_model::math;
use ip_model::{Derived, Params};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    Idle = 0,
    SwingUp = 1,
    Balance = 2,
    Fault = 3,
}

#[derive(Clone, Copy)]
pub struct Config {
    pub params: Params,
    pub derived: Derived,
    pub gains: [f64; 4],
    pub swing: SwingUpGains,
    pub bus_voltage: f64,
    pub cart_meters_per_count: f64,
    pub pend_counts_per_rev: f64,
    pub control_divider: u32,
    pub control_dt: f64,
    pub velocity_cutoff_hz: f64,
    pub fall_angle: f64,
    pub soft_limit: f64,
    pub capture_angle: f64,
    pub capture_rate: f64,
    pub target_x: f64,
    pub target_ramp_m_per_s: f64,
    pub auto_start: Mode,
    pub auto_start_delay_us: u64,
}

pub struct Fw {
    cfg: Config,
    mode: Mode,
    tick: u64,
    cart_vel: VelocityEstimator,
    pend_vel: VelocityEstimator,
    est_ready: bool,
    x: f64,
    theta: f64,
    target_x: f64,
    volts: f64,
    heartbeat: bool,
    started: bool,
}

impl Fw {
    fn set_mode(&mut self, m: Mode, reason: &str) {
        if m != self.mode {
            ip_hal::log(
                if m == Mode::Fault {
                    LogLevel::Error
                } else {
                    LogLevel::Info
                },
                &format!("{:?} -> {:?} ({reason})", self.mode, m),
            );
            self.mode = m;
        }
    }

    fn fault(&mut self, reason: &str) {
        self.volts = 0.0;
        ip_hal::pwm_write(0, 0.0);
        ip_hal::gpio_write(0, false);
        self.set_mode(Mode::Fault, reason);
    }

    fn start(&mut self, target: Mode) {
        if self.mode != Mode::Idle {
            ip_hal::log(
                LogLevel::Warn,
                &format!("{:?} 中は開始できません", self.mode),
            );
            return;
        }
        self.est_ready = false;
        self.target_x = 0.0;
        self.volts = 0.0;
        ip_hal::gpio_write(0, true); // ドライブ有効 (立ち上がりで保護解除)
        self.set_mode(target, "開始操作");
    }

    fn stop(&mut self) {
        if matches!(self.mode, Mode::SwingUp | Mode::Balance) {
            self.volts = 0.0;
            ip_hal::pwm_write(0, 0.0);
            ip_hal::gpio_write(0, false);
            self.set_mode(Mode::Idle, "停止操作");
        }
    }

    fn handle_commands(&mut self) {
        for b in ip_hal::uart_read(0, 16) {
            match b {
                b's' => self.start(Mode::SwingUp),
                b'b' => self.start(Mode::Balance),
                b'x' => self.stop(),
                b'c' => {
                    if self.mode == Mode::Fault {
                        self.set_mode(Mode::Idle, "異常リセット");
                    }
                }
                _ => {}
            }
        }
    }

    fn control(&mut self) {
        let c = self.cfg; // Copy (借用を持ち越さない)
                          // 計測
        let x = ip_hal::enc_read(0) as f64 * c.cart_meters_per_count;
        let theta_raw =
            ip_hal::enc_read(1) as f64 * (2.0 * core::f64::consts::PI) / c.pend_counts_per_rev;
        if !self.est_ready {
            self.cart_vel.reset(x);
            self.pend_vel.reset(theta_raw);
            self.est_ready = true;
        } else {
            self.cart_vel.update(x, c.control_dt);
            self.pend_vel.update(theta_raw, c.control_dt);
        }
        self.x = x;
        self.theta = math::wrap_angle(theta_raw);
        let x_dot = self.cart_vel.value;
        let theta_dot = self.pend_vel.value;

        ip_hal::trace(0, self.theta);
        ip_hal::trace(1, self.x);

        if !matches!(self.mode, Mode::SwingUp | Mode::Balance) {
            return;
        }

        // ドライブ側の保護を冗長に監視
        if ip_hal::gpio_read(0) {
            self.fault("ドライブ FAULT 入力");
            return;
        }

        let mut volts;
        if self.mode == Mode::SwingUp {
            if self.theta.abs() < c.capture_angle && theta_dot.abs() < c.capture_rate {
                self.set_mode(Mode::Balance, "倒立領域に到達");
            } else {
                if self.x.abs() > c.soft_limit {
                    self.fault("スイングアップ中にソフトリミット超過");
                    return;
                }
                let s = State::new(self.x, theta_raw, x_dot, theta_dot);
                volts = swing_up_voltage(&c.params, &c.derived, &s, &c.swing);
                self.output(volts);
                return;
            }
        }

        if self.theta.abs() > c.fall_angle {
            self.fault("転倒検知");
            return;
        }
        if self.x.abs() > c.soft_limit {
            self.fault("ソフトリミット超過");
            return;
        }
        let ramp = c.target_ramp_m_per_s * c.control_dt;
        self.target_x += math::clamp(c.target_x - self.target_x, -ramp, ramp);
        volts = lqr_voltage(
            &c.gains,
            self.x - self.target_x,
            self.theta,
            x_dot,
            theta_dot,
        );
        volts = math::clamp(volts, -c.params.max_voltage, c.params.max_voltage);
        self.output(volts);
    }

    fn output(&mut self, volts: f64) {
        self.volts = volts;
        let duty = math::clamp(volts / self.cfg.bus_voltage, -1.0, 1.0);
        ip_hal::pwm_write(0, duty as f32);
        ip_hal::trace(2, volts);
    }
}

fn parse_config(json: &str) -> Result<Config, String> {
    use cfg::{number, string};
    let d = Params::default();
    let params = Params {
        cart_mass: number(json, "cartMass", d.cart_mass),
        rotor_inertia: number(json, "rotorInertia", d.rotor_inertia),
        pulley_radius: number(json, "pulleyRadius", d.pulley_radius),
        torque_constant: number(json, "kt", d.torque_constant),
        back_emf_constant: number(json, "ke", d.back_emf_constant),
        motor_resistance: number(json, "resistance", d.motor_resistance),
        max_voltage: number(json, "maxVoltage", d.max_voltage),
        cart_viscous_friction: number(json, "cartViscousFriction", d.cart_viscous_friction),
        cart_coulomb_friction: number(json, "cartCoulombFriction", d.cart_coulomb_friction),
        pendulum_mass: number(json, "poleMass", d.pendulum_mass),
        pendulum_length: number(json, "pendulumLength", d.pendulum_length),
        pivot_viscous_friction: number(json, "pivotFriction", d.pivot_viscous_friction),
        rail_stroke: number(json, "railStroke", d.rail_stroke),
        stop_stiffness: d.stop_stiffness,
        stop_damping: d.stop_damping,
    };
    let gains = [
        number(json, "k0", 0.0),
        number(json, "k1", 0.0),
        number(json, "k2", 0.0),
        number(json, "k3", 0.0),
    ];
    if gains.iter().all(|g| *g == 0.0) {
        return Err("LQR ゲイン k0..k3 が設定されていません".into());
    }
    let sd = SwingUpGains::default();
    let swing = SwingUpGains {
        energy_gain: number(json, "swingEnergyGain", sd.energy_gain),
        acceleration_amplitude: number(json, "swingAccelAmplitude", sd.acceleration_amplitude),
        position_gain: number(json, "swingPositionGain", sd.position_gain),
        velocity_gain: number(json, "swingVelocityGain", sd.velocity_gain),
        energy_offset: number(json, "swingEnergyOffset", sd.energy_offset),
        acceleration_limit: number(json, "swingAccelLimit", sd.acceleration_limit),
        voltage_limit: number(json, "swingVoltageLimit", sd.voltage_limit),
    };
    let tick_hz = number(json, "tickHz", 1000.0);
    let control_hz = number(json, "controlHz", 200.0);
    if !(tick_hz > 0.0 && control_hz > 0.0 && control_hz <= tick_hz) {
        return Err("tickHz, controlHz が不正".into());
    }
    let control_divider = (tick_hz / control_hz).round().max(1.0) as u32;
    let auto_start = match string(json, "autoStart", "none") {
        "swingup" => Mode::SwingUp,
        "balance" => Mode::Balance,
        _ => Mode::Idle,
    };
    let derived = params.derive();
    Ok(Config {
        gains,
        swing,
        bus_voltage: number(json, "busVoltage", 24.0),
        cart_meters_per_count: number(
            json,
            "cartMetersPerCount",
            2.0 * core::f64::consts::PI * 0.015 / 2000.0,
        ),
        pend_counts_per_rev: number(json, "pendCountsPerRev", 4096.0),
        control_divider,
        control_dt: control_divider as f64 / tick_hz,
        velocity_cutoff_hz: number(json, "velocityCutoffHz", 25.0),
        fall_angle: number(json, "fallAngleRad", 0.6),
        soft_limit: number(json, "softLimitM", params.rail_stroke - 0.05),
        capture_angle: number(json, "captureAngleRad", 0.3),
        capture_rate: number(json, "captureRateRadPerSec", 4.0),
        target_x: number(json, "targetX", 0.0),
        target_ramp_m_per_s: number(json, "targetRampMetersPerSec", 0.3),
        auto_start,
        auto_start_delay_us: (number(json, "autoStartDelayMs", 500.0) * 1000.0) as u64,
        params,
        derived,
    })
}

impl ip_hal::Firmware for Fw {
    fn init(cfg_json: &str) -> Result<Self, String> {
        let cfg = parse_config(cfg_json)?;
        ip_hal::log(
            LogLevel::Info,
            &format!(
                "fw-balance (rust) init: K=[{:.2}, {:.2}, {:.2}, {:.2}] control {:.0} Hz, L={:.2} m",
                cfg.gains[0], cfg.gains[1], cfg.gains[2], cfg.gains[3], 1.0 / cfg.control_dt, cfg.params.pendulum_length
            ),
        );
        ip_hal::pwm_write(0, 0.0);
        ip_hal::gpio_write(0, false);
        let cutoff = cfg.velocity_cutoff_hz;
        Ok(Self {
            cfg,
            mode: Mode::Idle,
            tick: 0,
            cart_vel: VelocityEstimator::new(cutoff),
            pend_vel: VelocityEstimator::new(cutoff),
            est_ready: false,
            x: 0.0,
            theta: 0.0,
            target_x: 0.0,
            volts: 0.0,
            heartbeat: false,
            started: false,
        })
    }

    fn tick(&mut self) {
        self.tick += 1;
        self.handle_commands();
        if !self.started
            && self.cfg.auto_start != Mode::Idle
            && ip_hal::micros() >= self.cfg.auto_start_delay_us
        {
            self.started = true;
            let m = self.cfg.auto_start;
            self.start(m);
        }
        if self.tick % self.cfg.control_divider as u64 == 0 {
            self.heartbeat = !self.heartbeat;
            ip_hal::gpio_write(1, self.heartbeat);
            self.control();
            ip_hal::trace(3, self.mode as u8 as f64);
        }
    }

    fn irq(&mut self, _id: u32) {}
}

ip_hal::firmware_entry!(Fw);
