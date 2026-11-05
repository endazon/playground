/*
 * fw-balance (C 版) — 仮想 MCU で走る倒立振子の制御ファームウェア。
 * Rust 版 (firmware/fw-balance-rs) と同じ仕様。HAL (ip_hal.h) だけに依存し、
 * 同じソースが wasm (配布) とネイティブ共有ライブラリ (デバッグ) になる。
 *
 * 周辺の割当: enc0 = 台車, enc1 = 振子, pwm0 = デューティ, gpio-out0 = ドライブ有効,
 *             gpio-out1 = ハートビート, gpio-in0 = ドライブ FAULT, uart0 = 操作 ('s' 'b' 'x' 'c')
 */
#include "ip_hal.h"
#include "cfg.h"
#include "fw_math.h"

#define PI 3.14159265358979323846
#define GRAVITY 9.81

typedef enum { MODE_IDLE = 0, MODE_SWINGUP = 1, MODE_BALANCE = 2, MODE_FAULT = 3 } mode_t_;

typedef struct {
    /* 物理パラメータ (設計モデル) */
    double cart_mass, rotor_inertia, pulley_radius, kt, ke, resistance, max_voltage;
    double cart_viscous, cart_coulomb, pole_mass, pole_length, pivot_friction, rail_stroke;
    /* 派生量 */
    double half_length, pivot_inertia, eq_cart_mass, force_gain, back_emf_damping;
    /* 制御 */
    double k[4];
    double swing_energy_gain, swing_accel_amp, swing_pos_gain, swing_vel_gain, swing_energy_offset, swing_accel_limit, swing_voltage_limit;
    double bus_voltage, cart_meters_per_count, pend_counts_per_rev;
    uint32_t control_divider;
    double control_dt, velocity_cutoff_hz, fall_angle, soft_limit, capture_angle, capture_rate, target_x, target_ramp;
    mode_t_ auto_start;
    uint64_t auto_start_delay_us;
} config_t;

typedef struct { double cutoff_hz, value, last_input; int initialized; } vel_est_t;

static config_t cfg;
static mode_t_ mode = MODE_IDLE;
static uint64_t tick_count = 0;
static vel_est_t cart_vel, pend_vel;
static int est_ready = 0;
static double est_x = 0.0, est_theta = 0.0, target_x = 0.0, volts_out = 0.0;
static int heartbeat = 0, started = 0;

static double clampd(double v, double lo, double hi) { return v < lo ? lo : v > hi ? hi : v; }
static double wrap_angle(double a) {
    double r = fmod(a + PI, 2.0 * PI);
    if (r < 0.0) r += 2.0 * PI;
    return r - PI;
}
static double signum(double x) { return x > 0.0 ? 1.0 : x < 0.0 ? -1.0 : 0.0; }

static void vel_reset(vel_est_t *e, double input) { e->last_input = input; e->value = 0.0; e->initialized = 1; }
static double vel_update(vel_est_t *e, double input, double dt) {
    if (!e->initialized) { vel_reset(e, input); return e->value; }
    if (dt > 0.0) {
        double a = 1.0 - exp(-2.0 * PI * e->cutoff_hz * dt);
        e->value += a * ((input - e->last_input) / dt - e->value);
    }
    e->last_input = input;
    return e->value;
}

static const char *mode_name(mode_t_ m) {
    switch (m) { case MODE_IDLE: return "Idle"; case MODE_SWINGUP: return "SwingUp"; case MODE_BALANCE: return "Balance"; default: return "Fault"; }
}

static void log2s(ip_log_level lvl, const char *a, const char *b, const char *c, const char *d) {
    char buf[160]; size_t n = 0;
    const char *parts[4] = { a, b, c, d };
    for (int i = 0; i < 4; i++) for (const char *p = parts[i]; p && *p && n < sizeof buf - 1; p++) buf[n++] = *p;
    buf[n] = 0;
    ip_hal_log(lvl, buf);
}

static void set_mode(mode_t_ m, const char *reason) {
    if (m == mode) return;
    log2s(m == MODE_FAULT ? IP_LOG_ERROR : IP_LOG_INFO, mode_name(mode), " -> ", mode_name(m), reason);
    mode = m;
}

static void fault(const char *reason) {
    volts_out = 0.0;
    ip_hal_pwm_write(0, 0.0f);
    ip_hal_gpio_write(0, false);
    set_mode(MODE_FAULT, reason);
}

static void start(mode_t_ target) {
    if (mode != MODE_IDLE) { log2s(IP_LOG_WARN, mode_name(mode), " 中は開始できません", 0, 0); return; }
    est_ready = 0;
    target_x = 0.0;
    volts_out = 0.0;
    ip_hal_gpio_write(0, true);
    set_mode(target, " (開始操作)");
}

static void stop(void) {
    if (mode == MODE_SWINGUP || mode == MODE_BALANCE) {
        volts_out = 0.0;
        ip_hal_pwm_write(0, 0.0f);
        ip_hal_gpio_write(0, false);
        set_mode(MODE_IDLE, " (停止操作)");
    }
}

