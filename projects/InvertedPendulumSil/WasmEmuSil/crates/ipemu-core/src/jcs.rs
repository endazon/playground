//! RFC 8785 (JSON Canonicalization Scheme) の正規化。
//!
//! `approval.capabilitiesHash` と manifest 署名 (M3) の入力をサーバとクライアントで
//! 同じバイト列にするために使う。serde_json の `Value` を受け取り、
//! オブジェクトのキーを UTF-16 コード単位順に並べ、空白なし・ES6 の数値表記で出力する。

use serde_json::Value;
use std::fmt::Write;

/// `Value` を RFC 8785 の正規形にする。
pub fn canonicalize(v: &Value) -> String {
    let mut out = String::new();
    write_value(v, &mut out);
    out
}

fn write_value(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => write_number(n, out),
        Value::String(s) => write_string(s, out),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(x, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            // JCS: キーは UTF-16 コード単位の列として比較して昇順
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(k, out);
                out.push(':');
                write_value(&m[*k], out);
            }
            out.push('}');
        }
    }
}

fn write_number(n: &serde_json::Number, out: &mut String) {
    // JCS の数値は IEEE 754 倍精度を ES6 の Number.prototype.toString で表記する。
    // 整数も倍精度に載る範囲 (|n| <= 2^53) ならそのまま 10 進で一致する。
    if let Some(i) = n.as_i64() {
        if i.unsigned_abs() <= (1u64 << 53) {
            let _ = write!(out, "{i}");
            return;
        }
    }
    if let Some(u) = n.as_u64() {
        if u <= (1u64 << 53) {
            let _ = write!(out, "{u}");
            return;
        }
    }
    let f = n.as_f64().unwrap_or(0.0);
    let mut buf = ryu_js::Buffer::new();
    out.push_str(buf.format(f));
}

fn write_string(s: &str, out: &mut String) {
    // JSON の最小エスケープ (RFC 8785 3.2.2.2)。制御文字は \uXXXX (小文字 hex)
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::canonicalize;

    #[test]
    fn rfc8785_sample() {
        // RFC 8785 Section 3.2.3 のサンプル (入力と期待出力)
        let input = "{\"numbers\": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001], \"string\": \"\\u20ac$\\u000F\\u000aA'\\u0042\\u0022\\u005c\\\\\\\"\\/\", \"literals\": [null, true, false]}";
        let v: serde_json::Value = serde_json::from_str(input).unwrap();
        let expected = "{\"literals\":[null,true,false],\"numbers\":[333333333.3333333,1e+30,4.5,0.002,1e-27],\"string\":\"€$\\u000f\\nA'B\\\"\\\\\\\\\\\"/\"}";
        assert_eq!(canonicalize(&v), expected);
    }

    #[test]
    fn key_order_is_utf16() {
        // RFC 8785 3.2.3: BMP の文字 (UTF-16 で 1 単位) はサロゲートペア (0xD83D…) より前に来る
        let v = serde_json::json!({ "€": 1, "😀": 2, "a": 3, "A": 4 });
        assert_eq!(canonicalize(&v), "{\"A\":4,\"a\":3,\"€\":1,\"😀\":2}");
    }
}
