//! 実行時間モデル fuel の性質テスト (M6)。実物のファームウェア (C 版 / Rust 版の wasm) で確かめる。
//!
//! fuel の絶対値はコンパイラの版で変わるので、値そのものは固定せず関係だけを見る:
//! 決定性、`exec-ns = round(fuel × nsPerFuel)`、オーバーランと出力の遅れ、fuel 上限による決定的な trap。
//! 前提: `scripts/build-components.sh` と `scripts/build-firmware.sh` (無ければ理由を出して skip)。

use ipemu_core::builtins::{CpuVirtual, CpuVirtualParams};
use ipemu_core::hal::{Firmware, FuelLimits, HalRegs, LogLevel, TickStats};
use ipemu_core::Part;
use ipemu_engine::{FirmwareSource, Session, SessionOptions};
use ipemu_wasm::runtime::RuntimeOptions;
use ipemu_wasm::{Runtime, WasmFirmware};
use serde_json::json;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn manifest() -> PathBuf {
    root().join("dev/cartpole.manifest.json")
}

const FIRMWARES: [&str; 2] = ["fw-balance-c", "fw-balance-rs"];

fn firmware_wasm(name: &str) -> Option<PathBuf> {
    let p = root().join(format!("build/firmware/{name}.wasm"));
    if p.exists() {
        Some(p)
    } else {
        eprintln!(
            "SKIP: {} がありません。scripts/build-firmware.sh を先に実行してください",
            p.display()
        );
        None
    }
}

fn components_ready() -> bool {
    let ok = ["plant-cartpole", "motor-driver", "encoder", "comm-link"]
        .iter()
        .all(|c| root().join(format!("build/components/{c}.wasm")).exists());
    if !ok {
        eprintln!("SKIP: build/components がありません。scripts/build-components.sh を先に実行してください");
    }
    ok
}

/// dev manifest の mcu と同じ cfg JSON (session.rs の firmware_cfg_json と同じく tickHz を足す)。
fn firmware_cfg() -> String {
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest()).unwrap()).unwrap();
    let mcu = v["components"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["instance"] == "mcu")
        .unwrap();
    let mut p = mcu["firmware"]["params"].clone();
    p["tickHz"] = mcu["params"]["tickHz"].clone();
    serde_json::to_string(&p).unwrap()
}

/// tick ごとの fuel を記録する包み。委譲するだけで値は変えない。
struct Recording {
    inner: Box<dyn Firmware>,
    fuel: Rc<RefCell<Vec<Option<u64>>>>,
}

impl Firmware for Recording {
    fn abi_version(&mut self) -> anyhow::Result<u32> {
        self.inner.abi_version()
    }
    fn init(&mut self, regs: &mut HalRegs, cfg: &str) -> anyhow::Result<()> {
        self.inner.init(regs, cfg)
    }
    fn tick(&mut self, regs: &mut HalRegs) -> anyhow::Result<TickStats> {
        let s = self.inner.tick(regs)?;
        self.fuel.borrow_mut().push(s.fuel);
        Ok(s)
    }
    fn irq(&mut self, regs: &mut HalRegs, id: u32) -> anyhow::Result<()> {
        self.inner.irq(regs, id)
    }
    fn flavor(&self) -> &'static str {
        self.inner.flavor()
    }
    fn meters_fuel(&self) -> bool {
        self.inner.meters_fuel()
    }
    fn set_fuel_limits(&mut self, limits: FuelLimits) {
        self.inner.set_fuel_limits(limits)
    }
}

/// プラントなしで cpu-virtual を 1 ms ステップ (= 1 tick) ずつ回した結果。
struct Bench {
    /// tick ごとの fuel
    fuel: Vec<u64>,
    /// ステップ後の出力 (pwm0, exec-ns, overruns)
    pwm: Vec<f64>,
    exec: Vec<f64>,
    overruns: f64,
}