static void handle_commands(void) {
    uint8_t buf[16];
    uint32_t n = ip_hal_uart_read(0, buf, sizeof buf);
    for (uint32_t i = 0; i < n; i++) {
        switch (buf[i]) {
            case 's': start(MODE_SWINGUP); break;
            case 'b': start(MODE_BALANCE); break;
            case 'x': stop(); break;
            case 'c': if (mode == MODE_FAULT) set_mode(MODE_IDLE, " (異常リセット)"); break;
            default: break;
        }
    }
}

static double pendulum_energy(double theta, double theta_dot) {
    return 0.5 * cfg.pivot_inertia * theta_dot * theta_dot + cfg.pole_mass * GRAVITY * cfg.half_length * (cos(theta) - 1.0);
}

static double swing_up_voltage(double x, double theta, double x_dot, double theta_dot) {
    double energy = pendulum_energy(theta, theta_dot);
    double sign = signum(theta_dot * cos(theta));
    if (sign == 0.0) sign = 1.0;
    double accel = cfg.swing_accel_amp * clampd(cfg.swing_energy_gain * (energy - cfg.swing_energy_offset), -1.0, 1.0) * sign
                   - cfg.swing_pos_gain * x - cfg.swing_vel_gain * x_dot;
    accel = clampd(accel, -cfg.swing_accel_limit, cfg.swing_accel_limit);
    double volts = ((cfg.eq_cart_mass + cfg.pole_mass) * accel
                    + (cfg.back_emf_damping + cfg.cart_viscous) * x_dot
                    + cfg.cart_coulomb * tanh(x_dot / 0.01)) / cfg.force_gain;
    return clampd(volts, -cfg.swing_voltage_limit, cfg.swing_voltage_limit);
}

static void output(double volts) {
    volts_out = volts;
    double duty = clampd(volts / cfg.bus_voltage, -1.0, 1.0);
    ip_hal_pwm_write(0, (float)duty);
    ip_hal_trace(2, volts);
}

static void control(void) {
    double x = (double)ip_hal_enc_read(0) * cfg.cart_meters_per_count;
    double theta_raw = (double)ip_hal_enc_read(1) * (2.0 * PI) / cfg.pend_counts_per_rev;
    if (!est_ready) { vel_reset(&cart_vel, x); vel_reset(&pend_vel, theta_raw); est_ready = 1; }
    else { vel_update(&cart_vel, x, cfg.control_dt); vel_update(&pend_vel, theta_raw, cfg.control_dt); }
    est_x = x;
    est_theta = wrap_angle(theta_raw);
    double x_dot = cart_vel.value, theta_dot = pend_vel.value;

    ip_hal_trace(0, est_theta);
    ip_hal_trace(1, est_x);

    if (!(mode == MODE_SWINGUP || mode == MODE_BALANCE)) return;

    if (ip_hal_gpio_read(0)) { fault(" (ドライブ FAULT 入力)"); return; }

    double volts;
    if (mode == MODE_SWINGUP) {
        if (fabs(est_theta) < cfg.capture_angle && fabs(theta_dot) < cfg.capture_rate) {
            set_mode(MODE_BALANCE, " (倒立領域に到達)");
        } else {
            if (fabs(est_x) > cfg.soft_limit) { fault(" (スイングアップ中にソフトリミット超過)"); return; }
            volts = swing_up_voltage(est_x, theta_raw, x_dot, theta_dot);
            output(volts);
            return;
        }
    }

    if (fabs(est_theta) > cfg.fall_angle) { fault(" (転倒検知)"); return; }
    if (fabs(est_x) > cfg.soft_limit) { fault(" (ソフトリミット超過)"); return; }
    double ramp = cfg.target_ramp * cfg.control_dt;
    target_x += clampd(cfg.target_x - target_x, -ramp, ramp);
    volts = -(cfg.k[0] * (est_x - target_x) + cfg.k[1] * est_theta + cfg.k[2] * x_dot + cfg.k[3] * theta_dot);
    volts = clampd(volts, -cfg.max_voltage, cfg.max_voltage);
    output(volts);
}

