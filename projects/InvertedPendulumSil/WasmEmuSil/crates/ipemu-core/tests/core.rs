//! ipemu-core の単体テスト: 検証規則、実行順、配線の転記、仮想 MCU。

use ipemu_core::builtins::{CpuVirtual, CpuVirtualParams};
use ipemu_core::desc::{PartDesc, PortDesc};
use ipemu_core::engine::{Engine, Part};
use ipemu_core::hal::{Firmware, HalRegs, TickStats};
use ipemu_core::manifest::{ComponentSpec, Manifest};
use ipemu_core::validate::{validate, validate_static, ValidateOptions};
use serde_json::json;

/// テスト用の部品: 入力 `in` を gain 倍して `out` に出す (step ごとに 1 回)。
struct Gain {
    desc: PartDesc,
    k: f64,
    x: f64,
    steps: u64,
}
impl Gain {
    fn new(k: f64) -> Self {
        Self {
            desc: PartDesc {
                name: "gain".into(),
                version: "0".into(),
                inputs: vec![PortDesc::scalar("in", "1", "")],
                outputs: vec![
                    PortDesc::scalar("out", "1", ""),
                    PortDesc::scalar("steps", "count", ""),
                ],
                ..Default::default()
            },
            k,
            x: 0.0,
            steps: 0,
        }
    }
}
impl Part for Gain {
    fn describe(&self) -> &PartDesc {
        &self.desc
    }
    fn init(&mut self, _: &serde_json::Value) -> anyhow::Result<()> {
        Ok(())
    }
    fn reset(&mut self) -> anyhow::Result<()> {
        self.x = 0.0;
        self.steps = 0;
        Ok(())
    }
    fn set_inputs(&mut self, v: &[f64]) -> anyhow::Result<()> {
        self.x = v[0];
        Ok(())
    }
    fn step(&mut self, _dt: i64) -> anyhow::Result<()> {
        self.steps += 1;
        Ok(())
    }
    fn get_outputs(&mut self, out: &mut [f64]) -> anyhow::Result<()> {
        out[0] = self.x * self.k;
        out[1] = self.steps as f64;
        Ok(())
    }
}

