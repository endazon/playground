//! フラットな JSON から数値 / 文字列を引く極小パーサ。
//! serde を使わないのは、C 版と同じ制約 (ヒープ・依存を増やさない) で書くため。

pub fn number(json: &str, key: &str, default: f64) -> f64 {
    let Some(rest) = find_value(json, key) else {
        return default;
    };
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E')))
        .unwrap_or(rest.len());
    rest[..end].parse::<f64>().unwrap_or(default)
}

pub fn string<'a>(json: &'a str, key: &str, default: &'a str) -> &'a str {
    let Some(rest) = find_value(json, key) else {
        return default;
    };
    let rest = rest.strip_prefix('"').unwrap_or(rest);
    match rest.find('"') {
        Some(i) => &rest[..i],
        None => default,
    }
}

/// `"key"` の直後の `:` と空白を飛ばした位置からの残りを返す。
fn find_value<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let mut search = json;
    loop {
        let i = search.find('"')?;
        let after = &search[i + 1..];
        let j = after.find('"')?;
        let found = &after[..j];
        let rest = &after[j + 1..];
        if found == key {
            let rest = rest.trim_start();
            let rest = rest.strip_prefix(':')?;
            return Some(rest.trim_start());
        }
        // 値側の文字列を飛ばす
        search = rest;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_flat_json() {
        let j = r#"{"a": 1.5, "name": "swingup", "b":-2e-3, "c" : 7}"#;
        assert_eq!(number(j, "a", 0.0), 1.5);
        assert_eq!(number(j, "b", 0.0), -2e-3);
        assert_eq!(number(j, "c", 0.0), 7.0);
        assert_eq!(number(j, "zzz", 9.0), 9.0);
        assert_eq!(string(j, "name", "x"), "swingup");
    }
}
