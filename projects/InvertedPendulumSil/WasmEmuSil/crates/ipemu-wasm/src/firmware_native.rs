//! ネイティブ形ファームウェア (.so / .dll / .dylib)。`abi/ip_hal.h` の vtable で HAL を渡す。
//!
//! デバッグ専用: サンドボックスも決定性も保証しない。Visual Studio / LLDB でブレークするための形。

use anyhow::Context;
use ipemu_core::hal::{Firmware, HalRegs, LogLevel, TickStats};
use libloading::{Library, Symbol};
use std::cell::Cell;
use std::ffi::{c_char, c_void, CStr};
use std::path::Path;

#[repr(C)]
pub struct IpHalVtable {
    size: u32,
    micros: unsafe extern "C" fn(*mut c_void) -> u64,
    enc_read: unsafe extern "C" fn(*mut c_void, u32) -> i32,
    pwm_write: unsafe extern "C" fn(*mut c_void, u32, f32),
    dac_write: unsafe extern "C" fn(*mut c_void, u32, f32),
    gpio_read: unsafe extern "C" fn(*mut c_void, u32) -> bool,
    gpio_write: unsafe extern "C" fn(*mut c_void, u32, bool),
    uart_write: unsafe extern "C" fn(*mut c_void, u32, *const u8, u32) -> u32,
    uart_read: unsafe extern "C" fn(*mut c_void, u32, *mut u8, u32) -> u32,
    log: unsafe extern "C" fn(*mut c_void, u32, *const c_char),
    trace: unsafe extern "C" fn(*mut c_void, u32, f64),
    ctx: *mut c_void,
}

thread_local! {
    /// 呼び出し中だけ有効な、現在のレジスタ像へのポインタ。スケジューラは単一スレッドなので thread_local で足りる。
    static CURRENT: Cell<*mut HalRegs> = const { Cell::new(std::ptr::null_mut()) };
}

fn with_regs<R>(f: impl FnOnce(&mut HalRegs) -> R, default: R) -> R {
    CURRENT.with(|c| {
        let p = c.get();
        if p.is_null() {
            default
        } else {
            // SAFETY: CURRENT は tick/init の呼び出し中だけ、有効な &mut HalRegs を指す
            f(unsafe { &mut *p })
        }
    })
}

unsafe extern "C" fn cb_micros(_: *mut c_void) -> u64 {
    with_regs(|r| r.hal_micros(), 0)
}
unsafe extern "C" fn cb_enc_read(_: *mut c_void, ch: u32) -> i32 {
    with_regs(|r| r.hal_enc_read(ch), 0)
}
unsafe extern "C" fn cb_pwm_write(_: *mut c_void, ch: u32, duty: f32) {
    with_regs(|r| r.hal_pwm_write(ch, duty), ())
}
unsafe extern "C" fn cb_dac_write(_: *mut c_void, ch: u32, v: f32) {
    with_regs(|r| r.hal_dac_write(ch, v), ())
}
unsafe extern "C" fn cb_gpio_read(_: *mut c_void, pin: u32) -> bool {
    with_regs(|r| r.hal_gpio_read(pin), false)
}
unsafe extern "C" fn cb_gpio_write(_: *mut c_void, pin: u32, level: bool) {
    with_regs(|r| r.hal_gpio_write(pin, level), ())
}
unsafe extern "C" fn cb_uart_write(_: *mut c_void, ch: u32, data: *const u8, len: u32) -> u32 {
    if data.is_null() {
        return 0;
    }
    let slice = std::slice::from_raw_parts(data, len as usize);
    with_regs(|r| r.hal_uart_write(ch, slice), 0)
}
unsafe extern "C" fn cb_uart_read(_: *mut c_void, ch: u32, buf: *mut u8, max: u32) -> u32 {
    if buf.is_null() {
        return 0;
    }
    let v = with_regs(|r| r.hal_uart_read(ch, max), Vec::new());
    std::ptr::copy_nonoverlapping(v.as_ptr(), buf, v.len());
    v.len() as u32
}
unsafe extern "C" fn cb_log(_: *mut c_void, level: u32, msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    let s = CStr::from_ptr(msg).to_string_lossy().into_owned();
    with_regs(|r| r.hal_log(LogLevel::from_u32(level), &s), ())
}
unsafe extern "C" fn cb_trace(_: *mut c_void, id: u32, value: f64) {
    with_regs(|r| r.hal_trace(id, value), ())
}

