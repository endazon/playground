//! ファームウェア (Rust) の HAL。`abi/ip_hal.h` と 1 対 1。
//!
//! - wasm32: `ip:emu/hal` の import を呼ぶ (wit-bindgen)
//! - native: `ip_fw_init` で受け取った `ip_hal_vtable` に委譲する
//!
//! ファームウェアは [`Firmware`] を実装し、`ip_hal::firmware_entry!(MyFw)` で入口を生やす。
//! どちらの形でも同じソースが動く。
#![cfg_attr(target_arch = "wasm32", allow(static_mut_refs))]

pub const ABI_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

/// ファームウェアが実装する契約。
pub trait Firmware: Sized {
    /// manifest の firmware.params (JSON)
    fn init(cfg_json: &str) -> Result<Self, String>;
    fn tick(&mut self);
    fn irq(&mut self, _id: u32) {}
    fn idle(&mut self) {}
}

// ------------------------------------------------------------------------------------------
// wasm 形
// ------------------------------------------------------------------------------------------
#[cfg(target_arch = "wasm32")]
pub mod bindings {
    wit_bindgen::generate!({
        path: "../../abi/wit",
        world: "firmware",
        pub_export_macro: true,
        default_bindings_module: "ip_hal::bindings",
    });
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::bindings::ip::emu::hal;
    use super::LogLevel;

    fn lvl(l: LogLevel) -> super::bindings::ip::emu::types::LogLevel {
        use super::bindings::ip::emu::types::LogLevel as L;
        match l {
            LogLevel::Trace => L::Trace,
            LogLevel::Debug => L::Debug,
            LogLevel::Info => L::Info,
            LogLevel::Warn => L::Warn,
            LogLevel::Error => L::Error,
        }
    }
    pub fn micros() -> u64 {
        hal::micros()
    }
    pub fn enc_read(ch: u32) -> i32 {
        hal::enc_read(ch)
    }
    pub fn pwm_write(ch: u32, duty: f32) {
        hal::pwm_write(ch, duty)
    }
    pub fn dac_write(ch: u32, v: f32) {
        hal::dac_write(ch, v)
    }
    pub fn gpio_read(pin: u32) -> bool {
        hal::gpio_read(pin)
    }
    pub fn gpio_write(pin: u32, level: bool) {
        hal::gpio_write(pin, level)
    }
    pub fn uart_write(ch: u32, data: &[u8]) -> u32 {
        hal::uart_write(ch, data)
    }
    pub fn uart_read(ch: u32, max: u32) -> Vec<u8> {
        hal::uart_read(ch, max)
    }
    pub fn log(level: LogLevel, msg: &str) {
        hal::log(lvl(level), msg)
    }
    pub fn trace(id: u32, value: f64) {
        hal::trace(id, value)
    }
}

