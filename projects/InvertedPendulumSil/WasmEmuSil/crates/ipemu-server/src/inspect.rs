//! アップロードされた wasm の検査: Component Model の import 一覧 (`ipemu_core::imports`、エンジンの V7 / V8 と同じ抽出) と、
//! `ipemu-wasm` による実体化 (part なら describe()、firmware なら world の適合確認)。

use anyhow::Context;
use ipemu_core::capability::{Capabilities, ImportInfo};
use ipemu_core::PartDesc;
use ipemu_wasm::wasi::WasiGrant;
use ipemu_wasm::{Runtime, WasmFirmware, WasmPart};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    /// 実装するワールド (`ip:emu/part-world@0.1.0` / `ip:emu/firmware@0.1.0`)。export 名から推定
    pub world: String,
    /// 最上位の import (`ipemu_core::imports::component_imports`)。manifest 検証の V7 / V8 にそのまま渡す
    pub imports: Vec<ImportInfo>,
    /// imports のうち `wasi:` で始まるもの (表示用)
    pub wasi_imports: Vec<String>,
    pub exports: Vec<String>,
    /// part のときの describe()
    pub describe: Option<PartDesc>,
}

/// Component Model バイナリの最上位 export 名を読む (world の推定用。import は ipemu-core に任せる)。
fn component_exports(bytes: &[u8]) -> anyhow::Result<Vec<String>> {
    use wasmparser::{Parser, Payload};
    let mut exports = Vec::new();
    let mut depth = 0usize;
    let mut is_component = false;
    for payload in Parser::new(0).parse_all(bytes) {
        match payload? {
            Payload::Version { encoding, .. } => {
                if depth == 0 {
                    is_component = encoding == wasmparser::Encoding::Component;
                }
                depth += 1;
            }
            Payload::End(_) => {
                depth = depth.saturating_sub(1);
            }
            // 入れ子のコンポーネント / モジュールは読み飛ばす (最上位の import だけが capability の対象)
            Payload::ModuleSection { .. } | Payload::ComponentSection { .. } => {
                // parse_all は入れ子も payload として流してくるので depth で区別する
            }
            Payload::ComponentExportSection(s) if depth == 1 => {
                for exp in s {
                    exports.push(exp?.name.name.to_owned());
                }
            }
            _ => {}
        }
    }
    anyhow::ensure!(
        is_component,
        "Component Model 形式の wasm ではありません (wasm-tools component new を通してください)"
    );
    Ok(exports)
}

/// firmware 用: import に対応する wasi:* を deny-all 形で link する grant (part は `describe_component_sandboxed` が同じことをする)。
fn deny_all_grant_for_imports(imports: &[ImportInfo]) -> WasiGrant {
    WasiGrant::deny_all_for(&Capabilities::covering_imports(imports))
}

fn guess_world(exports: &[String]) -> String {
    if let Some(e) = exports.iter().find(|e| e.starts_with("ip:emu/part@")) {
        return e.replace("ip:emu/part@", "ip:emu/part-world@");
    }
    if let Some(e) = exports.iter().find(|e| e.starts_with("ip:emu/fw@")) {
        return e.replace("ip:emu/fw@", "ip:emu/firmware@");
    }
    String::new()
}

/// kind (`part` / `firmware` / `ui`) に応じて検査する。ui は tar.gz の magic だけ確かめる。
pub fn inspect(kind: &str, bytes: &[u8]) -> anyhow::Result<Inspection> {
    match kind {
        "ui" => {
            anyhow::ensure!(
                bytes.len() > 2 && bytes[0] == 0x1f && bytes[1] == 0x8b,
                "UI バンドルは単一ファイルの tar.gz で登録してください"
            );
            Ok(Inspection::default())
        }
        "part" | "firmware" => {
            let exports = component_exports(bytes)?;
            let imports = ipemu_core::imports::component_imports(bytes)?;
            // 型だけの instance import は Linker に実体が要らない (D32) ので、表示でも区別しない
            let wasi_imports: Vec<String> = imports
                .iter()
                .filter(|i| i.name.starts_with("wasi:") && !i.types_only)
                .map(|i| i.name.clone())
                .collect();
            let world = guess_world(&exports);
            // 実体化は deny-all 形の grant で行う: import に対応する wasi:* は link するが何も許可しない。
            // describe() は通り、外部への副作用は起きない (エンジンの validate / inspect と同じ D34)
            let rt = Runtime::new(ipemu_wasm::runtime::RuntimeOptions {
                call_budget: Some(std::time::Duration::from_secs(5)),
                measure_fuel: false,
                debug_info: false,
            })?;
            let describe = if kind == "part" {
                anyhow::ensure!(
                    world.starts_with("ip:emu/part-world@"),
                    "part は ip:emu/part を export する必要があります (exports: {exports:?})"
                );
                let (desc, _) = WasmPart::describe_component_sandboxed(&rt, bytes)
                    .context("describe() の実行に失敗 (import 不足か world 不一致)")?;
                Some(desc)
            } else {
                anyhow::ensure!(
                    world.starts_with("ip:emu/firmware@"),
                    "firmware は ip:emu/fw を export する必要があります (exports: {exports:?})"
                );
                WasmFirmware::load_with(&rt, bytes, deny_all_grant_for_imports(&imports))
                    .context("ファームウェアの実体化に失敗")?;
                None
            };
            Ok(Inspection {
                world,
                imports,
                wasi_imports,
                exports,
                describe,
            })
        }
        other => anyhow::bail!("kind `{other}` は part / firmware / ui のいずれか"),
    }
}
