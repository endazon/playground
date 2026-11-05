//! wasm 形ファームウェア (`ip:emu/firmware` world) を `ipemu_core::hal::Firmware` として動かす。

use crate::runtime::Runtime;
use crate::wasi::{WasiGrant, WasiHost, WasiState};
use anyhow::Context;
use ipemu_core::hal::{Firmware, FuelLimits, HalRegs, LogLevel, TickStats};
use std::path::Path;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Store, Trap};
use wasmtime_wasi::{WasiCtxView, WasiView};

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
    /// cpu-virtual の instance に宣言された capability (ファームウェアに適用する)
    pub wasi: WasiState,
}

impl WasiView for FwHost {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        self.wasi.view()
    }
}

impl WasiHost for FwHost {
    fn wasi(&mut self) -> &mut WasiState {
        &mut self.wasi
    }
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

/// 上限なしのときに毎回与える fuel。1 呼び出しで使い切れる量ではない
const UNLIMITED_FUEL: u64 = u64::MAX / 2;

/// 呼び出しの種類 (fuel 上限の選択とエラー文言に使う)。
#[derive(Clone, Copy)]
enum Call {
    AbiVersion,
    Init,
    Tick,
    Irq,
}

impl Call {
    fn name(self) -> &'static str {
        match self {
            Call::AbiVersion => "abi-version",
            Call::Init => "init",
            Call::Tick => "tick",
            Call::Irq => "irq",
        }
    }
    /// 効く上限の manifest キー
    fn limit_key(self) -> &'static str {
        match self {
            Call::AbiVersion | Call::Init => "maxFuelPerInit",
            Call::Tick | Call::Irq => "maxFuelPerTick",
        }
    }
}

pub struct WasmFirmware {
    store: Store<FwHost>,
    world: bindings::Firmware,
    epoch_budget: u64,
    measure_fuel: bool,
    limits: FuelLimits,
}

impl WasmFirmware {
    pub fn load_file(rt: &Runtime, path: &Path) -> anyhow::Result<Self> {
        let bytes =
            std::fs::read(path).with_context(|| format!("{} を読めません", path.display()))?;
        Self::load(rt, &bytes)
    }

    pub fn load(rt: &Runtime, bytes: &[u8]) -> anyhow::Result<Self> {
        Self::load_with(rt, bytes, WasiGrant::default())
    }

    /// capability を付与して読み込む。宣言外の import は link されないので実体化で失敗する。
    pub fn load_with(rt: &Runtime, bytes: &[u8], grant: WasiGrant) -> anyhow::Result<Self> {
        let component = Component::new(&rt.engine, bytes)
            .map_err(anyhow::Error::from)
            .context("ファームウェアのコンパイルに失敗")?;
        let mut linker: Linker<FwHost> = Linker::new(&rt.engine);
        bindings::Firmware::add_to_linker::<_, HasSelf<_>>(&mut linker, |h| h)?;
        crate::wasi::add_to_linker(&mut linker, &grant)
            .context("ファームウェアへの capability の付与")?;
        let mut store = Store::new(
            &rt.engine,
            FwHost {
                regs: HalRegs::default(),
                wasi: WasiState::new(grant).context("ファームウェアの WASI 文脈")?,
            },
        );
        let epoch_budget = rt.epoch_budget();
        store.set_epoch_deadline(epoch_budget);
        if rt.options.measure_fuel {
            // 実体化 (start 関数など) の分。呼び出しごとに arm で張り直す
            store.set_fuel(UNLIMITED_FUEL)?;
        }
        let world = bindings::Firmware::instantiate(&mut store, &component, &linker)
            .map_err(anyhow::Error::from)
            .context("ファームウェアの実体化に失敗 (import 不足か world 不一致。wasi:* を使うなら capability が必要)")?;
        Ok(Self {
            store,
            world,
            epoch_budget,
            measure_fuel: rt.options.measure_fuel,
            limits: FuelLimits::default(),
        })
    }

    /// 呼び出しの直前に、壁時計の予算 (epoch) と fuel の予算を張り直す。戻り値は張った fuel 量。
    /// fuel は呼び出しごとに上限ちょうどから始めるので、消費量と trap の位置は直前の呼び出しに依存しない。
    fn arm(&mut self, call: Call) -> Option<u64> {
        self.store.set_epoch_deadline(self.epoch_budget);
        if !self.measure_fuel {
            return None;
        }
        let fuel = self.limit(call).unwrap_or(UNLIMITED_FUEL);
        self.store.set_fuel(fuel).ok().map(|_| fuel)
    }

    fn limit(&self, call: Call) -> Option<u64> {
        match call {
            Call::AbiVersion | Call::Init => self.limits.per_init,
            Call::Tick | Call::Irq => self.limits.per_tick,
        }
    }

    fn consumed(&self, armed: Option<u64>) -> Option<u64> {
        let a = armed?;
        self.store
            .get_fuel()
            .ok()
            .map(|left| a.saturating_sub(left))
    }

    /// wasmtime のエラーを、fuel 切れなら上限の値が分かる文言にして返す。
    fn map_err(&self, call: Call, e: wasmtime::Error) -> anyhow::Error {
        match (e.downcast_ref::<Trap>(), self.limit(call)) {
            (Some(Trap::OutOfFuel), Some(n)) => anyhow::Error::from(e).context(format!(
                "ファームウェアの {} が fuel の上限 ({}={n}) を使い切りました (無限ループか予算不足)",
                call.name(),
                call.limit_key()
            )),
            (Some(Trap::Interrupt), _) => anyhow::Error::from(e).context(format!(
                "ファームウェアの {} が壁時計の予算 (epoch ウォッチドッグ) を超えました",
                call.name()
            )),
            _ => anyhow::Error::from(e),
        }
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
        self.arm(Call::AbiVersion);
        self.world
            .ip_emu_fw()
            .call_abi_version(&mut self.store)
            .map_err(|e| self.map_err(Call::AbiVersion, e))
    }

    fn init(&mut self, regs: &mut HalRegs, cfg_json: &str) -> anyhow::Result<()> {
        self.arm(Call::Init);
        let r = self.with_regs(regs, |s| {
            s.world.ip_emu_fw().call_init(&mut s.store, cfg_json)
        });
        r.map_err(|e| self.map_err(Call::Init, e))?
            .map_err(|e| anyhow::anyhow!("ファームウェアの init が失敗: {e}"))
    }

    fn tick(&mut self, regs: &mut HalRegs) -> anyhow::Result<TickStats> {
        let armed = self.arm(Call::Tick);
        let r = self.with_regs(regs, |s| s.world.ip_emu_fw().call_tick(&mut s.store));
        r.map_err(|e| self.map_err(Call::Tick, e))?;
        Ok(TickStats {
            fuel: self.consumed(armed),
        })
    }

    fn irq(&mut self, regs: &mut HalRegs, id: u32) -> anyhow::Result<()> {
        self.arm(Call::Irq);
        let r = self.with_regs(regs, |s| s.world.ip_emu_fw().call_irq(&mut s.store, id));
        r.map_err(|e| self.map_err(Call::Irq, e))
    }

    fn flavor(&self) -> &'static str {
        "wasm"
    }

    fn hot_reloadable(&self) -> bool {
        true
    }

    fn meters_fuel(&self) -> bool {
        self.measure_fuel
    }

    fn set_fuel_limits(&mut self, limits: FuelLimits) {
        self.limits = limits;
    }
}