// ------------------------------------------------------------------------------------------
// native 形
// ------------------------------------------------------------------------------------------
#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    use super::LogLevel;
    use core::ffi::c_void;
    use core::sync::atomic::{AtomicPtr, Ordering};

    /// `abi/ip_hal.h` の `ip_hal_vtable` と同じレイアウト。
    #[repr(C)]
    pub struct IpHalVtable {
        pub size: u32,
        pub micros: Option<unsafe extern "C" fn(*mut c_void) -> u64>,
        pub enc_read: Option<unsafe extern "C" fn(*mut c_void, u32) -> i32>,
        pub pwm_write: Option<unsafe extern "C" fn(*mut c_void, u32, f32)>,
        pub dac_write: Option<unsafe extern "C" fn(*mut c_void, u32, f32)>,
        pub gpio_read: Option<unsafe extern "C" fn(*mut c_void, u32) -> bool>,
        pub gpio_write: Option<unsafe extern "C" fn(*mut c_void, u32, bool)>,
        pub uart_write: Option<unsafe extern "C" fn(*mut c_void, u32, *const u8, u32) -> u32>,
        pub uart_read: Option<unsafe extern "C" fn(*mut c_void, u32, *mut u8, u32) -> u32>,
        pub log: Option<unsafe extern "C" fn(*mut c_void, u32, *const core::ffi::c_char)>,
        pub trace: Option<unsafe extern "C" fn(*mut c_void, u32, f64)>,
        pub ctx: *mut c_void,
    }

    static VTABLE: AtomicPtr<IpHalVtable> = AtomicPtr::new(core::ptr::null_mut());

    /// `ip_fw_init` が呼ぶ。以後の HAL 呼び出しはこのテーブルへ委譲する。
    ///
    /// # Safety
    /// `vt` はプロセスの寿命の間 (少なくとも次の init まで) 有効であること。
    pub unsafe fn bind(vt: *const IpHalVtable) {
        VTABLE.store(vt as *mut _, Ordering::SeqCst);
    }

    fn vt() -> Option<&'static IpHalVtable> {
        let p = VTABLE.load(Ordering::SeqCst);
        if p.is_null() {
            None
        } else {
            // SAFETY: bind の契約により有効
            Some(unsafe { &*p })
        }
    }

    pub fn micros() -> u64 {
        vt().and_then(|v| v.micros.map(|f| unsafe { f(v.ctx) }))
            .unwrap_or(0)
    }
    pub fn enc_read(ch: u32) -> i32 {
        vt().and_then(|v| v.enc_read.map(|f| unsafe { f(v.ctx, ch) }))
            .unwrap_or(0)
    }
    pub fn pwm_write(ch: u32, duty: f32) {
        if let Some(v) = vt() {
            if let Some(f) = v.pwm_write {
                unsafe { f(v.ctx, ch, duty) }
            }
        }
    }
    pub fn dac_write(ch: u32, value: f32) {
        if let Some(v) = vt() {
            if let Some(f) = v.dac_write {
                unsafe { f(v.ctx, ch, value) }
            }
        }
    }
    pub fn gpio_read(pin: u32) -> bool {
        vt().and_then(|v| v.gpio_read.map(|f| unsafe { f(v.ctx, pin) }))
            .unwrap_or(false)
    }
    pub fn gpio_write(pin: u32, level: bool) {
        if let Some(v) = vt() {
            if let Some(f) = v.gpio_write {
                unsafe { f(v.ctx, pin, level) }
            }
        }
    }
    pub fn uart_write(ch: u32, data: &[u8]) -> u32 {
        vt().and_then(|v| {
            v.uart_write
                .map(|f| unsafe { f(v.ctx, ch, data.as_ptr(), data.len() as u32) })
        })
        .unwrap_or(0)
    }
    pub fn uart_read(ch: u32, max: u32) -> Vec<u8> {
        let mut buf = vec![0u8; max as usize];
        let n = vt()
            .and_then(|v| {
                v.uart_read
                    .map(|f| unsafe { f(v.ctx, ch, buf.as_mut_ptr(), max) })
            })
            .unwrap_or(0);
        buf.truncate(n as usize);
        buf
    }
    pub fn log(level: LogLevel, msg: &str) {
        if let Some(v) = vt() {
            if let Some(f) = v.log {
                let c = std::ffi::CString::new(msg.replace('\0', " ")).unwrap_or_default();
                unsafe { f(v.ctx, level as u32, c.as_ptr()) }
            }
        }
    }
    pub fn trace(id: u32, value: f64) {
        if let Some(v) = vt() {
            if let Some(f) = v.trace {
                unsafe { f(v.ctx, id, value) }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
use native as imp;

// 公開 API (両形で同じ)
pub fn micros() -> u64 {
    imp::micros()
}
pub fn enc_read(ch: u32) -> i32 {
    imp::enc_read(ch)
}
pub fn pwm_write(ch: u32, duty: f32) {
    imp::pwm_write(ch, duty)
}
pub fn dac_write(ch: u32, value: f32) {
    imp::dac_write(ch, value)
}
pub fn gpio_read(pin: u32) -> bool {
    imp::gpio_read(pin)
}
pub fn gpio_write(pin: u32, level: bool) {
    imp::gpio_write(pin, level)
}
pub fn uart_write(ch: u32, data: &[u8]) -> u32 {
    imp::uart_write(ch, data)
}
pub fn uart_read(ch: u32, max: u32) -> Vec<u8> {
    imp::uart_read(ch, max)
}
pub fn log(level: LogLevel, msg: &str) {
    imp::log(level, msg)
}
pub fn trace(id: u32, value: f64) {
    imp::trace(id, value)
}

/// ファームウェアの入口を生成する。wasm では WIT の export、native では C シンボル。
#[macro_export]
macro_rules! firmware_entry {
    ($ty:ty) => {
        mod __ip_fw_entry {
            use super::*;
            use $crate::Firmware as _;

            static mut INSTANCE: Option<$ty> = None;

            #[allow(static_mut_refs)]
            fn instance() -> Option<&'static mut $ty> {
                unsafe { INSTANCE.as_mut() }
            }

            #[allow(static_mut_refs)]
            pub fn do_init(cfg: &str) -> Result<(), String> {
                let fw = <$ty as $crate::Firmware>::init(cfg)?;
                unsafe {
                    INSTANCE = Some(fw);
                }
                Ok(())
            }
            pub fn do_tick() {
                if let Some(i) = instance() {
                    i.tick();
                }
            }
            pub fn do_irq(id: u32) {
                if let Some(i) = instance() {
                    i.irq(id);
                }
            }
            pub fn do_idle() {
                if let Some(i) = instance() {
                    i.idle();
                }
            }

            #[cfg(target_arch = "wasm32")]
            mod wasm {
                pub struct Entry;
                impl $crate::bindings::exports::ip::emu::fw::Guest for Entry {
                    fn abi_version() -> u32 {
                        $crate::ABI_VERSION
                    }
                    fn init(cfg_json: String) -> Result<(), String> {
                        super::do_init(&cfg_json)
                    }
                    fn tick() {
                        super::do_tick()
                    }
                    fn irq(id: u32) {
                        super::do_irq(id)
                    }
                    fn idle() {
                        super::do_idle()
                    }
                }
                $crate::bindings::export!(Entry with_types_in $crate::bindings);
            }

            #[cfg(not(target_arch = "wasm32"))]
            mod native {
                use core::ffi::c_char;

                #[no_mangle]
                pub extern "C" fn ip_fw_abi_version() -> u32 {
                    $crate::ABI_VERSION
                }
                /// # Safety
                /// `hal` は有効な vtable、`cfg_json` は `cfg_len` バイト読めること。
                #[no_mangle]
                pub unsafe extern "C" fn ip_fw_init(hal: *const $crate::native::IpHalVtable, cfg_json: *const c_char, cfg_len: usize) -> i32 {
                    if hal.is_null() {
                        return -1;
                    }
                    $crate::native::bind(hal);
                    let cfg = if cfg_json.is_null() {
                        ""
                    } else {
                        let bytes = core::slice::from_raw_parts(cfg_json as *const u8, cfg_len);
                        match core::str::from_utf8(bytes) {
                            Ok(s) => s,
                            Err(_) => return -2,
                        }
                    };
                    match super::do_init(cfg) {
                        Ok(()) => 0,
                        Err(e) => {
                            $crate::log($crate::LogLevel::Error, &format!("init 失敗: {e}"));
                            -3
                        }
                    }
                }
                #[no_mangle]
                pub extern "C" fn ip_fw_tick() {
                    super::do_tick()
                }
                #[no_mangle]
                pub extern "C" fn ip_fw_irq(id: u32) {
                    super::do_irq(id)
                }
                #[no_mangle]
                pub extern "C" fn ip_fw_idle() {
                    super::do_idle()
                }
            }
        }
    };
}
