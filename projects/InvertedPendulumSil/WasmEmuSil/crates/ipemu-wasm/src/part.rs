//! wasm 部品 (`ip:emu/part-world`) を `ipemu_core::Part` として動かす。

use crate::runtime::Runtime;
use crate::wasi::{WasiGrant, WasiHost, WasiState};
use anyhow::Context;
use ipemu_core::desc::{PartDesc, PortDesc, PortKind};
use ipemu_core::hal::LogLevel;
use ipemu_core::ImportInfo;
use ipemu_core::Part;
use std::path::Path;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::Store;
use wasmtime_wasi::{WasiCtxView, WasiView};

mod bindings {
    wasmtime::component::bindgen!({
        path: "../../abi/wit",
        world: "part-world",
    });
}

use bindings::ip::emu::types as wit_types;

/// 部品ごとの Store データ: ホスト関数の状態。
pub struct PartHost {
    pub now_ns: i64,
    pub logs: Vec<(LogLevel, String)>,
    pub traces: Vec<(u32, f64)>,
    pub instance_name: String,
    /// 付与された capability の WASI 状態 (付与なしでも空の ctx を持つが、link しないので到達しない)
    pub wasi: WasiState,
}

impl WasiView for PartHost {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        self.wasi.view()
    }
}

impl WasiHost for PartHost {
    fn wasi(&mut self) -> &mut WasiState {
        &mut self.wasi
    }
}

impl bindings::ip::emu::host::Host for PartHost {
    fn now_ns(&mut self) -> i64 {
        self.now_ns
    }
    fn log(&mut self, level: wit_types::LogLevel, msg: String) {
        if self.logs.len() < 1024 {
            self.logs.push((convert_level(level), msg));
        }
    }
    fn trace(&mut self, id: u32, value: f64) {
        if self.traces.len() < 4096 {
            self.traces.push((id, value));
        }
    }
}

impl bindings::ip::emu::types::Host for PartHost {}

fn convert_level(l: wit_types::LogLevel) -> LogLevel {
    match l {
        wit_types::LogLevel::Trace => LogLevel::Trace,
        wit_types::LogLevel::Debug => LogLevel::Debug,
        wit_types::LogLevel::Info => LogLevel::Info,
        wit_types::LogLevel::Warn => LogLevel::Warn,
        wit_types::LogLevel::Error => LogLevel::Error,
    }
}

fn convert_desc(d: wit_types::PartDesc) -> PartDesc {
    let port = |p: wit_types::PortDesc| PortDesc {
        name: p.name,
        kind: match p.kind {
            wit_types::PortKind::Scalar => PortKind::Scalar,
            wit_types::PortKind::Bytes => PortKind::Bytes,
        },
        unit: p.unit,
        default: p.default,
        doc: p.doc,
    };
    PartDesc {
        name: d.name,
        version: d.version,
        inputs: d.inputs.into_iter().map(port).collect(),
        outputs: d.outputs.into_iter().map(port).collect(),
        params: d
            .params
            .into_iter()
            .map(|p| ipemu_core::desc::ParamDesc {
                name: p.name,
                schema_json: p.schema_json,
                doc: p.doc,
            })
            .collect(),
        deterministic: d.deterministic,
        doc: d.doc,
    }
}

/// Wasmtime 上の部品。
pub struct WasmPart {
    store: Store<PartHost>,
    world: bindings::PartWorld,
    desc: PartDesc,
    epoch_budget: u64,
    scalar_outputs: Vec<f64>,
    t_ns: i64,
}

impl WasmPart {
    /// コンパイル済みコンポーネントを返す (同じバイナリを複数 instance で共有するため)。
    pub fn compile(rt: &Runtime, bytes: &[u8]) -> anyhow::Result<Component> {
        Component::new(&rt.engine, bytes)
            .map_err(anyhow::Error::from)
            .context("コンポーネントのコンパイルに失敗")
    }

    pub fn compile_file(rt: &Runtime, path: &Path) -> anyhow::Result<Component> {
        let bytes =
            std::fs::read(path).with_context(|| format!("{} を読めません", path.display()))?;
        Self::compile(rt, &bytes)
    }

    /// 実体化して describe() を読む (capability なし = サンドボックス既定)。
    pub fn instantiate(
        rt: &Runtime,
        component: &Component,
        instance_name: &str,
    ) -> anyhow::Result<Self> {
        Self::instantiate_with(rt, component, instance_name, WasiGrant::default())
    }

