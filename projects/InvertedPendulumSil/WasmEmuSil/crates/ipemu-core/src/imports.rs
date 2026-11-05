//! wasm コンポーネントのバイナリから、最上位の import 名を取り出す (`wasm-tools component wit` の import 行と同じ集合)。
//!
//! サーバは部品登録時にこれをメタデータに保存し、クライアントは起動時に実物から再抽出して
//! V7 / V8 に使う。Wasmtime に依存せず `wasmparser` だけで読む。

use crate::capability::ImportInfo;
use anyhow::{bail, Context};
use wasmparser::{
    ComponentImportSectionReader, ComponentType, ComponentTypeRef, ComponentTypeSectionReader,
    InstanceTypeDeclaration, Parser, Payload, TypeBounds,
};

/// 最上位コンポーネントの型インデックス空間に置かれた型 (instance 型だけ中身を見る)。
enum TopType {
    /// instance 型: 関数・リソース・入れ子 instance を 1 つでも export するか
    Instance {
        has_runtime_items: bool,
    },
    Other,
}

/// コンポーネントの import 一覧を返す。名前順に整列し、重複は除く。
pub fn component_imports(bytes: &[u8]) -> anyhow::Result<Vec<ImportInfo>> {
    let mut depth = 0usize;
    let mut types: Vec<TopType> = Vec::new();
    let mut out: Vec<ImportInfo> = Vec::new();
    let mut is_component = false;

    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.context("wasm の解析に失敗")?;
        match payload {
            Payload::Version { encoding, .. } => {
                if depth == 0 {
                    is_component = encoding == wasmparser::Encoding::Component;
                }
            }
            // 入れ子のモジュール / コンポーネントは実装の詳細。最上位だけを見る
            Payload::ModuleSection { .. } | Payload::ComponentSection { .. } => depth += 1,
            Payload::End(_) => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            Payload::ComponentTypeSection(r) if depth == 0 => read_types(r, &mut types)?,
            Payload::ComponentAliasSection(r) if depth == 0 => {
                for a in r {
                    if let wasmparser::ComponentAlias::InstanceExport {
                        kind: wasmparser::ComponentExternalKind::Type,
                        ..
                    }
                    | wasmparser::ComponentAlias::Outer {
                        kind: wasmparser::ComponentOuterAliasKind::Type,
                        ..
                    } = a?
                    {
                        types.push(TopType::Other);
                    }
                }
            }
            Payload::ComponentImportSection(r) if depth == 0 => {
                read_imports(r, &mut types, &mut out)?
            }
            _ => {}
        }
    }
    if !is_component {
        bail!("コンポーネントではありません (コアモジュール)。wasm-tools component new で変換してください");
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.name == b.name);
    Ok(out)
}

fn read_types(r: ComponentTypeSectionReader<'_>, types: &mut Vec<TopType>) -> anyhow::Result<()> {
    for t in r {
        let t = match t? {
            ComponentType::Instance(decls) => {
                let has_runtime_items = decls.iter().any(|d| match d {
                    InstanceTypeDeclaration::Export { ty, .. } => is_runtime_ref(ty),
                    _ => false,
                });
                TopType::Instance { has_runtime_items }
            }
            _ => TopType::Other,
        };
        types.push(t);
    }
    Ok(())
}

/// Linker に実体を要求する参照か。型の別名 (`type x = ...`) は要求しない。
fn is_runtime_ref(ty: &ComponentTypeRef) -> bool {
    match ty {
        ComponentTypeRef::Type(TypeBounds::Eq(_)) => false,
        // リソースの定義は実体 (型インデックス) が要る
        ComponentTypeRef::Type(TypeBounds::SubResource) => true,
        ComponentTypeRef::Func(_)
        | ComponentTypeRef::Module(_)
        | ComponentTypeRef::Value(_)
        | ComponentTypeRef::Instance(_)
        | ComponentTypeRef::Component(_) => true,
    }
}

fn read_imports(
    r: ComponentImportSectionReader<'_>,
    types: &mut Vec<TopType>,
    out: &mut Vec<ImportInfo>,
) -> anyhow::Result<()> {
    for imp in r {
        let imp = imp?;
        let name = imp.name.name.to_owned();
        let types_only = match imp.ty {
            ComponentTypeRef::Instance(idx) => match types.get(idx as usize) {
                Some(TopType::Instance { has_runtime_items }) => !has_runtime_items,
                _ => false,
            },
            ComponentTypeRef::Type(TypeBounds::Eq(_)) => {
                types.push(TopType::Other);
                true
            }
            ComponentTypeRef::Type(TypeBounds::SubResource) => {
                types.push(TopType::Other);
                false
            }
            _ => false,
        };
        out.push(ImportInfo { name, types_only });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WAT で書いた最小のコンポーネント: 関数を持つ instance、型だけの instance、関数 import。
    #[test]
    fn extracts_top_level_imports_only() {
        let wat = r#"
        (component
          (type $types (instance (type $dt (record (field "s" u64))) (export "datetime" (type (eq $dt)))))
          (import "wasi:clocks/wall-clock@0.2.12" (instance (type $types)))
          (import "ip:emu/host@0.1.0" (instance (export "now-ns" (func (result s64)))))
          (import "wasi:sockets/network@0.2.12" (instance (export "network" (type (sub resource)))))
          (component $inner
            (import "nested:pkg/x" (instance (export "f" (func))))
          )
        )"#;
        let bytes = wat::parse_str(wat).unwrap();
        let imports = component_imports(&bytes).unwrap();
        let names: Vec<(&str, bool)> = imports
            .iter()
            .map(|i| (i.name.as_str(), i.types_only))
            .collect();
        assert_eq!(
            names,
            [
                ("ip:emu/host@0.1.0", false),
                ("wasi:clocks/wall-clock@0.2.12", true),
                ("wasi:sockets/network@0.2.12", false),
            ]
        );
    }

    #[test]
    fn rejects_core_module() {
        let bytes = wat::parse_str("(module)").unwrap();
        assert!(component_imports(&bytes).is_err());
    }
}
