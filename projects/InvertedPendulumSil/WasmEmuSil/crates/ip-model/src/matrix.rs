//! 固定次元 (4 状態) の最小限の密行列演算と ZOH 離散化。

use crate::math;

pub const N: usize = 4;
pub type Mat4 = [[f64; N]; N];
pub type Vec4 = [f64; N];

pub fn identity() -> Mat4 {
    let mut m = [[0.0; N]; N];
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    m
}

pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut r = [[0.0; N]; N];
    for i in 0..N {
        for p in 0..N {
            let aip = a[i][p];
            if aip == 0.0 {
                continue;
            }
            for j in 0..N {
                r[i][j] += aip * b[p][j];
            }
        }
    }
    r
}

pub fn mul_vec(a: &Mat4, x: &Vec4) -> Vec4 {
    let mut r = [0.0; N];
    for i in 0..N {
        let mut s = 0.0;
        for j in 0..N {
            s += a[i][j] * x[j];
        }
        r[i] = s;
    }
    r
}

/// 一般サイズの行列 (5x5 まで) を使う行列指数。Van Loan のブロック行列用。
const NB: usize = 5;
type MatB = [[f64; NB]; NB];

fn mul_b(a: &MatB, b: &MatB, n: usize) -> MatB {
    let mut r = [[0.0; NB]; NB];
    for i in 0..n {
        for p in 0..n {
            let aip = a[i][p];
            if aip == 0.0 {
                continue;
            }
            for j in 0..n {
                r[i][j] += aip * b[p][j];
            }
        }
    }
    r
}

fn infinity_norm_b(a: &MatB, n: usize) -> f64 {
    let mut best = 0.0;
    for row in a.iter().take(n) {
        let mut s = 0.0;
        for v in row.iter().take(n) {
            s += math::fabs(*v);
        }
        if s > best {
            best = s;
        }
    }
    best
}

/// 行列指数 exp(M)。scaling & squaring + 14 次テイラー (C# 版と同じ)。
fn exp_b(m: &MatB, n: usize) -> MatB {
    let norm = infinity_norm_b(m, n);
    let squarings = {
        let l = math::ceil(math::log2(if norm <= 0.0 { 1.0 } else { norm })) as i32 + 2;
        if l < 0 {
            0
        } else {
            l as u32
        }
    };
    let scale = math::pow(2.0, squarings as f64);
    let mut s = [[0.0; NB]; NB];
    for i in 0..n {
        for j in 0..n {
            s[i][j] = m[i][j] / scale;
        }
    }
    let mut result = [[0.0; NB]; NB];
    let mut term = [[0.0; NB]; NB];
    for i in 0..n {
        result[i][i] = 1.0;
        term[i][i] = 1.0;
    }
    for k in 1..=14 {
        term = mul_b(&term, &s, n);
        for i in 0..n {
            for j in 0..n {
                term[i][j] /= k as f64;
                result[i][j] += term[i][j];
            }
        }
    }
    for _ in 0..squarings {
        result = mul_b(&result, &result, n);
    }
    result
}

/// 連続時間 (A, B) を零次ホールドで離散化した (Ad, Bd)。Van Loan 法。
pub fn zero_order_hold(a: &Mat4, b: &Vec4, ts: f64) -> (Mat4, Vec4) {
    let mut block = [[0.0; NB]; NB];
    for i in 0..N {
        for j in 0..N {
            block[i][j] = a[i][j] * ts;
        }
        block[i][N] = b[i] * ts;
    }
    let e = exp_b(&block, NB);
    let mut ad = [[0.0; N]; N];
    let mut bd = [0.0; N];
    for i in 0..N {
        for j in 0..N {
            ad[i][j] = e[i][j];
        }
        bd[i] = e[i][N];
    }
    (ad, bd)
}
