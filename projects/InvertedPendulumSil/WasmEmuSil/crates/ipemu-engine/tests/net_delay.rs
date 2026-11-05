//! 通信路を制御ループに入れた結合テスト (M6: NetworkedSil の遅延実験との比較, Docs/10-delay-comparison.md)。
//!
//! `dev/cartpole-netdelay.manifest.json` (MCU ↔ 駆動系の信号をすべて comm-link に通す構成, D60 / D61) を仮想時間で回し、
//! 往復遅延 (RTT) を振って「倒立を維持できる / 倒れる」の境目を求める。NetworkedSil の
//! `tests/Ip.Shared.Tests/IntegrationTests.cs` (`DelayedLoopIntegrationTests`) と同じ条件にそろえてある:
//! 振子 0.6 m・帰還周期 5 ms・倒立付近 (θ=0.02 rad) から balance で開始・2.03 s から 2 s ごとに
//! 0.3 Nm × 50 ms の外乱を交互の向きに加える (D62)・12 s 回す。片道遅延 = RTT / 2 を上り下りの全リンクに与える。
//!
//! 転倒の判定 (D63。どれか 1 つでも起きたら「倒れた」):
//! - ファームウェアが FAULT に落ちた (ログ `-> Fault`。転倒検知 |θ̂|>0.6 rad・ソフトリミット・ドライブ FAULT 入力)
//! - プラントの真の振子角が |θ| >= 0.6 rad (`FALL_ANGLE_RAD`、NetworkedSil の `FallAngleRad` と同じ)
//! - 台車がメカストッパに触れた (`plant.hit-stopper`)
//!
//! 部品の wasm のコンパイル (debug ビルドで 1 回 5 秒ほど) を繰り返さないよう、1 つの Session を
//! comm-link のパラメータ変更 (`set_param` で再 init) + `reset` で使い回す。
//!
//! 前提: `scripts/build-components.sh` と `scripts/build-firmware.sh` で build/ に成果物があること (無ければ skip)。
//!
//! 数字の再生成 (Docs/10 の表。release で 2 分ほど):
//! `cargo test --release -p ipemu-engine --test net_delay -- --ignored --nocapture print_delay_report`

use ipemu_core::hal::LogLevel;
use ipemu_engine::{FirmwareSource, Session, SessionOptions};
use std::path::{Path, PathBuf};

/// NetworkedSil の `ControllerCore.FallAngleRad` と同じ。
const FALL_ANGLE_RAD: f64 = 0.6;
/// 外乱を加えながら回す時間 [s] (NetworkedSil の `RunWithDisturbances` の既定 12000 ms)。
const RUN_SECONDS: f64 = 12.0;
/// 振子 0.6 m・周期 5 ms・LPF 25 Hz の理論遅延余裕 [ms]。NetworkedSil の表の値で、Rust 移植
/// (`ip_model::delay_margin`) も同じ値を出す (`crates/ip-model/tests/delay_margin.rs`)。ここでも計算して確かめる。
const THEORY_MARGIN_MS: u32 = 85;
/// manifest のファームウェア自動開始 [ms] (`autoStartDelayMs`)。これ以下の片道遅延なら、
/// 制御側は最初の本物の計測が届いてから balance を始める (D61)。
const AUTO_START_MS: u32 = 60;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn manifest_path() -> PathBuf {
    root().join("dev/cartpole-netdelay.manifest.json")
}

fn artifacts_ready() -> bool {
    let need = [
        "build/components/plant-cartpole.wasm",
        "build/components/motor-driver.wasm",
        "build/components/encoder.wasm",
        "build/components/comm-link.wasm",
        "build/firmware/fw-balance-c.wasm",
        "build/firmware/fw-balance-rs.wasm",
    ];
    let missing: Vec<_> = need.iter().filter(|p| !root().join(p).exists()).collect();
    if !missing.is_empty() {
        eprintln!("SKIP: ビルド成果物がありません ({missing:?})。scripts/build-components.sh と scripts/build-firmware.sh を先に実行してください");
        return false;
    }
    true
}

/// 通信路の条件。上りと下りの全リンクに同じ値を与える (片道遅延 = RTT / 2)。
#[derive(Debug, Clone, Copy)]
struct Net {
    /// 片道遅延 [ms] (comm-link の `delayMs`)。1 以上 (D60: 0 でも 1 ステップ遅れる)
    one_way_ms: u32,
    jitter_ms: f64,
    spike_percent: f64,
    /// comm-link の `sampleEveryMs`。既定は 1 (D60)
    sample_every_ms: f64,
}

