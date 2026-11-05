//! ip-model の性質テスト (09-verification.md の L1)。外部の基準値に依存しない。
// 行列の添字アクセスは数式どおりに読める方を優先する (clippy --tests)
#![allow(clippy::needless_range_loop)]

use ip_model::control::{swing_up_voltage, SwingUpGains};
use ip_model::dynamics::{derivative, pendulum_energy, rk4_step, total_energy, Disturbance, State};
use ip_model::estimator::{low_pass_gain, VelocityEstimator};
use ip_model::lqr::{
    design_gains, linearize, riccati_residual, DEFAULT_INPUT_WEIGHT, DEFAULT_STATE_WEIGHTS,
};
use ip_model::math;
use ip_model::matrix::{self, zero_order_hold};
use ip_model::Params;
use proptest::prelude::*;

const H: f64 = 0.001;

fn simulate(p: &Params, s0: State, volts: f64, steps: usize, h: f64) -> State {
    let d = p.derive();
    let mut s = s0;
    for _ in 0..steps {
        s = rk4_step(p, &d, &s, volts, &Disturbance::NONE, h);
    }
    s
}

#[test]
fn energy_is_conserved_without_friction_and_input() {
    // 摩擦 0・電圧 0 で 10 秒。ストッパに当たらない範囲で大きく振る。
    let p = Params::default().frictionless();
    let d = p.derive();
    let s0 = State::new(0.0, 2.5, 0.0, 0.0);
    let e0 = total_energy(&p, &d, &s0);
    let mut s = s0;
    let mut worst: f64 = 0.0;
    for _ in 0..10_000 {
        s = rk4_step(&p, &d, &s, 0.0, &Disturbance::NONE, H);
        assert!(
            s.x.abs() < p.rail_stroke,
            "ストッパに当たると保存しない: x={}",
            s.x
        );
        worst = worst.max((total_energy(&p, &d, &s) - e0).abs());
    }
    let scale = p.pendulum_mass * ip_model::params::GRAVITY * d.half_length * 2.0;
    assert!(
        worst / scale < 1e-6,
        "エネルギー変動 {worst:e} J (相対 {:e})",
        worst / scale
    );
}

#[test]
fn equilibria_are_stationary() {
    let p = Params::default();
    for theta in [0.0, core::f64::consts::PI] {
        let s0 = State::new(0.0, theta, 0.0, 0.0);
        let s = simulate(&p, s0, 0.0, 5_000, H);
        assert!(
            (s.x - s0.x).abs() < 1e-9 && (s.theta - s0.theta).abs() < 1e-9,
            "θ0={theta}: {s:?}"
        );
    }
}

#[test]
fn dynamics_are_mirror_symmetric() {
    // (x, θ, u) → (−x, −θ, −u) で軌道が鏡像になる
    let p = Params::default();
    let s0 = State::new(0.1, 0.3, 0.2, -0.5);
    let m0 = State::new(-0.1, -0.3, -0.2, 0.5);
    let a = simulate(&p, s0, 3.0, 2_000, H);
    let b = simulate(&p, m0, -3.0, 2_000, H);
    assert!((a.x + b.x).abs() < 1e-9 && (a.theta + b.theta).abs() < 1e-9);
    assert!((a.x_dot + b.x_dot).abs() < 1e-9 && (a.theta_dot + b.theta_dot).abs() < 1e-9);
}

#[test]
fn rk4_has_fourth_order_convergence() {
    // 刻みを半分にすると、基準解 (1/8 刻み) に対する誤差が約 1/16 になる
    let p = Params::default().frictionless();
    let s0 = State::new(0.0, 2.0, 0.0, 0.0);
    let t = 1.0;
    let reference = simulate(&p, s0, 2.0, (t / (H / 8.0)) as usize, H / 8.0);
    let coarse = simulate(&p, s0, 2.0, (t / (2.0 * H)) as usize, 2.0 * H);
    let fine = simulate(&p, s0, 2.0, (t / H) as usize, H);
    let err = |s: State| (s.theta - reference.theta).abs() + (s.x - reference.x).abs();
    let ratio = err(coarse) / err(fine);
    assert!(
        ratio > 10.0 && ratio < 24.0,
        "誤差比 {ratio} (4 次なら ≈16)"
    );
}

