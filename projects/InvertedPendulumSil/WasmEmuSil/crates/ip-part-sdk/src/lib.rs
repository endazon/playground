//! 部品を Rust で書くための SDK。
//!
//! 部品は [`PartImpl`] を実装し、`ip_part_sdk::part_entry!(MyPart)` で入口を生やす。
//! WIT の `part` インターフェースとの橋渡し (単一インスタンスの保持、JSON の受け渡し) はここで吸収する。

pub mod bindings {
    wit_bindgen::generate!({
        path: "../../abi/wit",
        world: "part-world",
        pub_export_macro: true,
        default_bindings_module: "ip_part_sdk::bindings",
    });
}

pub use bindings::exports::ip::emu::part::Guest;
pub use bindings::ip::emu::host;
pub use bindings::ip::emu::types::{LogLevel, ParamDesc, PartDesc, PortDesc, PortKind};
pub use serde_json;

/// ポート記述のショートカット。
pub fn scalar(name: &str, unit: &str, doc: &str) -> PortDesc {
    PortDesc {
        name: name.into(),
        kind: PortKind::Scalar,
        unit: unit.into(),
        default: 0.0,
        doc: doc.into(),
    }
}
pub fn scalar_default(name: &str, unit: &str, default: f64, doc: &str) -> PortDesc {
    PortDesc {
        name: name.into(),
        kind: PortKind::Scalar,
        unit: unit.into(),
        default,
        doc: doc.into(),
    }
}

/// 部品の実装が満たす契約。
pub trait PartImpl: Sized {
    fn describe() -> PartDesc;
    /// params (JSON) から初期化する
    fn init(params: &serde_json::Value) -> Result<Self, String>;
    fn reset(&mut self);
    /// describe().inputs の順
    fn set_inputs(&mut self, values: &[f64]);
    fn step(&mut self, dt_ns: i64) -> Result<(), String>;
    /// describe().outputs の順で `out` に書く
    fn get_outputs(&self, out: &mut Vec<f64>);
    fn snapshot(&self) -> Vec<u8> {
        Vec::new()
    }
    fn restore(&mut self, _state: &[u8]) -> Result<(), String> {
        Err("snapshot/restore 未対応".into())
    }
}

/// 仮想時刻 [ns] (ホスト提供)。
pub fn now_ns() -> i64 {
    host::now_ns()
}
pub fn log(level: LogLevel, msg: &str) {
    host::log(level, msg)
}
pub fn trace(id: u32, value: f64) {
    host::trace(id, value)
}

/// 部品の入口を生成する。単一インスタンスを `static` に保持する (wasm は単一スレッド)。
#[macro_export]
macro_rules! part_entry {
    ($ty:ty) => {
        mod __ip_part_entry {
            use super::*;
            use $crate::PartImpl as _;

            static mut INSTANCE: Option<$ty> = None;
            static mut LAST_PARAMS: Option<$crate::serde_json::Value> = None;

            #[allow(static_mut_refs)]
            fn instance() -> Option<&'static mut $ty> {
                unsafe { INSTANCE.as_mut() }
            }

            pub struct Entry;

            impl $crate::Guest for Entry {
                fn describe() -> $crate::PartDesc {
                    <$ty as $crate::PartImpl>::describe()
                }
                #[allow(static_mut_refs)]
                fn init(params_json: String) -> Result<(), String> {
                    let v: $crate::serde_json::Value =
                        $crate::serde_json::from_str(&params_json).map_err(|e| format!("params の JSON が不正: {e}"))?;
                    let inst = <$ty as $crate::PartImpl>::init(&v)?;
                    unsafe {
                        INSTANCE = Some(inst);
                        LAST_PARAMS = Some(v);
                    }
                    Ok(())
                }
                #[allow(static_mut_refs)]
                fn reset() {
                    if let Some(i) = instance() {
                        i.reset();
                    }
                }
                fn set_inputs(values: Vec<f64>) {
                    if let Some(i) = instance() {
                        i.set_inputs(&values);
                    }
                }
                fn get_outputs() -> Vec<f64> {
                    let mut out = Vec::new();
                    if let Some(i) = instance() {
                        i.get_outputs(&mut out);
                    }
                    out
                }
                fn write_bytes(_input_index: u32, _data: Vec<u8>) {}
                fn read_bytes(_output_index: u32) -> Vec<u8> {
                    Vec::new()
                }
                fn step(dt_ns: i64) -> Result<(), String> {
                    match instance() {
                        Some(i) => i.step(dt_ns),
                        None => Err("init が呼ばれていません".into()),
                    }
                }
                fn snapshot() -> Vec<u8> {
                    instance().map(|i| i.snapshot()).unwrap_or_default()
                }
                fn restore(state: Vec<u8>) -> Result<(), String> {
                    match instance() {
                        Some(i) => i.restore(&state),
                        None => Err("init が呼ばれていません".into()),
                    }
                }
            }

            $crate::bindings::export!(Entry with_types_in $crate::bindings);
        }
    };
}

/// JSON から数値を読む補助 (既定値付き)。
pub fn num(v: &serde_json::Value, key: &str, default: f64) -> f64 {
    v.get(key).and_then(|x| x.as_f64()).unwrap_or(default)
}
pub fn num_nested(v: &serde_json::Value, path: &[&str], default: f64) -> f64 {
    let mut cur = v;
    for k in path {
        match cur.get(k) {
            Some(x) => cur = x,
            None => return default,
        }
    }
    cur.as_f64().unwrap_or(default)
}
