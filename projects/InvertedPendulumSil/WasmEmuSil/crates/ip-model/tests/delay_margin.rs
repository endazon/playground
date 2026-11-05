//! 理論遅延余裕 (`ip_model::delay_margin`) の検算 (M6, Docs/10-delay-comparison.md)。
//!
//! NetworkedSil の `tests/Ip.Shared.Tests/DelayMarginTests.cs` と同じ表・同じ許容誤差 (±2ms)・同じ単調性を確かめる。
//! 表は NetworkedSil の `Docs/03-delay-margin.md` に載っている値そのもの (C# 実装の出力) で、
//! ここが通れば Rust 移植が C# 版と同じ数字を出すことになる。
//!
//! 表の再生成 (Docs/10 に貼る数字): `cargo test -p ip-model --test delay_margin -- --nocapture print_table`

use ip_model::delay_margin::{design, Analyzer, DEFAULT_CUTOFF_HZ, MAX_DELAY_MS};
use ip_model::lqr::linearize;
use ip_model::Params;

const PERIODS_MS: [u32; 5] = [1, 5, 20, 50, 100];
const LENGTHS_M: [f64; 3] = [0.3, 0.6, 1.0];

/// NetworkedSil の設計メモ (Docs/03-delay-margin.md) の表。行 = 振子長、列 = 周期 1/5/20/50/100 ms。
const NETWORKED_SIL_TABLE: [[u32; 5]; 3] = [
    [58, 57, 52, 41, 35],
    [86, 85, 80, 68, 59],
    [110, 109, 105, 95, 86],
];
/// 同じく不安定極 [rad/s] (表は小数 1 桁)。
const NETWORKED_SIL_POLES: [f64; 3] = [7.5, 5.3, 4.1];

fn params(length: f64) -> Params {
    Params {
        pendulum_length: length,
        ..Params::default()
    }
}

fn margin(length: f64, period_ms: u32) -> u32 {
    design(&params(length), period_ms, DEFAULT_CUTOFF_HZ)
        .expect("LQR 設計")
        .margin_ms
}

/// 設計メモに貼る表を出力する。Docs/10 の数字はここから取る。
#[test]
fn print_table() {
    println!(
        "振子長 | 不安定極 | {}",
        PERIODS_MS.map(|p| format!("{p}ms")).join(" | ")
    );
    for length in LENGTHS_M {
        let values = PERIODS_MS.map(|p| margin(length, p));
        let pole = linearize(&params(length)).unstable_pole;
        println!(
            "{length:.1} m | {pole:.1} rad/s | {}",
            values.map(|v| v.to_string()).join(" | ")
        );
        assert!(
            values.iter().all(|&m| m > 0),
            "遅延ゼロでも安定化できない設計になっている"
        );
    }
}

#[test]
fn delay_margin_matches_the_networked_sil_table() {
    let mut mismatches = Vec::new();
    for (row, length) in LENGTHS_M.iter().enumerate() {
        for (col, period) in PERIODS_MS.iter().enumerate() {
            let expected = NETWORKED_SIL_TABLE[row][col];
            let got = margin(*length, *period);
            if got.abs_diff(expected) > 2 {
                mismatches.push(format!(
                    "{length} m / {period} ms: {got} ms (NetworkedSil {expected} ms)"
                ));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "NetworkedSil の表から ±2ms を外れた: {mismatches:?}"
    );
}

#[test]
fn unstable_pole_matches_the_networked_sil_table() {
    for (length, expected) in LENGTHS_M.iter().zip(NETWORKED_SIL_POLES) {
        let pole = linearize(&params(*length)).unstable_pole;
        assert!(
            (pole - expected).abs() <= 0.05,
            "{length} m: 不安定極 {pole} rad/s (NetworkedSil {expected})"
        );
    }
}

#[test]
fn delay_margin_shrinks_as_the_pendulum_gets_shorter() {
    let long_rod = margin(1.0, 5);
    let medium_rod = margin(0.6, 5);
    let short_rod = margin(0.3, 5);
    assert!(long_rod > medium_rod, "{long_rod} <= {medium_rod}");
    assert!(medium_rod > short_rod, "{medium_rod} <= {short_rod}");
}

#[test]
fn delay_margin_shrinks_as_the_feedback_period_grows() {
    let fast = margin(0.6, 1);
    let slow = margin(0.6, 100);
    assert!(
        fast > slow,
        "周期を伸ばしても余裕が減っていない: {fast} → {slow}"
    );
}

/// 二分探索の前提 (安定 / 不安定の境目が 1 つ) を、0.6 m / 5 ms で遅延を格子状に振って確かめる。
/// 境目より下はすべて安定、上はすべて不安定 (遅延が増えて安定に戻ることはない)。
#[test]
fn stability_switches_once_at_the_margin() {
    let p = params(0.6);
    let d = design(&p, 5, DEFAULT_CUTOFF_HZ).unwrap();
    let lin = linearize(&p);
    let analyzer = Analyzer::new(&lin.a, &lin.b, &d.k, 0.005, DEFAULT_CUTOFF_HZ);
    for delay in (0..=200).step_by(5).chain([d.margin_ms, d.margin_ms + 1]) {
        let unstable = analyzer.is_unstable(delay);
        assert_eq!(
            unstable,
            delay > d.margin_ms,
            "遅延 {delay} ms: unstable={unstable} (余裕 {} ms)",
            d.margin_ms
        );
    }
    assert!(analyzer.is_unstable(MAX_DELAY_MS));
}