/// 入力は合成の振り子 (enc1 を正弦波で振る)。出力を入力に戻さないので、
/// ファームウェアが見る入力列は実行時間モデルによらず同じになる。
fn bench(fw_path: &Path, timing: serde_json::Value, ticks: usize) -> Bench {
    let rt = Runtime::new(RuntimeOptions::default()).unwrap();
    let params =
        CpuVirtualParams::from_json(&json!({"tickHz": 1000, "timingModel": timing})).unwrap();
    let period = params.tick_period_ns();
    assert_eq!(period, 1_000_000);
    let mut cpu = CpuVirtual::new(params, firmware_cfg());
    let fuel = Rc::new(RefCell::new(Vec::new()));
    let fw = WasmFirmware::load_file(&rt, fw_path).unwrap();
    cpu.attach_firmware(Box::new(Recording {
        inner: Box::new(fw),
        fuel: fuel.clone(),
    }))
    .unwrap();
    let desc = cpu.describe().clone();
    let (pwm0, exec, over) = (
        desc.output_index("pwm0").unwrap(),
        desc.output_index("exec-ns").unwrap(),
        desc.output_index("overruns").unwrap(),
    );
    let mut out = vec![0.0; desc.outputs.len()];
    let mut b = Bench {
        fuel: Vec::new(),
        pwm: Vec::new(),
        exec: Vec::new(),
        overruns: 0.0,
    };
    for k in 0..ticks {
        let t = k as f64 * 1e-3;
        // enc0 (台車), enc1 (振り子 4096 cpr で ±0.3 rad 程度), gpio-in0, gpio-in1
        let theta = std::f64::consts::PI + 0.3 * (2.0 * std::f64::consts::PI * 0.8 * t).sin();
        let enc1 = (theta / (2.0 * std::f64::consts::PI) * 4096.0).round();
        let enc0 = (200.0 * (2.0 * std::f64::consts::PI * 0.3 * t).sin()).round();
        cpu.set_inputs(&[enc0, enc1, 0.0, 0.0]).unwrap();
        cpu.step(period).unwrap();
        cpu.get_outputs(&mut out).unwrap();
        b.pwm.push(out[pwm0]);
        b.exec.push(out[exec]);
        b.overruns = out[over];
    }
    b.fuel = fuel
        .borrow()
        .iter()
        .map(|f| f.expect("wasm 形は fuel を返す"))
        .collect();
    assert_eq!(b.fuel.len(), ticks);
    b
}

/// 計測値の要約 (Docs/05 の較正表に使う)。
fn summary(fuel: &[u64]) -> (u64, f64, u64) {
    let min = *fuel.iter().min().unwrap();
    let max = *fuel.iter().max().unwrap();
    let mean = fuel.iter().sum::<u64>() as f64 / fuel.len() as f64;
    (min, mean, max)
}

#[test]
fn fuel_exec_ns_sequence_is_deterministic_across_runs() {
    // (a) 同じ manifest (閉ループ、fuel モデル) を 2 回走らせて、tick ごとの mcu.exec-ns が一致する
    if !components_ready() {
        return;
    }
    for name in FIRMWARES {
        let Some(path) = firmware_wasm(name) else {
            return;
        };
        let once = || {
            let mut opts = SessionOptions {
                allow_unhashed: true,
                ..Default::default()
            };
            opts.firmware_overrides
                .insert("balance".into(), FirmwareSource::Wasm(path.clone()));
            let mut s = Session::load(&manifest(), &opts).expect("session");
            let exec = s
                .snapshot_ports
                .iter()
                .position(|c| c == "mcu.exec-ns")
                .unwrap();
            let mut buf = Vec::new();
            let mut seq = Vec::new();
            // baseStep = tick 周期 = 1 ms なので、ステップごとに読めば tick ごとの値になる
            for _ in 0..3000 {
                s.step().unwrap();
                s.snapshot_values(&mut buf);
                seq.push(buf[exec]);
            }
            seq
        };
        let a = once();
        let b = once();
        assert_eq!(a, b, "{name}: exec-ns の列が 2 回の実行で一致しない");
        assert!(
            a.iter().all(|v| *v > 0.0),
            "{name}: fuel モデルで exec-ns が 0 の tick がある"
        );
        let distinct: std::collections::BTreeSet<u64> = a.iter().map(|v| *v as u64).collect();
        assert!(
            distinct.len() > 1,
            "{name}: exec-ns が一定 (制御 tick とそれ以外で変わるはず)"
        );
    }
}