/// 定数源: 出力 `out` = c
struct Source(PartDesc, f64);
impl Source {
    fn new(c: f64) -> Self {
        Self(
            PartDesc {
                name: "src".into(),
                version: "0".into(),
                outputs: vec![PortDesc::scalar("out", "1", "")],
                ..Default::default()
            },
            c,
        )
    }
}
impl Part for Source {
    fn describe(&self) -> &PartDesc {
        &self.0
    }
    fn init(&mut self, _: &serde_json::Value) -> anyhow::Result<()> {
        Ok(())
    }
    fn reset(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
    fn set_inputs(&mut self, _: &[f64]) -> anyhow::Result<()> {
        Ok(())
    }
    fn step(&mut self, _: i64) -> anyhow::Result<()> {
        Ok(())
    }
    fn get_outputs(&mut self, out: &mut [f64]) -> anyhow::Result<()> {
        out[0] = self.1;
        Ok(())
    }
}

fn manifest(wires: serde_json::Value, extra: serde_json::Value) -> Manifest {
    let mut v = json!({
        "manifestVersion": 1, "id": "t", "revision": 1, "name": "t",
        "scheduler": {"baseStepNs": 1000000},
        "artifacts": {"g@1.0.0": {"sha256": "00".repeat(32), "url": "/x"}},
        "components": [
            {"instance": "src", "artifact": "g@1.0.0"},
            {"instance": "a", "artifact": "g@1.0.0"},
            {"instance": "b", "artifact": "g@1.0.0"}
        ],
        "wires": wires
    });
    if let Some(obj) = extra.as_object() {
        for (k, val) in obj {
            v[k] = val.clone();
        }
    }
    serde_json::from_value(v).unwrap()
}

fn resolver(spec: &ComponentSpec) -> anyhow::Result<PartDesc> {
    Ok(match spec.instance.as_str() {
        "src" => Source::new(1.0).0,
        _ => Gain::new(1.0).desc,
    })
}

#[test]
fn validation_rejects_zero_latency_cycle_but_accepts_latency_one_cycle() {
    let m = manifest(
        json!([{"from": "a.out", "to": "b.in", "latency": 0}, {"from": "b.out", "to": "a.in", "latency": 0}]),
        json!({}),
    );
    let (d, g) = validate(&m, &mut resolver, ValidateOptions::default());
    assert!(g.is_none() && d.errors().any(|e| e.rule == "V6"), "{d}");

    let m = manifest(
        json!([{"from": "a.out", "to": "b.in", "latency": 0}, {"from": "b.out", "to": "a.in"}]),
        json!({}),
    );
    let (d, g) = validate(&m, &mut resolver, ValidateOptions::default());
    assert!(!d.has_errors(), "{d}");
    assert_eq!(g.unwrap().order, vec![0, 1, 2]);
}

#[test]
fn validation_detects_bad_ports_and_duplicates() {
    let m = manifest(json!([{"from": "a.nope", "to": "b.in"}]), json!({}));
    let (d, _) = validate(&m, &mut resolver, ValidateOptions::default());
    assert!(d.errors().any(|e| e.rule == "V4"), "{d}");

    let m = manifest(
        json!([{"from": "a.out", "to": "b.in"}, {"from": "src.out", "to": "b.in"}]),
        json!({}),
    );
    let (d, _) = validate(&m, &mut resolver, ValidateOptions::default());
    assert!(d.errors().any(|e| e.rule == "V5"), "{d}");
}

#[test]
fn validation_checks_artifacts_and_hashes() {
    let mut m = manifest(json!([]), json!({}));
    m.artifacts.get_mut("g@1.0.0").unwrap().sha256 = None;
    let (d, _) = validate(&m, &mut resolver, ValidateOptions::default());
    assert!(d.errors().any(|e| e.rule == "V2"), "{d}");
    let (d, g) = validate(
        &m,
        &mut resolver,
        ValidateOptions {
            allow_unhashed: true,
        },
    );
    assert!(g.is_some() && d.warnings().any(|w| w.rule == "V2"), "{d}");
}

#[test]
fn static_validation_needs_no_descriptions_and_gates_validate() {
    // approval.capabilitiesHash が宣言と合わない manifest (承認後に書き換えられた想定)
    let m = manifest(
        json!([]),
        json!({ "approval": { "approvedBy": "a", "approvedAt": "2026-11-05T00:00:00Z", "capabilitiesHash": "0".repeat(64) } }),
    );
    let d = validate_static(&m, ValidateOptions::default());
    assert!(d.errors().any(|e| e.rule == "V12"), "{d}");

    // validate は静的検査で止まり、記述 (resolver) を引かない
    let mut never = |_: &ComponentSpec| -> anyhow::Result<PartDesc> {
        panic!("静的検査のエラーがあるのに記述を引いた")
    };
    let (d2, g) = validate(&m, &mut never, ValidateOptions::default());
    assert!(g.is_none());
    assert_eq!(d2.items, d.items);

    // 正しいハッシュなら静的検査は通り、記述を要する検査 (V4) は validate だけが出す
    let mut m = manifest(json!([{"from": "a.nope", "to": "b.in"}]), json!({}));
    m.approval = Some(
        serde_json::from_value(json!({
            "approvedBy": "a", "approvedAt": "2026-11-05T00:00:00Z",
            "capabilitiesHash": ipemu_core::capabilities_hash(&m)
        }))
        .unwrap(),
    );
    let d = validate_static(&m, ValidateOptions::default());
    assert!(!d.has_errors(), "{d}");
    let (d, _) = validate(&m, &mut resolver, ValidateOptions::default());
    assert!(d.errors().any(|e| e.rule == "V4"), "{d}");
}

#[test]
fn topological_order_follows_zero_latency_edges_and_manifest_order() {
    // b → a (latency 0) なら b が先。src は依存なしで manifest 順の先頭
    let m = manifest(
        json!([{"from": "b.out", "to": "a.in", "latency": 0}]),
        json!({}),
    );
    let (_, g) = validate(&m, &mut resolver, ValidateOptions::default());
    assert_eq!(g.unwrap().order, vec![0, 2, 1]);
}

#[test]
fn engine_propagates_with_latency_semantics() {
    // src(=2) --latency0--> a(×3) --latency1--> b(×10)
    let m = manifest(
        json!([{"from": "src.out", "to": "a.in", "latency": 0}, {"from": "a.out", "to": "b.in"}]),
        json!({}),
    );
    let (d, g) = validate(&m, &mut resolver, ValidateOptions::default());
    assert!(!d.has_errors(), "{d}");
    let parts: Vec<Box<dyn Part>> = vec![
        Box::new(Source::new(2.0)),
        Box::new(Gain::new(3.0)),
        Box::new(Gain::new(10.0)),
    ];
    let mut e = Engine::new(g.unwrap(), parts, 1_000_000).unwrap();
    e.step().unwrap();
    // 同一ステップ: a は src の 2 を見て 6 を出す。b は前ステップの a (初期 0) を見る
    assert_eq!(e.output_by_ref("a.out"), Some(6.0));
    assert_eq!(e.output_by_ref("b.out"), Some(0.0));
    e.step().unwrap();
    assert_eq!(e.output_by_ref("b.out"), Some(60.0));
    assert_eq!(e.time_ns(), 2_000_000);
    e.reset().unwrap();
    assert_eq!(e.time_ns(), 0);
    assert_eq!(e.output_by_ref("a.steps"), Some(0.0));
}

#[test]
fn engine_honours_rate_divider() {
    let mut m = manifest(json!([]), json!({}));
    m.components[1].rate_divider = 4;
    let (_, g) = validate(&m, &mut resolver, ValidateOptions::default());
    let parts: Vec<Box<dyn Part>> = vec![
        Box::new(Source::new(0.0)),
        Box::new(Gain::new(1.0)),
        Box::new(Gain::new(1.0)),
    ];
    let mut e = Engine::new(g.unwrap(), parts, 1_000_000).unwrap();
    for _ in 0..8 {
        e.step().unwrap();
    }
    assert_eq!(e.output_by_ref("a.steps"), Some(2.0));
    assert_eq!(e.output_by_ref("b.steps"), Some(8.0));
}

/// テスト用ファームウェア: enc0 を読んで pwm0 = enc0/1000 を出し、gpio-out0 を立てる。
struct EchoFw {
    ticks: u32,
}
impl Firmware for EchoFw {
    fn abi_version(&mut self) -> anyhow::Result<u32> {
        Ok(1)
    }
    fn init(&mut self, regs: &mut HalRegs, cfg: &str) -> anyhow::Result<()> {
        assert!(cfg.contains("hello"));
        regs.hal_log(ipemu_core::hal::LogLevel::Info, "init");
        Ok(())
    }
    fn tick(&mut self, regs: &mut HalRegs) -> anyhow::Result<TickStats> {
        self.ticks += 1;
        let c = regs.hal_enc_read(0);
        regs.hal_pwm_write(0, c as f32 / 1000.0);
        regs.hal_gpio_write(0, true);
        Ok(TickStats { fuel: Some(500) })
    }
    fn irq(&mut self, _: &mut HalRegs, _: u32) -> anyhow::Result<()> {
        Ok(())
    }
    fn flavor(&self) -> &'static str {
        "test"
    }
}