    /// 付与する capability を指定して実体化する。`grant` にないインターフェースは link しないので、
    /// 宣言外の import を持つ部品はここで `unresolved import` として失敗する。
    pub fn instantiate_with(
        rt: &Runtime,
        component: &Component,
        instance_name: &str,
        grant: WasiGrant,
    ) -> anyhow::Result<Self> {
        let mut linker: Linker<PartHost> = Linker::new(&rt.engine);
        bindings::PartWorld::add_to_linker::<_, HasSelf<_>>(&mut linker, |h| h)?;
        crate::wasi::add_to_linker(&mut linker, &grant)
            .with_context(|| format!("部品 `{instance_name}` への capability の付与"))?;
        let nondeterministic = grant.nondeterministic();
        let host = PartHost {
            now_ns: 0,
            logs: Vec::new(),
            traces: Vec::new(),
            instance_name: instance_name.to_owned(),
            wasi: WasiState::new(grant)
                .with_context(|| format!("部品 `{instance_name}` の WASI 文脈"))?,
        };
        let mut store = Store::new(&rt.engine, host);
        let epoch_budget = rt.epoch_budget();
        store.set_epoch_deadline(epoch_budget);
        if rt.options.measure_fuel {
            store.set_fuel(u64::MAX / 2)?;
        }
        let world = bindings::PartWorld::instantiate(&mut store, component, &linker)
            .map_err(anyhow::Error::from)
            .with_context(|| {
                format!("部品 `{instance_name}` の実体化に失敗 (import 不足か world 不一致)")
            })?;
        let mut desc = convert_desc(world.ip_emu_part().call_describe(&mut store)?);
        // clock / random を与えた部品は自己申告によらず決定性の保証外
        if nondeterministic {
            desc.deterministic = false;
        }
        let n_out = desc.scalar_output_count();
        Ok(Self {
            store,
            world,
            desc,
            epoch_budget,
            scalar_outputs: vec![0.0; n_out],
            t_ns: 0,
        })
    }

    /// 実体化せずに describe() だけ読む (capability を使わない部品向け。WASI を import する部品は失敗する)。
    pub fn describe_component(rt: &Runtime, component: &Component) -> anyhow::Result<PartDesc> {
        Ok(Self::instantiate(rt, component, "describe")?.desc)
    }

    /// manifest (宣言) がない文脈で describe() を読む: サーバの部品登録時 (M3 `server/src/inspect.rs`)、`inspect`。
    ///
    /// バイナリの import を調べ、対応する WASI インターフェースをすべて **deny-all** 形
    /// (許可リスト空・preopen なし。clock / random は link) で link して実体化し、describe と import 一覧を返す。
    /// describe / init の中で I/O を試みても拒否されるだけで外部には届かない。
    /// `wasi:cli` など提供しない import があれば実体化で失敗する (= 登録時に弾ける)。
    pub fn describe_component_sandboxed(
        rt: &Runtime,
        bytes: &[u8],
    ) -> anyhow::Result<(PartDesc, Vec<ImportInfo>)> {
        let imports = ipemu_core::imports::component_imports(bytes)?;
        let caps = ipemu_core::Capabilities::covering_imports(&imports);
        let grant = WasiGrant::deny_all_for(&caps);
        let component = Self::compile(rt, bytes)?;
        let part = Self::instantiate_with(rt, &component, "describe", grant)?;
        Ok((part.desc, imports))
    }

    fn arm(&mut self) {
        self.store.set_epoch_deadline(self.epoch_budget);
    }

    pub fn drain_traces(&mut self) -> Vec<(u32, f64)> {
        std::mem::take(&mut self.store.data_mut().traces)
    }

    /// 付与された capability。
    pub fn grant(&self) -> &WasiGrant {
        &self.store.data().wasi.grant
    }

    /// 許可リストで拒否した宛先 (診断用)。
    pub fn take_denied_destinations(&mut self) -> Vec<String> {
        self.store.data().wasi.allow.take_denied()
    }
}

impl Part for WasmPart {
    fn describe(&self) -> &PartDesc {
        &self.desc
    }

    fn init(&mut self, params: &serde_json::Value) -> anyhow::Result<()> {
        self.arm();
        let json = serde_json::to_string(params)?;
        self.world
            .ip_emu_part()
            .call_init(&mut self.store, &json)?
            .map_err(|e| {
                anyhow::anyhow!(
                    "部品 `{}` の init が失敗: {e}",
                    self.store.data().instance_name
                )
            })
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        self.arm();
        self.t_ns = 0;
        self.store.data_mut().now_ns = 0;
        Ok(self.world.ip_emu_part().call_reset(&mut self.store)?)
    }

    fn set_inputs(&mut self, values: &[f64]) -> anyhow::Result<()> {
        self.arm();
        Ok(self
            .world
            .ip_emu_part()
            .call_set_inputs(&mut self.store, values)?)
    }

    fn step(&mut self, dt_ns: i64) -> anyhow::Result<()> {
        self.arm();
        self.store.data_mut().now_ns = self.t_ns;
        let r = self.world.ip_emu_part().call_step(&mut self.store, dt_ns)?;
        self.t_ns += dt_ns;
        r.map_err(|e| {
            anyhow::anyhow!(
                "部品 `{}` の step が失敗: {e}",
                self.store.data().instance_name
            )
        })
    }

    fn get_outputs(&mut self, out: &mut [f64]) -> anyhow::Result<()> {
        self.arm();
        let v = self.world.ip_emu_part().call_get_outputs(&mut self.store)?;
        anyhow::ensure!(
            v.len() == out.len(),
            "部品 `{}` の出力数 {} が describe の {} と一致しません",
            self.store.data().instance_name,
            v.len(),
            out.len()
        );
        out.copy_from_slice(&v);
        self.scalar_outputs.copy_from_slice(&v);
        Ok(())
    }

    fn drain_logs(&mut self) -> Vec<(LogLevel, String)> {
        std::mem::take(&mut self.store.data_mut().logs)
    }
}
