//! 契約型の往復テストと、TypeScript 型の書き出し。
//!
//! `cargo test -p ipemu-ipc` を走らせると ts-rs が `bindings/*.ts` を更新する (`scripts/gen-ipc-types.sh`)。

use ipemu_ipc::*;
use std::collections::BTreeMap;

fn sample_launch() -> LaunchParams {
    let mut firmware = BTreeMap::new();
    firmware.insert(
        "balance".to_owned(),
        "wasm:build/firmware/fw-balance-rs.wasm".to_owned(),
    );
    LaunchParams {
        source: LaunchSource::Local {
            manifest_path: "dev/cartpole.manifest.json".into(),
            allow_unhashed: true,
        },
        firmware,
        params: vec![ParamOverride {
            instance: "plant".into(),
            key: "pendulumLength".into(),
            value: serde_json::json!(0.5),
        }],
        pacing: Some(PacingConfig {
            mode: PacingMode::Fast,
            speed: 1.0,
        }),
        auto_start: true,
    }
}

#[test]
fn request_roundtrip_and_method_tag() {
    let req = Request::new(7, RequestBody::SessionLaunch(sample_launch()));
    let text = serde_json::to_string(&req).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["jsonrpc"], "2.0");
    assert_eq!(v["id"], 7);
    assert_eq!(v["method"], "session.launch");
    assert_eq!(v["params"]["source"]["kind"], "local");
    assert_eq!(
        v["params"]["source"]["manifestPath"],
        "dev/cartpole.manifest.json"
    );
    assert_eq!(v["params"]["autoStart"], true);
    let back = Request::parse(&text).unwrap();
    assert_eq!(back, req);
}

#[test]
fn request_without_params_is_accepted() {
    for text in [
        r#"{"jsonrpc":"2.0","id":1,"method":"session.describe"}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"session.describe","params":null}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":"session.describe","params":{}}"#,
        r#"{"jsonrpc":"2.0","id":"abc","method":"run.start"}"#,
    ] {
        let r = Request::parse(text).unwrap_or_else(|e| panic!("{text}: {e:?}"));
        assert!(matches!(
            r.body,
            RequestBody::SessionDescribe(_) | RequestBody::RunStart(_)
        ));
    }
}