#[test]
fn cpu_virtual_runs_firmware_at_tick_rate_with_timing_model() {
    // 1ms ステップ、tick 200Hz (5ms)、fuel 500 × 2ns = 1µs の実行時間
    let params = CpuVirtualParams::from_json(
        &json!({"tickHz": 200, "timingModel": {"kind": "fuel", "nsPerFuel": 2.0}}),
    )
    .unwrap();
    let mut cpu = CpuVirtual::new(params, r#"{"hello": true}"#.into());
    cpu.attach_firmware(Box::new(EchoFw { ticks: 0 })).unwrap();
    let desc = cpu.describe().clone();
    let n_out = desc.outputs.len();
    let mut out = vec![0.0; n_out];
    let pwm0 = desc.output_index("pwm0").unwrap();
    let gpio0 = desc.output_index("gpio-out0").unwrap();
    let ticks = desc.output_index("tick-count").unwrap();
    let exec = desc.output_index("exec-ns").unwrap();

    let inputs = vec![500.0, 0.0, 0.0, 0.0]; // enc0=500, enc1, gpio-in0, gpio-in1
    for _ in 0..10 {
        cpu.set_inputs(&inputs).unwrap();
        cpu.step(1_000_000).unwrap();
    }
    cpu.get_outputs(&mut out).unwrap();
    assert_eq!(out[ticks], 2.0, "10ms で 5ms 周期なら 2 tick");
    assert!((out[pwm0] - 0.5).abs() < 1e-6);
    assert_eq!(out[gpio0], 1.0);
    assert_eq!(out[exec], 1000.0);
    let logs = cpu.drain_logs();
    assert!(logs.iter().any(|(_, m)| m == "init"));

    // 実行時間モデル fixed で tick 周期を超えるとオーバーラン
    let params = CpuVirtualParams::from_json(
        &json!({"tickHz": 1000, "timingModel": {"kind": "fixed", "fixedNs": 2_000_000}}),
    )
    .unwrap();
    let mut cpu = CpuVirtual::new(params, "{\"hello\":1}".into());
    cpu.attach_firmware(Box::new(EchoFw { ticks: 0 })).unwrap();
    for _ in 0..5 {
        cpu.set_inputs(&inputs).unwrap();
        cpu.step(1_000_000).unwrap();
    }
    assert!(cpu.overruns() >= 4);
}

#[test]
fn builtin_schema_validates_example_manifest() {
    let text = include_str!("../../../schemas/examples/cartpole-lqr.manifest.json");
    let v: serde_json::Value = serde_json::from_str(text).unwrap();
    let mut d = ipemu_core::validate::Diagnostics::default();
    ipemu_core::validate::validate_schema(&v, &mut d);
    assert!(!d.has_errors(), "{d}");
    let m: Manifest = serde_json::from_value(v).unwrap();
    assert_eq!(m.components.len(), 6);
}