impl Net {
    fn delay(one_way_ms: u32) -> Self {
        Self {
            one_way_ms,
            jitter_ms: 0.0,
            spike_percent: 0.0,
            sample_every_ms: 1.0,
        }
    }
    fn with_delay(self, one_way_ms: u32) -> Self {
        Self { one_way_ms, ..self }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fw {
    C,
    Rust,
}

#[derive(Debug, Clone)]
struct Outcome {
    fell: bool,
    /// 倒れたと判定した時刻 [s] と理由
    fell_at_s: f64,
    reason: String,
    /// 実行中の最大 |θ| [rad] (倒れたらその時点まで)
    max_abs_theta: f64,
}

/// dev manifest の artifact の path を絶対パスにし、FW の自動開始時刻を `start_ms` にした manifest を `dir` に書く。
fn write_variant(dir: &Path, start_ms: Option<u32>) -> PathBuf {
    let text = std::fs::read_to_string(manifest_path()).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    // path は manifest 相対なので絶対パスにする (UI バンドルなど未ビルドのものはそのまま結合)
    for (_, a) in v["artifacts"].as_object_mut().unwrap() {
        let p = a["path"].as_str().unwrap().to_owned();
        let joined = root().join("dev").join(p);
        a["path"] = serde_json::json!(joined.canonicalize().unwrap_or(joined).to_string_lossy());
    }
    for c in v["components"].as_array_mut().unwrap() {
        if c["instance"] == "mcu" {
            let p = &mut c["firmware"]["params"];
            assert_eq!(p["autoStartDelayMs"], AUTO_START_MS, "manifest の自動開始");
            if let Some(ms) = start_ms {
                p["autoStartDelayMs"] = serde_json::json!(ms);
            }
        }
    }
    let path = dir.join(format!("netdelay-{start_ms:?}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    path
}

/// 1 つの Session を使い回す試験台。
struct Rig {
    s: Session,
    links: Vec<String>,
    theta: (usize, usize),
    stopper: (usize, usize),
    start_ms: u32,
    _dir: tempfile::TempDir,
}

impl Rig {
    /// `start_ms`: FW の自動開始 [ms]。None なら manifest の値 (`AUTO_START_MS`)
    fn new(fw: Fw, start_ms: Option<u32>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = write_variant(dir.path(), start_ms);
        let mut opts = SessionOptions {
            allow_unhashed: true,
            no_consent_store: true,
            ..Default::default()
        };
        if fw == Fw::Rust {
            opts.firmware_overrides.insert(
                "balance".into(),
                FirmwareSource::Wasm(root().join("build/firmware/fw-balance-rs.wasm")),
            );
        }
        let s = Session::load(&path, &opts).expect("session");
        let links: Vec<String> = s
            .manifest
            .components
            .iter()
            .filter(|c| c.artifact.as_deref() == Some("comm-link@0.1.0"))
            .map(|c| c.instance.clone())
            .collect();
        assert_eq!(
            links.len(),
            7,
            "netdelay manifest の comm-link は上り 4 + 下り 3 本"
        );
        let g = s.engine.graph();
        let theta = g.resolve_output("plant.theta").unwrap();
        let stopper = g.resolve_output("plant.hit-stopper").unwrap();
        Self {
            s,
            links,
            theta,
            stopper,
            start_ms: start_ms.unwrap_or(AUTO_START_MS),
            _dir: dir,
        }
    }

    fn trial(&mut self, net: Net) -> Outcome {
        assert!(
            (1..=self.start_ms).contains(&net.one_way_ms),
            "片道遅延 {} ms は 1..={} ms (自動開始より後に最初の計測が届くと、初期値 0 カウントで制御を始めてしまう)",
            net.one_way_ms,
            self.start_ms
        );
        for link in self.links.clone() {
            for (k, v) in [
                ("delayMs", serde_json::json!(net.one_way_ms)),
                ("jitterMs", serde_json::json!(net.jitter_ms)),
                ("spikePercent", serde_json::json!(net.spike_percent)),
                ("sampleEveryMs", serde_json::json!(net.sample_every_ms)),
            ] {
                self.s
                    .set_param(&link, k, v)
                    .expect("comm-link の set_param");
            }
        }
        self.s.reset().expect("reset");
        let _ = self.s.engine.drain_logs();
        let steps = (RUN_SECONDS * 1e9 / self.s.engine.base_step_ns() as f64).round() as u64;
        let mut max_abs_theta: f64 = 0.0;
        for _ in 0..steps {
            self.s.step().expect("step");
            let theta =
                ip_model::math::wrap_angle(self.s.engine.output(self.theta.0, self.theta.1));
            max_abs_theta = max_abs_theta.max(theta.abs());
            let mut reason = None;
            for (inst, lvl, msg) in self.s.engine.drain_logs() {
                if inst == "mcu" && lvl == LogLevel::Error && msg.contains("-> Fault") {
                    reason = Some(format!("FW FAULT: {msg}"));
                }
            }
            if reason.is_none() && theta.abs() >= FALL_ANGLE_RAD {
                reason = Some(format!("|θ|={:.3} rad", theta.abs()));
            }
            if reason.is_none() && self.s.engine.output(self.stopper.0, self.stopper.1) > 0.5 {
                reason = Some("メカストッパに接触".into());
            }
            if let Some(reason) = reason {
                return Outcome {
                    fell: true,
                    fell_at_s: self.s.engine.time_ns() as f64 * 1e-9,
                    reason,
                    max_abs_theta,
                };
            }
        }
        Outcome {
            fell: false,
            fell_at_s: f64::NAN,
            reason: String::new(),
            max_abs_theta,
        }
    }

    /// 片道遅延を `grid` (昇順) で振り、最初に倒れた点の直前 (維持できた最大の片道遅延) を返す。
    /// `monotone` なら「遅延を増やすと助かる」ことがない (倒れた点より大きい遅延で維持しない) ことも確かめる。
    fn sweep(
        &mut self,
        grid: &[u32],
        base: Net,
        monotone: bool,
    ) -> (Option<u32>, Vec<(u32, Outcome)>) {
        let results: Vec<(u32, Outcome)> = grid
            .iter()
            .map(|&d| (d, self.trial(base.with_delay(d))))
            .collect();
        let first_fall = results.iter().position(|(_, o)| o.fell);
        if let (true, Some(i)) = (monotone, first_fall) {
            let revived: Vec<_> = results[i..]
                .iter()
                .filter(|(_, o)| !o.fell)
                .map(|(d, _)| 2 * d)
                .collect();
            assert!(
                revived.is_empty(),
                "RTT {} ms で倒れたのに、より長い RTT {revived:?} ms で維持した (遅延が増えて助かった)\n{}",
                2 * results[i].0,
                describe(&results)
            );
        }
        let last_keep = match first_fall {
            Some(0) => None,
            Some(i) => Some(results[i - 1].0),
            None => results.last().map(|(d, _)| *d),
        };
        (last_keep, results)
    }

    /// 境目を片道 1 ms (RTT 2 ms) の分解能まで二分探索する。`keep` で維持、`fall` で倒れることが前提。
    fn bisect(&mut self, mut keep: u32, mut fall: u32, base: Net) -> (u32, u32) {
        while fall - keep > 1 {
            let mid = (keep + fall) / 2;
            if self.trial(base.with_delay(mid)).fell {
                fall = mid;
            } else {
                keep = mid;
            }
        }
        (keep, fall)
    }
}

fn describe(results: &[(u32, Outcome)]) -> String {
    results
        .iter()
        .map(|(d, o)| {
            if o.fell {
                format!(
                    "RTT {:>3} ms: 転倒 t={:.2}s ({})",
                    2 * d,
                    o.fell_at_s,
                    o.reason
                )
            } else {
                format!(
                    "RTT {:>3} ms: 維持 (max|θ|={:.3} rad)",
                    2 * d,
                    o.max_abs_theta
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn theory_margin_ms() -> u32 {
    let d = ip_model::delay_margin::design(
        &ip_model::Params::default(),
        5,
        ip_model::delay_margin::DEFAULT_CUTOFF_HZ,
    )
    .unwrap();
    assert_eq!(
        d.margin_ms, THEORY_MARGIN_MS,
        "Rust の理論遅延余裕が表と違う"
    );
    d.margin_ms
}

/// C 版 FW (manifest の既定): 境目のはっきり下 (RTT 60 / 80 ms) で維持し、はっきり上 (RTT 100 / 120 ms) で倒れる。
/// RTT 80〜100 ms の間を RTT 2 ms の分解能で絞り、境目 (維持できた最大の RTT) が理論遅延余裕 ±15 ms に入ることを確かめる
/// (±15 ms は NetworkedSil の `MeasuredBoundary_AgreesWithTheTheoreticalDelayMargin` と同じ幅)。
/// 実測は RTT 88 ms 維持 / 90 ms 転倒 (Docs/10)。
#[test]
fn c_firmware_delay_boundary_agrees_with_theory() {
    if !artifacts_ready() {
        return;
    }
    let theory = theory_margin_ms();
    let mut rig = Rig::new(Fw::C, None);
    let (last_keep, results) = rig.sweep(&[30, 40, 50, 60], Net::delay(1), true);
    eprintln!("C 版 FW:\n{}", describe(&results));
    let fell = |rtt: u32| results.iter().find(|(d, _)| 2 * d == rtt).unwrap().1.fell;
    assert!(
        !fell(60) && !fell(80),
        "境目より十分短い RTT で倒れた\n{}",
        describe(&results)
    );
    assert!(
        fell(100) && fell(120),
        "境目より十分長い RTT で倒れなかった\n{}",
        describe(&results)
    );
    assert_eq!(last_keep, Some(40));
    let (keep, fall) = rig.bisect(40, 50, Net::delay(1));
    let boundary = 2 * keep;
    eprintln!(
        "C 版 FW の境目: RTT {boundary} ms 維持 / {} ms 転倒 (理論 {theory} ms)",
        2 * fall
    );
    assert!(
        boundary.abs_diff(theory) <= 15,
        "境目 RTT {boundary} ms が理論 {theory} ms ± 15 ms を外れた"
    );
}

/// Rust 版 FW も境目の両側で同じ結果になる (C 版と同じ仕様の 2 実装)。
#[test]
fn rust_firmware_keeps_at_80_and_falls_at_100() {
    if !artifacts_ready() {
        return;
    }
    let mut rig = Rig::new(Fw::Rust, None);
    let keep = rig.trial(Net::delay(40));
    let fall = rig.trial(Net::delay(50));
    assert!(!keep.fell, "Rust 版 FW が RTT 80 ms で倒れた: {keep:?}");
    assert!(
        fall.fell,
        "Rust 版 FW が RTT 100 ms で倒れなかった: {fall:?}"
    );
}

/// Docs/10 の表を出す (重いので既定では走らせない)。
/// `cargo test --release -p ipemu-engine --test net_delay -- --ignored --nocapture print_delay_report`
#[test]
#[ignore]
fn print_delay_report() {
    if !artifacts_ready() {
        return;
    }
    println!("理論遅延余裕 (Rust 移植): {} ms", theory_margin_ms());
    let grid: Vec<u32> = (5..=60).step_by(5).collect();
    for fw in [Fw::C, Fw::Rust] {
        let mut rig = Rig::new(fw, None);
        let (last_keep, results) = rig.sweep(&grid, Net::delay(1), true);
        println!(
            "== {fw:?} 版 FW, ジッタ・スパイクなし ==\n{}",
            describe(&results)
        );
        if let Some(k) = last_keep.filter(|&k| k < AUTO_START_MS) {
            let (k, f) = rig.bisect(k, k + 5, Net::delay(1));
            println!("境目: RTT {} ms 維持 / {} ms 転倒", 2 * k, 2 * f);
        }
    }

    // 感度: 自動開始を「最初の計測が届いた直後」(片道遅延、ただし 30 ms 以上) にした場合 (NetworkedSil の開始に近い)
    for d in [42u32, 43, 44, 45] {
        let start = d.max(30);
        let o = Rig::new(Fw::C, Some(start)).trial(Net::delay(d));
        println!(
            "自動開始 {start} ms: RTT {} ms → {}",
            2 * d,
            if o.fell { "転倒" } else { "維持" }
        );
    }

    // 観察: ジッタ (各リンクで一様 [0, J) ms) と再送スパイク (+150 ms) で、維持できる基準遅延がどう動くか (C 版 FW)。
    // スパイク率は「1 秒あたりの回数」をそろえる (sampleEvery 1 ms と 5 ms で 1 サンプルあたりの確率を変える)
    let mut rig = Rig::new(Fw::C, None);
    for (jitter, spikes_per_s) in [
        (5.0, 0.0),
        (10.0, 0.0),
        (20.0, 0.0),
        (0.0, 0.1),
        (0.0, 0.5),
        (0.0, 2.0),
    ] {
        for sample_every_ms in [1.0, 5.0] {
            let base = Net {
                one_way_ms: 1,
                jitter_ms: jitter,
                // 1 サンプルあたりの確率 [%] = 回数/s × 周期 [s] × 100
                spike_percent: spikes_per_s * sample_every_ms * 1e-3 * 100.0,
                sample_every_ms,
            };
            // 乱数で決まるので単調とは限らない (観察のみ)。二分探索は最初に倒れた格子点の手前で行う
            let (last_keep, results) = rig.sweep(&grid, base, false);
            let keep = last_keep.map(|k| {
                if k < AUTO_START_MS {
                    rig.bisect(k, k + 5, base).0
                } else {
                    k
                }
            });
            let fall_rtts: Vec<u32> = results
                .iter()
                .filter(|(_, o)| o.fell)
                .map(|(d, _)| 2 * d)
                .collect();
            let first_falls: Vec<String> = results
                .iter()
                .filter(|(_, o)| o.fell)
                .take(1)
                .map(|(d, o)| format!("RTT {} ms で t={:.2}s ({})", 2 * d, o.fell_at_s, o.reason))
                .collect();
            println!(
                "jitter {jitter} ms, spike {spikes_per_s}/s ({:.3}%/sample), sampleEvery {sample_every_ms} ms: 境目 (最初の転倒の手前で維持できた基準 RTT) {:?} ms; 転倒した格子点 {fall_rtts:?}; 最初の転倒 {first_falls:?}",
                base.spike_percent,
                keep.map(|k| 2 * k)
            );
        }
    }
}