#[test]
fn exec_ns_is_fuel_times_ns_per_fuel_rounded() {
    // (b) exec-ns = round(fuel × nsPerFuel)。端数が出る係数で丸めも確かめる
    for name in FIRMWARES {
        let Some(path) = firmware_wasm(name) else {
            return;
        };
        for k in [1.0, 2.37, 0.123] {
            let b = bench(&path, json!({"kind": "fuel", "nsPerFuel": k}), 1500);
            for (i, (f, e)) in b.fuel.iter().zip(&b.exec).enumerate() {
                let want = (*f as f64 * k).round();
                assert_eq!(*e, want, "{name} k={k} tick {i}: fuel {f}");
            }
            assert_eq!(b.overruns, 0.0);
        }
        // fuel は実行時間モデルによらない (入力が同じなら同じ命令列)
        let z = bench(&path, json!({"kind": "zero"}), 1500);
        let f = bench(&path, json!({"kind": "fuel", "nsPerFuel": 2.0}), 1500);
        assert_eq!(z.fuel, f.fuel, "{name}: 実行時間モデルで fuel が変わった");
        let (min, mean, max) = summary(&z.fuel);
        eprintln!("{name}: fuel/tick min {min} mean {mean:.1} max {max} (合成入力 1500 tick)");
    }
}

/// 出力の反映を、記録した fuel から独立に計算する (tick 開始から exec 後、FIFO 順)。
/// `regs_after[k]` は tick k 直後のレジスタ像 (zero モデルでのステップ k の出力と同じ)。
fn predicted_outputs(regs_after: &[f64], exec_ns: &[i64], period: i64) -> Vec<f64> {
    fn drain(queue: &mut std::collections::VecDeque<(i64, f64)>, t: i64, shown: &mut f64) {
        while let Some(&(at, v)) = queue.front() {
            if at > t {
                break;
            }
            *shown = v;
            queue.pop_front();
        }
    }
    let mut queue = std::collections::VecDeque::new();
    let mut shown = 0.0;
    let mut out = Vec::new();
    for (k, (regs, exec)) in regs_after.iter().zip(exec_ns).enumerate() {
        let t0 = k as i64 * period;
        drain(&mut queue, t0, &mut shown);
        queue.push_back((t0 + exec, *regs));
        drain(&mut queue, t0 + period, &mut shown);
        out.push(shown);
    }
    out
}

#[test]
fn overrun_is_counted_and_outputs_are_applied_later() {
    // (c) nsPerFuel を大きくして exec-ns が tick 周期 (1 ms) を超えるようにすると、
    //     オーバーランが数えられ、PWM の変化が後のステップに現れる
    for name in FIRMWARES {
        let Some(path) = firmware_wasm(name) else {
            return;
        };
        let ticks = 1500;
        let zero = bench(&path, json!({"kind": "zero"}), ticks);
        // 最も重い tick がおよそ 1.5 周期になる係数
        let max = *zero.fuel.iter().max().unwrap() as f64;
        let k = 1.5e6 / max;
        let slow = bench(&path, json!({"kind": "fuel", "nsPerFuel": k}), ticks);
        assert_eq!(slow.fuel, zero.fuel);
        let exec: Vec<i64> = slow
            .fuel
            .iter()
            .map(|f| (*f as f64 * k).round() as i64)
            .collect();
        let over = exec.iter().filter(|e| **e > 1_000_000).count();
        assert!(over > 0, "{name}: オーバーランする tick がない (k={k})");
        assert_eq!(slow.overruns, over as f64, "{name}: オーバーラン数");
        // PWM: 記録した fuel から計算した反映時刻どおりに出る
        let want = predicted_outputs(&zero.pwm, &exec, 1_000_000);
        assert_eq!(slow.pwm, want, "{name}: 出力の反映時刻がモデルと違う");
        // 遅れが観測できる: zero モデルで PWM が最初に変わったステップより後に変わる
        let first_change = |v: &[f64]| v.iter().position(|x| *x != v[0]).expect("PWM が変化しない");
        let (z0, s0) = (first_change(&zero.pwm), first_change(&slow.pwm));
        assert!(
            s0 > z0,
            "{name}: PWM の変化が遅れていない (zero {z0}, fuel {s0})"
        );
        // 遅れのない係数ではオーバーランも遅れもない (比較の基準)
        let fast = bench(&path, json!({"kind": "fuel", "nsPerFuel": 2.0}), ticks);
        assert_eq!(fast.overruns, 0.0);
        assert_eq!(fast.pwm, zero.pwm);
        eprintln!("{name}: k={k:.1} ns/fuel で {over}/{ticks} tick がオーバーラン、最初の PWM 変化 {z0} → {s0} ms");
    }
}

