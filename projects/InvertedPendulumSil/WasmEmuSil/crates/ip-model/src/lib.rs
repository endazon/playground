//! 台車型倒立振子の物理モデルと制御設計。
//!
//! NetworkedSil の `Ip.Shared` (C#) からの移植。部品 (wasm) とテスト (native) の両方から
//! 同じコードを使うため `no_std` で書き、浮動小数点関数は `libm` に統一して
//! ターゲット間でビット一致しやすくしている。
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

pub mod control;
pub mod dynamics;
pub mod estimator;
pub mod lqr;
pub mod matrix;
pub mod params;

pub use dynamics::{Disturbance, State};
pub use params::{Derived, Params};

/// 共通の数学関数。`libm` を使うのは、std の実装がプラットフォームの libm に依存して
/// 最終ビットがずれるのを避けるため (決定性テストの前提)。
pub mod math {
    #[inline]
    pub fn sin(x: f64) -> f64 {
        libm::sin(x)
    }
    #[inline]
    pub fn cos(x: f64) -> f64 {
        libm::cos(x)
    }
    #[inline]
    pub fn tanh(x: f64) -> f64 {
        libm::tanh(x)
    }
    #[inline]
    pub fn exp(x: f64) -> f64 {
        libm::exp(x)
    }
    #[inline]
    pub fn sqrt(x: f64) -> f64 {
        libm::sqrt(x)
    }
    #[inline]
    pub fn round(x: f64) -> f64 {
        libm::round(x)
    }
    #[inline]
    pub fn fabs(x: f64) -> f64 {
        libm::fabs(x)
    }
    #[inline]
    pub fn log2(x: f64) -> f64 {
        libm::log2(x)
    }
    #[inline]
    pub fn ceil(x: f64) -> f64 {
        libm::ceil(x)
    }
    #[inline]
    pub fn pow(x: f64, y: f64) -> f64 {
        libm::pow(x, y)
    }
    #[inline]
    pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
        if v < lo {
            lo
        } else if v > hi {
            hi
        } else {
            v
        }
    }
    /// 角度を [-π, π) に正規化する (π は -π に写る)。
    pub fn wrap_angle(a: f64) -> f64 {
        let two_pi = 2.0 * core::f64::consts::PI;
        let mut r = (a + core::f64::consts::PI) % two_pi;
        if r < 0.0 {
            r += two_pi;
        }
        r - core::f64::consts::PI
    }
    #[inline]
    pub fn signum(x: f64) -> f64 {
        if x > 0.0 {
            1.0
        } else if x < 0.0 {
            -1.0
        } else {
            0.0
        }
    }
}
