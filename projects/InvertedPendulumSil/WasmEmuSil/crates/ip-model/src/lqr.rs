//! 倒立点まわりの線形化と離散時間 LQR。

use crate::matrix::{self, Mat4, Vec4, N};
use crate::params::{Params, GRAVITY};

/// 倒立点 (θ=0) まわりで線形化した連続時間モデル。
#[derive(Debug, Clone, Copy)]
pub struct Linearized {
    pub a: Mat4,
    pub b: Vec4,
    /// 倒立点の不安定極 λ [rad/s]
    pub unstable_pole: f64,
}

pub fn linearize(p: &Params) -> Linearized {
    let d = p.derive();
    let a11 = d.equivalent_cart_mass + p.pendulum_mass;
    let a12 = p.pendulum_mass * d.half_length;
    let det = a11 * d.pivot_inertia - a12 * a12;
    let mgl = p.pendulum_mass * GRAVITY * d.half_length;
    let bx = d.back_emf_damping + p.cart_viscous_friction;

    let mut a = [[0.0; N]; N];
    a[0][2] = 1.0;
    a[1][3] = 1.0;
    a[2][1] = -a12 * mgl / det;
    a[2][2] = -d.pivot_inertia * bx / det;
    a[2][3] = a12 * p.pivot_viscous_friction / det;
    a[3][1] = a11 * mgl / det;
    a[3][2] = a12 * bx / det;
    a[3][3] = -a11 * p.pivot_viscous_friction / det;

    let mut b = [0.0; N];
    b[2] = d.pivot_inertia * d.force_gain / det;
    b[3] = -a12 * d.force_gain / det;

    Linearized {
        a,
        b,
        unstable_pole: crate::math::sqrt(a11 * mgl / det),
    }
}

pub const DEFAULT_MAX_ITERATIONS: usize = 40_000;
const RELATIVE_TOLERANCE: f64 = 1e-11;

/// 離散時間 LQR の結果。
#[derive(Debug, Clone, Copy)]
pub struct LqrSolution {
    /// 状態フィードバックゲイン K (u = -K x)
    pub k: Vec4,
    /// リカッチ方程式の解 P
    pub p: Mat4,
    pub iterations: usize,
}

/// 単入力系の離散 LQR。Joseph 形式 + 対称化のリカッチ反復 (C# 版と同じ)。
pub fn solve_dlqr(
    ad: &Mat4,
    bd: &Vec4,
    q_diag: &Vec4,
    r: f64,
    max_iterations: usize,
) -> Option<LqrSolution> {
    if r.is_nan() || r <= 0.0 {
        return None;
    }
    let mut p = [[0.0; N]; N];
    for i in 0..N {
        p[i][i] = q_diag[i];
    }
    let mut k = [0.0; N];
    let mut iterations = 0;

    for it in 0..max_iterations {
        iterations = it + 1;
        let pb = matrix::mul_vec(&p, bd);
        let mut bpb = 0.0;
        for i in 0..N {
            bpb += bd[i] * pb[i];
        }
        for j in 0..N {
            let mut s = 0.0;
            for i in 0..N {
                s += pb[i] * ad[i][j];
            }
            k[j] = s / (r + bpb);
        }

        let mut acl = [[0.0; N]; N];
        for i in 0..N {
            for j in 0..N {
                acl[i][j] = ad[i][j] - bd[i] * k[j];
            }
        }

        let p_acl = matrix::mul(&p, &acl);
        let mut diff: f64 = 0.0;
        let mut magnitude: f64 = 0.0;
        let mut next = [[0.0; N]; N];
        for i in 0..N {
            for j in i..N {
                let mut s = 0.0;
                for m in 0..N {
                    s += acl[m][i] * p_acl[m][j];
                }
                let v = (if i == j { q_diag[i] } else { 0.0 }) + r * k[i] * k[j] + s;
                next[i][j] = v;
                next[j][i] = v;
                diff = diff.max(crate::math::fabs(v - p[i][j]));
                magnitude = magnitude.max(crate::math::fabs(v));
            }
        }
        p = next;
        if diff <= RELATIVE_TOLERANCE * magnitude {
            break;
        }
    }
    Some(LqrSolution { k, p, iterations })
}

/// 離散リカッチ方程式の残差 ||P - (Q + A'PA - A'PB (R + B'PB)^-1 B'PA)||_max。
/// 解の検証用 (性質テスト)。
pub fn riccati_residual(ad: &Mat4, bd: &Vec4, q_diag: &Vec4, r: f64, p: &Mat4) -> f64 {
    let pb = matrix::mul_vec(p, bd);
    let mut bpb = 0.0;
    for i in 0..N {
        bpb += bd[i] * pb[i];
    }
    // A'PA
    let pa = matrix::mul(p, ad);
    let mut apa = [[0.0; N]; N];
    for i in 0..N {
        for j in 0..N {
            let mut s = 0.0;
            for m in 0..N {
                s += ad[m][i] * pa[m][j];
            }
            apa[i][j] = s;
        }
    }
    // A'PB (R+B'PB)^-1 B'PA  = (A'Pb)(A'Pb)' / (r + bpb)
    let mut apb = [0.0; N];
    for i in 0..N {
        let mut s = 0.0;
        for m in 0..N {
            s += ad[m][i] * pb[m];
        }
        apb[i] = s;
    }
    let mut worst: f64 = 0.0;
    for i in 0..N {
        for j in 0..N {
            let q = if i == j { q_diag[i] } else { 0.0 };
            let rhs = q + apa[i][j] - apb[i] * apb[j] / (r + bpb);
            worst = worst.max(crate::math::fabs(p[i][j] - rhs));
        }
    }
    worst
}

/// 既定の重み (NetworkedSil の ControllerCore と同じ)。
pub const DEFAULT_STATE_WEIGHTS: Vec4 = [40.0, 60.0, 1.0, 0.5];
pub const DEFAULT_INPUT_WEIGHT: f64 = 0.6;

/// パラメータと制御周期から離散 LQR ゲインを設計する (ホストのツールとテストが使う)。
pub fn design_gains(
    p: &Params,
    ts: f64,
    q_diag: &Vec4,
    r: f64,
) -> Option<(LqrSolution, Linearized, Mat4, Vec4)> {
    let lin = linearize(p);
    let (ad, bd) = matrix::zero_order_hold(&lin.a, &lin.b, ts);
    let sol = solve_dlqr(&ad, &bd, q_diag, r, DEFAULT_MAX_ITERATIONS)?;
    Some((sol, lin, ad, bd))
}