/// ABI 1 を名乗り、`tick` を 3 回目まではすぐ返し、4 回目で無限ループに入るファームウェア (WAT)。
/// `INIT_SPIN` を置き換えると init も無限ループにできる。
const RUNAWAY_WAT: &str = r#"
(component
  (core module $m
    (memory (export "memory") 1)
    (global $ticks (mut i32) (i32.const 0))
    (global $heap (mut i32) (i32.const 1024))
    (func (export "cabi_realloc") (param i32 i32 i32 i32) (result i32)
      (local $p i32)
      (local.set $p (global.get $heap))
      (global.set $heap (i32.add (global.get $heap) (local.get 3)))
      (local.get $p))
    (func (export "abi-version") (result i32) (i32.const 1))
    (func (export "init") (param i32 i32) (result i32)
      (local $n i32)
      INIT_SPIN
      (i32.const 0))
    (func (export "tick")
      (local $n i32)
      (global.set $ticks (i32.add (global.get $ticks) (i32.const 1)))
      (if (i32.ge_u (global.get $ticks) (i32.const 4))
        (then (loop $spin
          (local.set $n (i32.add (local.get $n) (i32.const 1)))
          (br $spin)))))
    (func (export "irq") (param i32))
    (func (export "idle")))
  (core instance $i (instantiate $m))
  (alias core export $i "memory" (core memory $mem))
  (alias core export $i "cabi_realloc" (core func $realloc))
  (func $abi (result u32) (canon lift (core func $i "abi-version")))
  (func $init (param "cfg-json" string) (result (result (error string)))
    (canon lift (core func $i "init") (memory $mem) (realloc $realloc)))
  (func $tick (canon lift (core func $i "tick")))
  (func $irq (param "id" u32) (canon lift (core func $i "irq")))
  (func $idle (canon lift (core func $i "idle")))
  (instance $fw
    (export "abi-version" (func $abi))
    (export "init" (func $init))
    (export "tick" (func $tick))
    (export "irq" (func $irq))
    (export "idle" (func $idle)))
  (export "ip:emu/fw@0.1.0" (instance $fw)))
"#;

const SPIN: &str = "(loop $s (local.set $n (i32.add (local.get $n) (i32.const 1))) (br $s))";

/// 壁時計のウォッチドッグを切った Runtime (trap が fuel 由来であることを確かめるため)。
fn runtime_without_epoch() -> Runtime {
    Runtime::new(RuntimeOptions {
        call_budget: None,
        ..Default::default()
    })
    .unwrap()
}

/// 暴走するまで回し、(trap した tick 数, エラー文, ログ) を返す。
fn run_runaway(max_fuel_per_tick: u64) -> (u64, String, Vec<(LogLevel, String)>) {
    let rt = runtime_without_epoch();
    let wat = RUNAWAY_WAT.replace("INIT_SPIN", "");
    let fw = WasmFirmware::load(&rt, wat.as_bytes()).unwrap();
    let params = CpuVirtualParams::from_json(&json!({
        "tickHz": 1000,
        "timingModel": {"kind": "fuel", "nsPerFuel": 1.0},
        "maxFuelPerTick": max_fuel_per_tick
    }))
    .unwrap();
    let mut cpu = CpuVirtual::new(params, "{}".into());
    cpu.attach_firmware(Box::new(fw)).unwrap();
    let inputs = vec![0.0; cpu.describe().inputs.len()];
    let mut err = None;
    for _ in 0..10 {
        cpu.set_inputs(&inputs).unwrap();
        if let Err(e) = cpu.step(1_000_000) {
            err = Some(format!("{e:#}"));
            break;
        }
    }
    let ticks = cpu.tick_count();
    // FAULT 後はファームウェアを呼ばずに時間だけ進む (既存の trap 経路)
    cpu.set_inputs(&inputs).unwrap();
    cpu.step(1_000_000).expect("FAULT 後の step は失敗しない");
    assert_eq!(cpu.tick_count(), ticks + 1);
    (
        ticks,
        err.expect("暴走が trap しなかった"),
        cpu.drain_logs(),
    )
}