#[test]
fn linearization_matches_nonlinear_model_near_upright() {
    // 微小振幅では非線形微分と線形化 A x + B u の差が振幅の 2 乗オーダー
    let p = Params::default();
    let d = p.derive();
    let lin = linearize(&p);
    let mut prev: Option<(f64, f64)> = None;
    for eps in [1e-2, 5e-3, 2.5e-3] {
        let s = State::new(0.0, eps, 0.0, eps);
        let u = 0.0;
        let nl = derivative(&p, &d, &s, u, &Disturbance::NONE).to_array();
        let xs = s.to_array();
        let mut li = matrix::mul_vec(&lin.a, &xs);
        for (li_i, b_i) in li.iter_mut().zip(lin.b.iter()) {
            *li_i += b_i * u;
        }
        let mut err: f64 = 0.0;
        for i in 0..4 {
            err = err.max((nl[i] - li[i]).abs());
        }
        if let Some((pe, pr)) = prev {
            let ratio = pe / err;
            assert!(ratio > 3.0, "eps {pr}→{eps}: 誤差比 {ratio} (2 次なら ≈4)");
        }
        prev = Some((err, eps));
    }
}

#[test]
fn zero_order_hold_is_consistent() {
    let p = Params::default();
    let lin = linearize(&p);
    // 微小 Δt では Ad ≈ I + AΔt (残差は 2 次項 O(‖A‖²Δt²))
    let dt = 1e-5;
    let mut a_norm: f64 = 0.0;
    for row in lin.a {
        a_norm = a_norm.max(row.iter().map(|v| v.abs()).sum());
    }
    let tol = a_norm * a_norm * dt * dt;
    let (ad, bd) = zero_order_hold(&lin.a, &lin.b, dt);
    for i in 0..4 {
        for j in 0..4 {
            let expect = if i == j { 1.0 } else { 0.0 } + lin.a[i][j] * dt;
            assert!(
                (ad[i][j] - expect).abs() < tol,
                "ad[{i}][{j}]={} expect {expect}",
                ad[i][j]
            );
        }
        assert!((bd[i] - lin.b[i] * dt).abs() < tol);
    }
    // 半群性: Ad(2Δt) = Ad(Δt)^2
    let (a1, _) = zero_order_hold(&lin.a, &lin.b, 0.005);
    let (a2, _) = zero_order_hold(&lin.a, &lin.b, 0.010);
    let a1sq = matrix::mul(&a1, &a1);
    for i in 0..4 {
        for j in 0..4 {
            assert!((a1sq[i][j] - a2[i][j]).abs() < 1e-10);
        }
    }
}

#[test]
fn lqr_solution_satisfies_riccati_and_stabilizes() {
    let p = Params::default();
    let (sol, _lin, ad, bd) =
        design_gains(&p, 0.005, &DEFAULT_STATE_WEIGHTS, DEFAULT_INPUT_WEIGHT).expect("LQR");
    let residual = riccati_residual(
        &ad,
        &bd,
        &DEFAULT_STATE_WEIGHTS,
        DEFAULT_INPUT_WEIGHT,
        &sol.p,
    );
    let mut pmax: f64 = 0.0;
    for row in sol.p {
        for v in row {
            pmax = pmax.max(v.abs());
        }
    }
    assert!(
        residual / pmax < 1e-9,
        "リカッチ残差 {residual:e} (相対 {:e})",
        residual / pmax
    );

    // 閉ループ (A - BK) x が 0 へ減衰する (単位円内の固有値)
    let mut x = [0.0, 0.05, 0.0, 0.0];
    let norm0: f64 = x.iter().map(|v| v * v).sum::<f64>().sqrt();
    for _ in 0..2_000 {
        let u = -(sol.k[0] * x[0] + sol.k[1] * x[1] + sol.k[2] * x[2] + sol.k[3] * x[3]);
        let mut nx = matrix::mul_vec(&ad, &x);
        for i in 0..4 {
            nx[i] += bd[i] * u;
        }
        x = nx;
        assert!(x.iter().all(|v| v.is_finite() && v.abs() < 1e3));
    }
    let norm1: f64 = x.iter().map(|v| v * v).sum::<f64>().sqrt();
    assert!(
        norm1 < norm0 * 1e-3,
        "閉ループが減衰しない: {norm0} → {norm1}"
    );
}