int32_t fw_app_init(const char *json, size_t len) {
    config_t c;
    c.cart_mass = cfg_number(json, len, "cartMass", 0.6);
    c.rotor_inertia = cfg_number(json, len, "rotorInertia", 2e-5);
    c.pulley_radius = cfg_number(json, len, "pulleyRadius", 0.015);
    c.kt = cfg_number(json, len, "kt", 0.05);
    c.ke = cfg_number(json, len, "ke", 0.05);
    c.resistance = cfg_number(json, len, "resistance", 2.0);
    c.max_voltage = cfg_number(json, len, "maxVoltage", 24.0);
    c.cart_viscous = cfg_number(json, len, "cartViscousFriction", 1.0);
    c.cart_coulomb = cfg_number(json, len, "cartCoulombFriction", 0.4);
    c.pole_mass = cfg_number(json, len, "poleMass", 0.15);
    c.pole_length = cfg_number(json, len, "pendulumLength", 0.6);
    c.pivot_friction = cfg_number(json, len, "pivotFriction", 8e-4);
    c.rail_stroke = cfg_number(json, len, "railStroke", 0.5);

    c.half_length = c.pole_length / 2.0;
    double center_inertia = c.pole_mass * c.pole_length * c.pole_length / 12.0;
    c.pivot_inertia = center_inertia + c.pole_mass * c.half_length * c.half_length;
    c.eq_cart_mass = c.cart_mass + c.rotor_inertia / (c.pulley_radius * c.pulley_radius);
    c.force_gain = c.kt / (c.resistance * c.pulley_radius);
    c.back_emf_damping = c.kt * c.ke / (c.resistance * c.pulley_radius * c.pulley_radius);

    c.k[0] = cfg_number(json, len, "k0", 0.0);
    c.k[1] = cfg_number(json, len, "k1", 0.0);
    c.k[2] = cfg_number(json, len, "k2", 0.0);
    c.k[3] = cfg_number(json, len, "k3", 0.0);
    if (c.k[0] == 0.0 && c.k[1] == 0.0 && c.k[2] == 0.0 && c.k[3] == 0.0) {
        ip_hal_log(IP_LOG_ERROR, "LQR ゲイン k0..k3 が設定されていません");
        return -1;
    }
    c.swing_energy_gain = cfg_number(json, len, "swingEnergyGain", 80.0);
    c.swing_accel_amp = cfg_number(json, len, "swingAccelAmplitude", 3.0);
    c.swing_pos_gain = cfg_number(json, len, "swingPositionGain", 5.0);
    c.swing_vel_gain = cfg_number(json, len, "swingVelocityGain", 3.0);
    c.swing_energy_offset = cfg_number(json, len, "swingEnergyOffset", 0.05);
    c.swing_accel_limit = cfg_number(json, len, "swingAccelLimit", 5.0);
    c.swing_voltage_limit = cfg_number(json, len, "swingVoltageLimit", 18.0);

    double tick_hz = cfg_number(json, len, "tickHz", 1000.0);
    double control_hz = cfg_number(json, len, "controlHz", 200.0);
    if (!(tick_hz > 0.0 && control_hz > 0.0 && control_hz <= tick_hz)) { ip_hal_log(IP_LOG_ERROR, "tickHz, controlHz が不正"); return -2; }
    double div = round(tick_hz / control_hz);
    c.control_divider = div < 1.0 ? 1u : (uint32_t)div;
    c.control_dt = (double)c.control_divider / tick_hz;
    c.bus_voltage = cfg_number(json, len, "busVoltage", 24.0);
    c.cart_meters_per_count = cfg_number(json, len, "cartMetersPerCount", 2.0 * PI * 0.015 / 2000.0);
    c.pend_counts_per_rev = cfg_number(json, len, "pendCountsPerRev", 4096.0);
    c.velocity_cutoff_hz = cfg_number(json, len, "velocityCutoffHz", 25.0);
    c.fall_angle = cfg_number(json, len, "fallAngleRad", 0.6);
    c.soft_limit = cfg_number(json, len, "softLimitM", c.rail_stroke - 0.05);
    c.capture_angle = cfg_number(json, len, "captureAngleRad", 0.3);
    c.capture_rate = cfg_number(json, len, "captureRateRadPerSec", 4.0);
    c.target_x = cfg_number(json, len, "targetX", 0.0);
    c.target_ramp = cfg_number(json, len, "targetRampMetersPerSec", 0.3);
    c.auto_start = cfg_string_eq(json, len, "autoStart", "swingup") ? MODE_SWINGUP
                 : cfg_string_eq(json, len, "autoStart", "balance") ? MODE_BALANCE : MODE_IDLE;
    c.auto_start_delay_us = (uint64_t)(cfg_number(json, len, "autoStartDelayMs", 500.0) * 1000.0);
    cfg = c;

    mode = MODE_IDLE; tick_count = 0; est_ready = 0; est_x = est_theta = target_x = volts_out = 0.0;
    heartbeat = 0; started = 0;
    cart_vel.cutoff_hz = pend_vel.cutoff_hz = c.velocity_cutoff_hz;
    cart_vel.initialized = pend_vel.initialized = 0;
    ip_hal_pwm_write(0, 0.0f);
    ip_hal_gpio_write(0, false);
    ip_hal_log(IP_LOG_INFO, "fw-balance (c) init");
    return 0;
}

void fw_app_tick(void) {
    tick_count++;
    handle_commands();
    if (!started && cfg.auto_start != MODE_IDLE && ip_hal_micros() >= cfg.auto_start_delay_us) {
        started = 1;
        start(cfg.auto_start);
    }
    if (tick_count % cfg.control_divider == 0) {
        heartbeat = !heartbeat;
        ip_hal_gpio_write(1, heartbeat != 0);
        control();
        ip_hal_trace(3, (double)mode);
    }
}

void fw_app_irq(uint32_t id) { (void)id; }
void fw_app_idle(void) {}
