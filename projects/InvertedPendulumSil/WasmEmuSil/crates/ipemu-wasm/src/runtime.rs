//! Wasmtime の Engine 設定。決定性と暴走対策をここで固定する。

use std::sync::Arc;
use std::time::Duration;
use wasmtime::{Config, Engine};

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    /// 1 回の呼び出し (step / tick) に許す壁時計の予算。超えたら trap。None で無制限 (デバッグ向け)
    pub call_budget: Option<Duration>,
    /// ファームウェアの fuel 計測 (実行時間モデル fuel 用)
    pub measure_fuel: bool,
    /// DWARF を保持して LLDB からデバッグできるようにする (遅くなる)
    pub debug_info: bool,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            call_budget: Some(Duration::from_millis(5000)),
            measure_fuel: true,
            debug_info: false,
        }
    }
}

/// 共有の Wasmtime Engine と epoch ウォッチドッグ。
pub struct Runtime {
    pub engine: Engine,
    pub options: RuntimeOptions,
    _ticker: Option<EpochTicker>,
}

struct EpochTicker {
    stop: Arc<std::sync::atomic::AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Drop for EpochTicker {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Runtime {
    pub fn new(options: RuntimeOptions) -> anyhow::Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        // 決定性: NaN のビットパターンを正規化する
        config.cranelift_nan_canonicalization(true);
        // 暴走対策: epoch interruption。予算が無制限でも有効にしておき、deadline を大きく取る
        config.epoch_interruption(true);
        config.consume_fuel(options.measure_fuel);
        config.debug_info(options.debug_info);
        let engine = Engine::new(&config)?;

        // epoch を 1ms ごとに進めるスレッド。Store 側は call_budget に相当する epoch 数を deadline にする
        let ticker = options.call_budget.map(|_| {
            let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let e = engine.clone();
            let s = stop.clone();
            let handle = std::thread::Builder::new()
                .name("ipemu-epoch".into())
                .spawn(move || {
                    while !s.load(std::sync::atomic::Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_millis(1));
                        e.increment_epoch();
                    }
                })
                .expect("epoch ticker");
            EpochTicker {
                stop,
                handle: Some(handle),
            }
        });

        Ok(Self {
            engine,
            options,
            _ticker: ticker,
        })
    }

    /// 1 呼び出しあたりの epoch 予算 (1 epoch ≈ 1ms)。
    pub fn epoch_budget(&self) -> u64 {
        match self.options.call_budget {
            Some(d) => (d.as_millis() as u64).max(2),
            None => u64::MAX / 4,
        }
    }
}