type AbiVersionFn = unsafe extern "C" fn() -> u32;
type InitFn = unsafe extern "C" fn(*const IpHalVtable, *const c_char, usize) -> i32;
type TickFn = unsafe extern "C" fn();
type IrqFn = unsafe extern "C" fn(u32);

pub struct NativeFirmware {
    _lib: Library,
    vtable: Box<IpHalVtable>,
    abi_version: AbiVersionFn,
    init: InitFn,
    tick: TickFn,
    irq: IrqFn,
}

impl NativeFirmware {
    pub fn load_file(path: &Path) -> anyhow::Result<Self> {
        // SAFETY: 共有ライブラリの読み込み。信頼した開発ビルドだけを読む前提 (デバッグ専用形)
        let lib = unsafe { Library::new(path) }
            .with_context(|| format!("{} を読み込めません", path.display()))?;
        let (abi_version, init, tick, irq) = unsafe {
            let a: Symbol<AbiVersionFn> = lib
                .get(b"ip_fw_abi_version\0")
                .context("ip_fw_abi_version がありません")?;
            let i: Symbol<InitFn> = lib
                .get(b"ip_fw_init\0")
                .context("ip_fw_init がありません")?;
            let t: Symbol<TickFn> = lib
                .get(b"ip_fw_tick\0")
                .context("ip_fw_tick がありません")?;
            let q: Symbol<IrqFn> = lib.get(b"ip_fw_irq\0").context("ip_fw_irq がありません")?;
            (*a, *i, *t, *q)
        };
        let vtable = Box::new(IpHalVtable {
            size: std::mem::size_of::<IpHalVtable>() as u32,
            micros: cb_micros,
            enc_read: cb_enc_read,
            pwm_write: cb_pwm_write,
            dac_write: cb_dac_write,
            gpio_read: cb_gpio_read,
            gpio_write: cb_gpio_write,
            uart_write: cb_uart_write,
            uart_read: cb_uart_read,
            log: cb_log,
            trace: cb_trace,
            ctx: std::ptr::null_mut(),
        });
        Ok(Self {
            _lib: lib,
            vtable,
            abi_version,
            init,
            tick,
            irq,
        })
    }

    fn enter<R>(regs: &mut HalRegs, f: impl FnOnce() -> R) -> R {
        CURRENT.with(|c| c.set(regs as *mut HalRegs));
        let r = f();
        CURRENT.with(|c| c.set(std::ptr::null_mut()));
        r
    }
}

impl Firmware for NativeFirmware {
    fn abi_version(&mut self) -> anyhow::Result<u32> {
        Ok(unsafe { (self.abi_version)() })
    }

    fn init(&mut self, regs: &mut HalRegs, cfg_json: &str) -> anyhow::Result<()> {
        let vt: *const IpHalVtable = &*self.vtable;
        let rc = Self::enter(regs, || unsafe {
            (self.init)(vt, cfg_json.as_ptr() as *const c_char, cfg_json.len())
        });
        anyhow::ensure!(
            rc == 0,
            "ネイティブファームウェアの init が {rc} を返しました"
        );
        Ok(())
    }

    fn tick(&mut self, regs: &mut HalRegs) -> anyhow::Result<TickStats> {
        Self::enter(regs, || unsafe { (self.tick)() });
        Ok(TickStats { fuel: None })
    }

    fn irq(&mut self, regs: &mut HalRegs, id: u32) -> anyhow::Result<()> {
        Self::enter(regs, || unsafe { (self.irq)(id) });
        Ok(())
    }

    fn flavor(&self) -> &'static str {
        "native"
    }
}
