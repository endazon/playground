//! C ファームウェアを freestanding (libc なし) で wasm32 にビルドするときの数学関数。
//! `libm` crate を C ABI で公開する。Rust 版ファームウェア (ip-model 経由で同じ libm) と結果が一致する。
#![cfg_attr(target_arch = "wasm32", no_std)]
#![allow(clippy::missing_safety_doc)]

#[cfg(all(target_arch = "wasm32", not(test)))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

macro_rules! export1 {
    ($($name:ident),* $(,)?) => {$(
        #[no_mangle]
        pub extern "C" fn $name(x: f64) -> f64 { libm::$name(x) }
    )*};
}
export1!(sin, cos, tan, tanh, exp, log, log2, sqrt, round, floor, ceil, fabs, trunc);

#[no_mangle]
pub extern "C" fn fmod(x: f64, y: f64) -> f64 {
    libm::fmod(x, y)
}
#[no_mangle]
pub extern "C" fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}
#[no_mangle]
pub extern "C" fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}
#[no_mangle]
pub extern "C" fn fmin(x: f64, y: f64) -> f64 {
    libm::fmin(x, y)
}
#[no_mangle]
pub extern "C" fn fmax(x: f64, y: f64) -> f64 {
    libm::fmax(x, y)
}