#[test]
fn request_errors_map_to_jsonrpc_codes() {
    let (id, e) = Request::parse("{not json").unwrap_err();
    assert!(id.is_none());
    assert_eq!(e.code, codes::PARSE_ERROR);

    let (id, e) = Request::parse(r#"{"jsonrpc":"2.0","method":"run.start"}"#).unwrap_err();
    assert!(id.is_none());
    assert_eq!(e.code, codes::INVALID_REQUEST);

    let (id, e) = Request::parse(r#"{"jsonrpc":"2.0","id":3,"method":"run.fly"}"#).unwrap_err();
    assert_eq!(id, Some(RequestId::Num(3)));
    assert_eq!(e.code, codes::METHOD_NOT_FOUND);

    let (id, e) =
        Request::parse(r#"{"jsonrpc":"2.0","id":4,"method":"run.step","params":{"n":"x"}}"#)
            .unwrap_err();
    assert_eq!(id, Some(RequestId::Num(4)));
    assert_eq!(e.code, codes::INVALID_PARAMS);
}

#[test]
fn all_methods_are_listed_once() {
    // RequestBody::METHODS と enum の対応が崩れていないか (serialize した method 名が一覧にある)
    let bodies = vec![
        RequestBody::SessionLaunch(sample_launch()),
        RequestBody::SessionDescribe(Empty {}),
        RequestBody::CatalogLocal(Empty {}),
        RequestBody::RunStart(Empty {}),
        RequestBody::RunStop(Empty {}),
        RequestBody::RunReset(Empty {}),
        RequestBody::RunStep(StepParams { n: 3 }),
        RequestBody::RunSetPacing(PacingConfig::default()),
        RequestBody::ParamSet(ParamOverride {
            instance: "plant".into(),
            key: "k".into(),
            value: serde_json::json!(1),
        }),
        RequestBody::FirmwareLoad(FirmwareLoadParams {
            slot: "balance".into(),
            source: "native:x.so".into(),
        }),
        RequestBody::FirmwareWatch(FirmwareWatchParams {
            slot: "balance".into(),
            source: None,
        }),
        RequestBody::ScopeSubscribe(ScopeSubscribeParams {
            ports: vec!["plant.theta".into()],
            rate_hz: 1000.0,
            window_s: 0.05,
            subscription_id: None,
        }),
        RequestBody::ConsentAnswer(ConsentAnswerParams {
            request_id: "r1".into(),
            grant: true,
            remember: false,
        }),
        RequestBody::ManifestRefresh(Empty {}),
    ];
    assert_eq!(bodies.len(), RequestBody::METHODS.len());
    for b in bodies {
        let v = serde_json::to_value(&b).unwrap();
        let m = v["method"].as_str().unwrap();
        assert_eq!(m, b.method());
        assert!(RequestBody::METHODS.contains(&m), "{m}");
        let back: RequestBody = serde_json::from_value(v).unwrap();
        assert_eq!(back, b);
    }
}

#[test]
fn response_and_error_shape() {
    let ok = Response::ok(RequestId::Num(1), &RunStatus::default());
    let v = serde_json::to_value(&ok).unwrap();
    assert_eq!(v["result"]["state"], "idle");
    assert!(v.get("error").is_none());

    let err = Response::err(
        RequestId::Str("x".into()),
        RpcError::new(codes::RESTART_REQUIRED, "native")
            .with_data(serde_json::json!({"restartRequired": true})),
    );
    let v = serde_json::to_value(&err).unwrap();
    assert_eq!(v["error"]["code"], -32002);
    assert_eq!(v["error"]["data"]["restartRequired"], true);
    assert!(v.get("result").is_none());
    let back: Response = serde_json::from_value(v).unwrap();
    assert_eq!(back, err);
}

#[test]
fn notification_roundtrip() {
    let n = Notification::new(NotificationBody::ConsentRequired(ConsentRequest {
        request_id: "r1".into(),
        instances: vec!["modbus".into()],
        artifact: "modbus-client@1.0.0".into(),
        artifact_sha256: "ab".repeat(32),
        capability: "net.tcpConnect:192.168.10.5:502".into(),
        reason: "PLC".into(),
        manifest_id: "m".into(),
        revision: 2,
    }));
    let text = serde_json::to_string(&n).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["method"], "consent.required");
    assert_eq!(v["params"]["requestId"], "r1");
    assert!(v.get("id").is_none());
    let back: Notification = serde_json::from_str(&text).unwrap();
    assert_eq!(back, n);
}

#[test]
fn binary_frames_roundtrip_with_kind_byte() {
    let st = StateFrame {
        t_ns: 1_234_000_000,
        tick: 1234,
        epoch: 2,
        state: RunState::Running,
        pacing: PacingMode::Realtime,
        speed: 1.0,
        values: vec![0.1, -3.25, f64::INFINITY],
        stats: SchedulerStats {
            realtime_ratio: 1.0,
            step_wall_us: 12.5,
            slip_ms_total: 0.0,
            slips: 0,
            step_overruns: 0,
        },
    };
    let bytes = encode_frame(FrameKind::State, &st).unwrap();
    assert_eq!(bytes[0], 0x01);
    let back: StateFrame = decode_frame(FrameKind::State, &bytes).unwrap();
    assert_eq!(back, st);
    // 種別違いは None
    assert!(decode_frame::<StateFrame>(FrameKind::Scope, &bytes).is_none());

    let sc = ScopeFrame {
        subscription_id: 1,
        epoch: 0,
        t0_ns: 0,
        period_ns: 1_000_000,
        channels: vec![vec![1.0, 2.0], vec![3.0, 4.0]],
    };
    let bytes = encode_frame(FrameKind::Scope, &sc).unwrap();
    assert_eq!(bytes[0], 0x02);
    let back: ScopeFrame = decode_frame(FrameKind::Scope, &bytes).unwrap();
    assert_eq!(back, sc);

    // 名前付きフィールド (map) で符号化されていること: TS 側が構造体名でアクセスできる
    let v: rmp_serde::decode::Error = rmp_serde::from_slice::<Vec<i32>>(&bytes[1..]).unwrap_err();
    let _ = v;
    let as_value: serde_json::Value = rmp_serde::from_slice(&bytes[1..]).unwrap();
    assert_eq!(as_value["subscriptionId"], 1);
}

#[test]
fn discovery_line_is_single_line_json() {
    let d = Discovery {
        port: 54321,
        token: "t".into(),
        pid: 1,
        ipc_version: IPC_VERSION,
        engine_version: "0.1.0".into(),
    };
    let line = serde_json::to_string(&d).unwrap();
    assert!(!line.contains('\n'));
    assert!(line.starts_with(r#"{"port":54321,"token":"t""#));
}
