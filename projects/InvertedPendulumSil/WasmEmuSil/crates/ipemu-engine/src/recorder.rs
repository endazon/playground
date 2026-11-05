//! スナップショット列の記録 (CSV) と、決定性テスト用のハッシュ。

use sha2::{Digest, Sha256};
use std::io::Write;

/// 記録した値のハッシュ。f64 のビット列を順に流し込む。
#[derive(Default)]
pub struct TraceHash {
    hasher: Sha256,
    rows: u64,
}

impl TraceHash {
    pub fn push_row(&mut self, t_ns: i64, values: &[f64]) {
        self.hasher.update(t_ns.to_le_bytes());
        for v in values {
            self.hasher.update(v.to_bits().to_le_bytes());
        }
        self.rows += 1;
    }
    pub fn finish(self) -> String {
        format!("{}:{}", self.rows, hex::encode(self.hasher.finalize()))
    }
}

/// CSV 記録。
pub struct Recorder<W: Write> {
    out: W,
    columns: Vec<String>,
}

impl<W: Write> Recorder<W> {
    pub fn new(mut out: W, columns: &[String]) -> std::io::Result<Self> {
        write!(out, "t_s")?;
        for c in columns {
            write!(out, ",{c}")?;
        }
        writeln!(out)?;
        Ok(Self {
            out,
            columns: columns.to_vec(),
        })
    }
    pub fn row(&mut self, t_ns: i64, values: &[f64]) -> std::io::Result<()> {
        debug_assert_eq!(values.len(), self.columns.len());
        write!(self.out, "{:.6}", t_ns as f64 * 1e-9)?;
        for v in values {
            // 往復で同じ double に戻る表現 (Rust の Display は最短往復表現)
            write!(self.out, ",{v}")?;
        }
        writeln!(self.out)
    }
    pub fn into_inner(self) -> W {
        self.out
    }
}
