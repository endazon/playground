//! TypeScript 型を `bindings/` に書き出し、`index.ts` を更新する。
//! ts-rs は `#[ts(export)]` の型ごとにテストを生成して書き出すので、ここでは索引を作るだけ。

use ipemu_ipc::*;
use std::path::Path;
use ts_rs::TS;

#[test]
fn export_all_and_write_index() {
    // 全型を明示的に書き出す (ts-rs の自動テストと重複しても結果は同じ)
    // 64bit 整数は TS の number にする (仮想時刻 ns で 2^53 まで足りる。bigint は msgpack/JSON と相性が悪い)
    let cfg = ts_rs::Config::from_env().with_large_int("number");
    macro_rules! export {
        ($($t:ty),* $(,)?) => { $( <$t as TS>::export_all(&cfg).unwrap(); )* };
    }
    export!(
        Request,
        Response,
        Notification,
        Discovery,
        RpcError,
        RequestBody,
        ResultBody,
        NotificationBody,
        StateFrame,
        ScopeFrame,
    );

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("bindings");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().into_string().ok()?;
            let stem = n.strip_suffix(".ts")?;
            if stem == "index" || stem.ends_with(".d") {
                return None;
            }
            Some(stem.to_owned())
        })
        .collect();
    names.sort();
    let mut index = String::from(
        "// このファイルは `cargo test -p ipemu-ipc` (scripts/gen-ipc-types.sh) が生成する。手で編集しない。\n",
    );
    for n in &names {
        index.push_str(&format!("export type {{ {n} }} from \"./{n}\";\n"));
    }
    index.push_str("\n/** JSON-RPC エラーコード (crates/ipemu-ipc/src/error.rs と同じ値) */\n");
    index.push_str("export const codes = {\n");
    for (name, code) in [
        ("PARSE_ERROR", codes::PARSE_ERROR),
        ("INVALID_REQUEST", codes::INVALID_REQUEST),
        ("METHOD_NOT_FOUND", codes::METHOD_NOT_FOUND),
        ("INVALID_PARAMS", codes::INVALID_PARAMS),
        ("INTERNAL_ERROR", codes::INTERNAL_ERROR),
        ("ENGINE_ERROR", codes::ENGINE_ERROR),
        ("NO_SESSION", codes::NO_SESSION),
        ("RESTART_REQUIRED", codes::RESTART_REQUIRED),
        ("NOT_IMPLEMENTED", codes::NOT_IMPLEMENTED),
        ("UNAUTHORIZED", codes::UNAUTHORIZED),
        ("NOT_FOUND", codes::NOT_FOUND),
        ("INVALID_STATE", codes::INVALID_STATE),
    ] {
        index.push_str(&format!("  {name}: {code},\n"));
    }
    index.push_str("} as const;\n\n");
    index.push_str(&format!("export const IPC_VERSION = {IPC_VERSION};\n"));
    index.push_str("/** バイナリフレームの先頭バイト */\nexport const FRAME_STATE = 0x01;\nexport const FRAME_SCOPE = 0x02;\n");
    index
        .push_str("/** 要求メソッド名の一覧 (RequestBody::METHODS) */\nexport const METHODS = [\n");
    for m in RequestBody::METHODS {
        index.push_str(&format!("  \"{m}\",\n"));
    }
    index.push_str("] as const;\n");
    let path = dir.join("index.ts");
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    if old != index {
        std::fs::write(&path, index).unwrap();
    }
}