#[test]
fn fuel_budget_traps_runaway_firmware_deterministically() {
    // (d) 無限ループを、壁時計ではなく fuel の上限で、毎回同じ tick で止める
    let a = run_runaway(10_000);
    let b = run_runaway(10_000);
    assert_eq!(a.0, 3, "4 回目の tick (tick-count 3 の時点) で trap する");
    assert_eq!((a.0, &a.1), (b.0, &b.1), "2 回の実行で trap の位置が違う");
    assert!(a.1.contains("maxFuelPerTick=10000"), "{}", a.1);
    assert!(
        a.2.iter()
            .any(|(l, m)| *l == LogLevel::Error && m.contains("firmware.tick が失敗")),
        "{:?}",
        a.2
    );
    // init の暴走は maxFuelPerInit で止まり、装着が失敗する (cpu-virtual は FAULT にならず起動不可)
    let rt = runtime_without_epoch();
    let wat = RUNAWAY_WAT.replace("INIT_SPIN", SPIN);
    let fw = WasmFirmware::load(&rt, wat.as_bytes()).unwrap();
    let params =
        CpuVirtualParams::from_json(&json!({"tickHz": 1000, "maxFuelPerInit": 50_000})).unwrap();
    let mut cpu = CpuVirtual::new(params, "{}".into());
    let e = format!("{:#}", cpu.attach_firmware(Box::new(fw)).unwrap_err());
    assert!(e.contains("maxFuelPerInit=50000"), "{e}");
}

#[test]
fn real_firmware_under_tiny_budget_faults_the_session() {
    // 実物のファームウェアでも、予算が足りなければ Session の step が trap で失敗する
    // (制御器はこのエラーで FAULT に入る)。予算が十分なら走り切る
    if !components_ready() {
        return;
    }
    for name in FIRMWARES {
        let Some(path) = firmware_wasm(name) else {
            return;
        };
        let attempt = |budget: u64| {
            let mut opts = SessionOptions {
                allow_unhashed: true,
                ..Default::default()
            };
            opts.firmware_overrides
                .insert("balance".into(), FirmwareSource::Wasm(path.clone()));
            opts.param_overrides
                .push(("mcu".into(), "maxFuelPerTick".into(), json!(budget)));
            let mut s = Session::load(&manifest(), &opts).expect("session");
            let mut steps = 0u32;
            for _ in 0..2000 {
                if let Err(e) = s.step() {
                    return Err((steps, format!("{e:#}")));
                }
                steps += 1;
            }
            Ok(steps)
        };
        let small = attempt(50).unwrap_err();
        assert!(small.1.contains("maxFuelPerTick=50"), "{name}: {}", small.1);
        assert_eq!(
            attempt(50).unwrap_err(),
            small,
            "{name}: trap の位置が実行ごとに違う"
        );
        assert_eq!(
            attempt(10_000_000),
            Ok(2000),
            "{name}: 十分な予算で trap した"
        );
    }
}

#[test]
fn native_firmware_under_fuel_model_warns_once() {
    // native 形は fuel を数えない。fuel モデル (dev manifest の既定) で装着したら警告を 1 回だけ出す
    if !components_ready() {
        return;
    }
    let ext = if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    };
    let p = root().join(format!("build/firmware/fw-balance-c.{ext}"));
    if !p.exists() {
        eprintln!("SKIP: {} がありません", p.display());
        return;
    }
    let opts = SessionOptions {
        allow_unhashed: true,
        firmware_overrides: [("balance".to_string(), FirmwareSource::Native(p))].into(),
        ..Default::default()
    };
    let mut s = Session::load(&manifest(), &opts).expect("session");
    let mut warns = Vec::new();
    let exec = s
        .snapshot_ports
        .iter()
        .position(|c| c == "mcu.exec-ns")
        .unwrap();
    let mut buf = Vec::new();
    for _ in 0..200 {
        s.step().unwrap();
        s.snapshot_values(&mut buf);
        assert_eq!(buf[exec], 0.0, "native 形の exec-ns は 0");
        warns.extend(s.engine.drain_logs().into_iter().filter(|(i, l, m)| {
            i == "mcu" && *l == LogLevel::Warn && m.contains("fuel を計測しません")
        }));
    }
    assert_eq!(warns.len(), 1, "{warns:?}");
}
