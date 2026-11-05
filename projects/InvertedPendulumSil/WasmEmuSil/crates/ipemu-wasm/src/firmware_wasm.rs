//! wasm 形ファームウェア (`ip:emu/firmware` world) を `ipemu_core::hal::Firmware` として動かす。

use crate::runtime::Runtime;
use anyhow::Context;
use ipemu_core::hal::{Firmware, HalRegs, LogLevel, TickStats};
use std::path::Path;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::Store;

mod bindings {
    wasmtime::component::bindgen!({
        path: "../../abi/wit",
        world: "firmware",
    });
}

use bindings::ip::emu::types as wit_types;

/// Store データ: tick 中だけ cpu-virtual のレジスタ像を預かる。
pub struct FwHost {
    pub regs: HalRegs,
}

fn convert_level(l: wit_types::LogLevel) -> LogLevel {
    match l {
        wit_types::LogLevel::Trace => LogLevel::Trace,
        wit_types::LogLevel::Debug => LogLevel::Debug,
        wit_types::LogLevel::Info => LogLevel::Info,
        wit_types::LogLevel::Warn => LogLevel::Warn,
        wit_types::LogLevel::Error => LogLevel::Error,
    }
}

impl bindings::ip::emu::types::Host for FwHost {}

impl bindings::ip::emu::hal::Host for FwHost {
    fn micros(&mut self) -> u64 {
        self.regs.hal_micros()
    }
    fn enc_read(&mut self, ch: u32) -> i32 {
        self.regs.hal_enc_read(ch)
    }
    fn pwm_write(&mut self, ch: u32, duty: f32) {
        self.regs.hal_pwm_write(ch, duty)
    }
    fn dac_write(&mut self, ch: u32, value: f32) {
        self.regs.hal_dac_write(ch, value)
    }
    fn gpio_read(&mut self, pin: u32) -> bool {
        self.regs.hal_gpio_read(pin)
    }
    fn gpio_write(&mut self, pin: u32, level: bool) {
        self.regs.hal_gpio_write(pin, level)
    }
    fn uart_write(&mut self, ch: u32, data: Vec<u8>) -> u32 {
        self.regs.hal_uart_write(ch, &data)
    }
    fn uart_read(&mut self, ch: u32, max: u32) -> Vec<u8> {
        self.regs.hal_uart_read(ch, max)
    }
    fn log(&mut self, level: wit_types::LogLevel, msg: String) {
        self.regs.hal_log(convert_level(level), &msg)
    }
    fn trace(&mut self, id: u32, value: f64) {
        self.regs.hal_trace(id, value)
    }
}

pub struct WasmFirmware {
    store: Store<FwHost>,
    world: bindings::Firmware,
    epoch_budget: u64,
    measure_fuel: bool,
}

impl WasmFirmware {
    pub fn load_file(rt: &Runtime, path: &Path) -> anyhow::Result<Self> {
        let bytes =
            std::fs::read(path).with_context(|| format!("{} を読めません", path.display()))?;
        Self::load(rt, &bytes)
    }

    pub fn load(rt: &Runtime, bytes: &[u8]) -> anyhow::Result<Self> {
        let component = Component::new(&rt.engine, bytes)
            .map_err(anyhow::Error::from)
            .context("ファームウェアのコンパイルに失敗")?;
        let mut linker: Linker<FwHost> = Linker::new(&rt.engine);
        bindings::Firmware::add_to_linker::<_, HasSelf<_>>(&mut linker, |h| h)?;
        let mut store = Store::new(
            &rt.engine,
            FwHost {
                regs: HalRegs::default(),
            },
        );
        let epoch_budget = rt.epoch_budget();
        store.set_epoch_deadline(epoch_budget);
        if rt.options.measure_fuel {
            store.set_fuel(u64::MAX / 2)?;
        }
        let world = bindings::Firmware::instantiate(&mut store, &component, &linker)
            .map_err(anyhow::Error::from)
            .context("ファームウェアの実体化に失敗 (import 不足か world 不一致。wasi:* を使うなら capability が必要)")?;
        Ok(Self {
            store,
            world,
            epoch_budget,
            measure_fuel: rt.options.measure_fuel,
        })
    }

    fn with_regs<R>(&mut self, regs: &mut HalRegs, f: impl FnOnce(&mut Self) -> R) -> R {
        std::mem::swap(&mut self.store.data_mut().regs, regs);
        let r = f(self);
        std::mem::swap(&mut self.store.data_mut().regs, regs);
        r
    }
}

impl Firmware for WasmFirmware {
    fn abi_version(&mut self) -> anyhow::Result<u32> {
        self.store.set_epoch_deadline(self.epoch_budget);
        Ok(self.world.ip_emu_fw().call_abi_version(&mut self.store)?)
    }

    fn init(&mut self, regs: &mut HalRegs, cfg_json: &str) -> anyhow::Result<()> {
        self.store.set_epoch_deadline(self.epoch_budget);
        self.with_regs(regs, |s| {
            s.world
                .ip_emu_fw()
                .call_init(&mut s.store, cfg_json)?
                .map_err(|e| anyhow::anyhow!("ファームウェアの init が失敗: {e}"))
        })
    }

    fn tick(&mut self, regs: &mut HalRegs) -> anyhow::Result<TickStats> {
        self.store.set_epoch_deadline(self.epoch_budget);
        let before = if self.measure_fuel {
            self.store.get_fuel().ok()
        } else {
            None
        };
        self.with_regs(regs, |s| s.world.ip_emu_fw().call_tick(&mut s.store))?;
        let fuel = match before {
            Some(b) => self.store.get_fuel().ok().map(|a| b.saturating_sub(a)),
            None => None,
        };
        Ok(TickStats { fuel })
    }

    fn irq(&mut self, regs: &mut HalRegs, id: u32) -> anyhow::Result<()> {
        self.store.set_epoch_deadline(self.epoch_budget);
        Ok(self.with_regs(regs, |s| s.world.ip_emu_fw().call_irq(&mut s.store, id))?)
    }

    fn flavor(&self) -> &'static str {
        "wasm"
    }

    fn hot_reloadable(&self) -> bool {
        true
    }
}
