//! アップロードされた wasm の検査: Component Model の import 一覧 (wasi:* の抽出) と、
//! `ipemu-wasm` による実体化 (part なら describe()、firmware なら world の適合確認)。

use anyhow::Context;
use ipemu_core::PartDesc;
use ipemu_wasm::{Runtime, WasmFirmware, WasmPart};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    /// 実装するワールド (`ip:emu/part-world@0.1.0` / `ip:emu/firmware@0.1.0`)。export 名から推定
    pub world: String,
    /// 最上位の import 名 (例 `ip:emu/host@0.1.0`, `wasi:clocks/wall-clock@0.2.0`)
    pub imports: Vec<String>,
    /// imports のうち `wasi:` で始まるもの (V7 / V8 の照合用)
    pub wasi_imports: Vec<String>,
    pub exports: Vec<String>,
    /// part のときの describe()
    pub describe: Option<PartDesc>,
}

/// Component Model バイナリの最上位 import / export 名を wasmparser で読む。
pub fn component_io(bytes: &[u8]) -> anyhow::Result<(Vec<String>, Vec<String>)> {
    use wasmparser::{Parser, Payload};
    let mut imports = Vec::new();
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
            Payload::ComponentImportSection(s) if depth == 1 => {
                for imp in s {
                    imports.push(imp?.name.name.to_owned());
                }
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
    Ok((imports, exports))
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
            let (imports, exports) = component_io(bytes)?;
            let wasi_imports: Vec<String> = imports
                .iter()
                .filter(|i| i.starts_with("wasi:"))
                .cloned()
                .collect();
            let world = guess_world(&exports);
            // 実体化は wasi:* を一切 link しない Runtime で行う。wasi を import する部品は
            // ここでは describe を取れない (capability 付きの link は M5)。import 一覧だけ保存して先へ進む
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
                if wasi_imports.is_empty() {
                    let component = WasmPart::compile(&rt, bytes)?;
                    Some(
                        WasmPart::describe_component(&rt, &component)
                            .context("describe() の実行に失敗")?,
                    )
                } else {
                    None
                }
            } else {
                anyhow::ensure!(
                    world.starts_with("ip:emu/firmware@"),
                    "firmware は ip:emu/fw を export する必要があります (exports: {exports:?})"
                );
                if wasi_imports.is_empty() {
                    WasmFirmware::load(&rt, bytes).context("ファームウェアの実体化に失敗")?;
                }
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