#[test]
fn closed_loop_balances_nonlinear_plant_from_small_offset() {
    // 離散 LQR を 5ms 周期で非線形プラントに適用し、倒立を維持する
    let p = Params::default();
    let d = p.derive();
    let (sol, ..) = design_gains(&p, 0.005, &DEFAULT_STATE_WEIGHTS, DEFAULT_INPUT_WEIGHT).unwrap();
    // クーロン摩擦があるので原点には収束せず微小なリミットサイクルに入る。倒立の維持だけを見る。
    let mut s = State::near_upright(0.1);
    let mut u = 0.0;
    let mut worst_theta: f64 = 0.0;
    let mut worst_x: f64 = 0.0;
    for step in 0..8_000 {
        if step % 5 == 0 {
            u = math::clamp(
                ip_model::control::lqr_voltage(
                    &sol.k,
                    s.x,
                    math::wrap_angle(s.theta),
                    s.x_dot,
                    s.theta_dot,
                ),
                -p.max_voltage,
                p.max_voltage,
            );
        }
        s = rk4_step(&p, &d, &s, u, &Disturbance::NONE, H);
        if step >= 6_000 {
            worst_theta = worst_theta.max(s.theta.abs());
            worst_x = worst_x.max(s.x.abs());
        }
    }
    assert!(
        worst_theta < 0.02 && worst_x < 0.1,
        "θmax={worst_theta} xmax={worst_x} {s:?}"
    );
}

#[test]
fn swing_up_pumps_energy_toward_target() {
    // 吊り下げ近傍から、エネルギーが単調とは限らないが目標まで上がる
    let p = Params::default();
    let d = p.derive();
    let g = SwingUpGains::default();
    let mut s = State::new(0.0, core::f64::consts::PI - 0.05, 0.0, 0.0);
    let e0 = pendulum_energy(&p, &d, &s);
    let mut reached = false;
    for _ in 0..15_000 {
        let u = swing_up_voltage(&p, &d, &s, &g);
        s = rk4_step(&p, &d, &s, u, &Disturbance::NONE, H);
        if pendulum_energy(&p, &d, &s) >= 0.0 {
            reached = true;
            break;
        }
    }
    assert!(reached, "15 s 以内に目標エネルギーに達しない (E0={e0})");
}

#[test]
fn velocity_estimator_tracks_constant_velocity() {
    let mut est = VelocityEstimator::new(25.0);
    let dt = 0.005;
    let mut x = 0.0;
    let mut v = 0.0;
    for _ in 0..400 {
        x += 0.3 * dt;
        v = est.update(x, dt);
    }
    assert!((v - 0.3).abs() < 1e-6, "{v}");
    assert!(low_pass_gain(25.0, dt) > 0.0 && low_pass_gain(25.0, dt) < 1.0);
}

proptest! {
    #[test]
    fn wrap_angle_is_in_range_and_periodic(a in -100.0f64..100.0) {
        let w = math::wrap_angle(a);
        prop_assert!((-core::f64::consts::PI..core::f64::consts::PI).contains(&w));
        let w2 = math::wrap_angle(a + 2.0 * core::f64::consts::PI);
        prop_assert!((w - w2).abs() < 1e-9);
    }

    #[test]
    fn derivative_is_finite_for_bounded_states(
        x in -0.6f64..0.6, th in -10.0f64..10.0, xd in -5.0f64..5.0, thd in -30.0f64..30.0, u in -24.0f64..24.0)
    {
        let p = Params::default();
        let d = p.derive();
        let ds = derivative(&p, &d, &State::new(x, th, xd, thd), u, &Disturbance::NONE);
        prop_assert!(ds.to_array().iter().all(|v| v.is_finite()));
    }
}
