//! manifest の検証 (04-manifest.md の V1〜V12)。サーバとクライアントで同じコードを使う。

use crate::builtins;
use crate::capability::{capabilities_hash, ImportInfo};
use crate::desc::PartDesc;
use crate::graph::{Graph, GraphError};
use crate::manifest::{split_port_ref, ComponentSpec, Manifest};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Default, Clone)]
pub struct Diagnostics {
    pub items: Vec<Diagnostic>,
}

impl Diagnostics {
    fn error(&mut self, rule: &'static str, message: impl Into<String>) {
        self.items.push(Diagnostic {
            rule,
            severity: Severity::Error,
            message: message.into(),
        });
    }
    fn warn(&mut self, rule: &'static str, message: impl Into<String>) {
        self.items.push(Diagnostic {
            rule,
            severity: Severity::Warning,
            message: message.into(),
        });
    }
    /// 呼び出し側 (エンジン) が同じ規則番号で診断を追加するための公開版。
    pub fn error_pub(&mut self, rule: &'static str, message: impl Into<String>) {
        self.error(rule, message)
    }
    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items.iter().filter(|d| d.severity == Severity::Error)
    }
    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items
            .iter()
            .filter(|d| d.severity == Severity::Warning)
    }
}

impl std::fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for d in &self.items {
            let sev = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            writeln!(f, "{sev}[{}]: {}", d.rule, d.message)?;
        }
        Ok(())
    }
}

/// 検証の文脈。ローカル開発 (署名・ハッシュなし) を許すかどうか。
#[derive(Debug, Clone, Copy, Default)]
pub struct ValidateOptions {
    /// true なら sha256 / url の欠落を Error ではなく Warning にする (ローカル manifest)
    pub allow_unhashed: bool,
}

/// 部品の記述を引く手段。wasm 部品はバイナリの `describe()` から、組み込みは `builtins` から。
pub trait DescResolver {
    fn describe(&mut self, spec: &ComponentSpec) -> anyhow::Result<PartDesc>;

    /// その instance が実行する wasm (部品、または cpu-virtual のファームウェア) の import 一覧。
    /// `Ok(None)` は「この文脈では import を読めない」の意味で、V7 / V8 を省略する。
    /// 既定は `None` (閉包だけの簡易 resolver 用)。
    fn imports(&mut self, _spec: &ComponentSpec) -> anyhow::Result<Option<Vec<ImportInfo>>> {
        Ok(None)
    }
}

impl<F> DescResolver for F
where
    F: FnMut(&ComponentSpec) -> anyhow::Result<PartDesc>,
{
    fn describe(&mut self, spec: &ComponentSpec) -> anyhow::Result<PartDesc> {
        self(spec)
    }
}

/// 埋め込んだ JSON Schema (V1)。
#[cfg(feature = "schema")]
pub const SCHEMA_JSON: &str = include_str!("../../../schemas/emulator-manifest.schema.json");

/// JSON テキストの段階で行う検証 (V1: スキーマ適合)。
#[cfg(feature = "schema")]
pub fn validate_schema(json: &serde_json::Value, diags: &mut Diagnostics) {
    let schema: serde_json::Value =
        serde_json::from_str(SCHEMA_JSON).expect("組み込みスキーマが壊れています");
    match jsonschema::validator_for(&schema) {
        Ok(v) => {
            for err in v.iter_errors(json) {
                diags.error("V1", format!("{} (at {})", err, err.instance_path()));
            }
        }
        Err(e) => diags.error("V1", format!("スキーマのコンパイルに失敗: {e}")),
    }
}

/// 部品の記述を要しない静的検査 (D45)。manifest だけで判定できる規則を集めたもの:
/// V2 (アーティファクト参照と sha256)、V3 の組み込み名と firmware の付け先、capability 宣言の
/// 自己整合 (fs.path の閉じ込め等、V7 として報告)、V10、V11、V12 (capabilitiesHash の照合)。
///
/// エンジンはこれを**同意と実体化の前**に実行し、エラーがあれば同意を求めず部品も動かさない。
/// [`validate`] は最初にこれを呼ぶので、両方を呼ぶ側は診断の重複に注意すること。
pub fn validate_static(manifest: &Manifest, opts: ValidateOptions) -> Diagnostics {
    let mut diags = Diagnostics::default();

    // V2: artifact 参照とハッシュ (同じアーティファクトの警告は 1 回だけ)
    let mut warned: BTreeSet<&str> = BTreeSet::new();
    for c in &manifest.components {
        match (&c.artifact, &c.kind) {
            (Some(a), None) => match manifest.artifacts.get(a) {
                None => diags.error(
                    "V2",
                    format!(
                        "部品 `{}` のアーティファクト `{a}` が artifacts にありません",
                        c.instance
                    ),
                ),
                Some(r) => {
                    if !warned.insert(a.as_str()) {
                        continue;
                    }
                    match &r.sha256 {
                        Some(h) if h.len() == 64 && h.chars().all(|ch| ch.is_ascii_hexdigit()) => {}
                        Some(h) => diags.error("V2", format!("アーティファクト `{a}` の sha256 `{h}` が 64 桁 hex ではありません")),
                        None if opts.allow_unhashed => diags.warn("V2", format!("アーティファクト `{a}` に sha256 がありません (ローカル開発のみ許可)")),
                        None => diags.error("V2", format!("アーティファクト `{a}` に sha256 がありません")),
                    }
                    if r.url.is_none() && r.path.is_none() {
                        diags.error(
                            "V2",
                            format!("アーティファクト `{a}` に url も path もありません"),
                        );
                    }
                }
            },
            (None, Some(k)) => {
                if !builtins::BUILTIN_KINDS.contains(&k.as_str()) {
                    diags.error(
                        "V3",
                        format!("部品 `{}`: 未知の組み込み部品 `{k}`", c.instance),
                    );
                }
            }
            (Some(_), Some(_)) => diags.error(
                "V1",
                format!(
                    "部品 `{}`: artifact と kind は同時に指定できません",
                    c.instance
                ),
            ),
            (None, None) => diags.error(
                "V1",
                format!(
                    "部品 `{}`: artifact か kind のどちらかが必要です",
                    c.instance
                ),
            ),
        }
        if let Some(fw) = &c.firmware {
            if c.kind.as_deref() != Some("builtin:cpu-virtual") {
                diags.error(
                    "V3",
                    format!(
                        "部品 `{}`: firmware は builtin:cpu-virtual にだけ指定できます",
                        c.instance
                    ),
                );
            }
            if let Some(def) = &fw.default {
                if !manifest.artifacts.contains_key(def) {
                    diags.error(
                        "V2",
                        format!(
                            "部品 `{}`: firmware.default `{def}` が artifacts にありません",
                            c.instance
                        ),
                    );
                }
            }
        }
    }

    // V11: 周期の整合
    let base = manifest.scheduler.base_step_ns;
    if base <= 0 {
        diags.error("V11", "scheduler.baseStepNs は正である必要があります");
    }
    for c in &manifest.components {
        if c.rate_divider == 0 {
            diags.error(
                "V11",
                format!("部品 `{}`: rateDivider は 1 以上", c.instance),
            );
        }
        if c.kind.as_deref() == Some("builtin:cpu-virtual") && base > 0 {
            if let Ok(p) = builtins::CpuVirtualParams::from_json(&c.params) {
                let period = p.tick_period_ns();
                let step = base * c.rate_divider.max(1) as i64;
                if period % step != 0 && step % period != 0 {
                    diags.error("V11", format!("部品 `{}`: tickHz={} (周期 {period} ns) が baseStep×rateDivider={step} ns の整数倍/約数ではありません", c.instance, p.tick_hz));
                }
            }
        }
    }

    // capability 宣言そのものの整合性 (fs.path の閉じ込め、未知の hostExt)、V10: 決定性外の capability
    for c in &manifest.components {
        if let Some(cap) = &c.capabilities {
            for e in cap.self_check() {
                diags.error("V7", format!("部品 `{}`: {e}", c.instance));
            }
            if cap.clock || cap.random {
                diags.warn(
                    "V10",
                    format!(
                        "部品 `{}` は wasi:clocks / wasi:random を使うため決定性の保証外です",
                        c.instance
                    ),
                );
            }
        }
    }

    // V12 (ハッシュ部分): approval.capabilitiesHash が宣言と一致する。署名の検証は呼び出し側 (クライアント) の責務
    if let Some(ap) = &manifest.approval {
        let actual = capabilities_hash(manifest);
        if !ap.capabilities_hash.eq_ignore_ascii_case(&actual) {
            diags.error(
                "V12",
                format!(
                    "approval.capabilitiesHash が capability 宣言と一致しません (approval {}, 宣言 {actual})。承認後に宣言が変更されています",
                    ap.capabilities_hash
                ),
            );
        }
    }

    diags
}

/// 構造の検証 (V2〜V12)。[`validate_static`] の後に記述を要する検査 (V3 の記述取得、V7 / V8、
/// V4〜V6、V9、snapshot) を続ける。成功時は実行計画 (Graph) も返す。
pub fn validate(
    manifest: &Manifest,
    resolver: &mut dyn DescResolver,
    opts: ValidateOptions,
) -> (Diagnostics, Option<Graph>) {
    let mut diags = validate_static(manifest, opts);
    if diags.has_errors() {
        return (diags, None);
    }

    // 部品の記述を集める (V3: ワールド適合は resolver が検査する)
    let mut descs = Vec::with_capacity(manifest.components.len());
    for c in &manifest.components {
        match resolver.describe(c) {
            Ok(d) => descs.push(d),
            Err(e) => {
                diags.error(
                    "V3",
                    format!("部品 `{}` の記述を取得できません: {e:#}", c.instance),
                );
            }
        }
    }
    if diags.has_errors() {
        return (diags, None);
    }

    // V7 / V8: import と capability 宣言の突き合わせ (import を読める文脈のみ)
    for c in &manifest.components {
        let imports = match resolver.imports(c) {
            Ok(Some(i)) => i,
            Ok(None) => continue,
            Err(e) => {
                diags.error(
                    "V7",
                    format!("部品 `{}` の import を取得できません: {e:#}", c.instance),
                );
                continue;
            }
        };
        let (errors, warnings) = c.capabilities().check_imports(&imports);
        for e in errors {
            diags.error("V7", format!("部品 `{}`: {e}", c.instance));
        }
        for w in warnings {
            diags.warn("V8", format!("部品 `{}`: {w}", c.instance));
        }
    }
    if diags.has_errors() {
        return (diags, None);
    }

    // V4〜V6: 配線
    let graph = match Graph::build(manifest, descs.clone()) {
        Ok(g) => g,
        Err(e) => {
            let rule = match e {
                GraphError::DuplicateInput(_) => "V5",
                GraphError::ZeroLatencyCycle(_) => "V6",
                _ => "V4",
            };
            diags.error(rule, e.to_string());
            return (diags, None);
        }
    };

    // V9: 単位の不一致 (警告)
    for w in &graph.wires {
        let fu = &graph.descs[w.from_inst].outputs[w.from_port].unit;
        let tu = &graph.descs[w.to_inst].inputs[w.to_port].unit;
        if !fu.is_empty() && !tu.is_empty() && fu != tu && w.gain == 1.0 && w.offset == 0.0 {
            diags.warn(
                "V9",
                format!("配線 {}: 単位が一致しません ({fu} → {tu})", w.label),
            );
        }
    }

    // snapshot の参照先が存在するか
    for c in manifest
        .components
        .iter()
        .filter(|c| c.kind.as_deref() == Some("builtin:snapshot"))
    {
        if let Some(ports) = c.params.get("ports").and_then(|v| v.as_array()) {
            let mut seen = BTreeSet::new();
            for p in ports {
                let Some(s) = p.as_str() else {
                    diags.error(
                        "V4",
                        format!("snapshot `{}`: ports の要素は文字列", c.instance),
                    );
                    continue;
                };
                if split_port_ref(s).is_none() || graph.resolve_output(s).is_none() {
                    diags.error(
                        "V4",
                        format!("snapshot `{}`: 出力ポート `{s}` がありません", c.instance),
                    );
                }
                if !seen.insert(s) {
                    diags.warn(
                        "V4",
                        format!("snapshot `{}`: `{s}` が重複しています", c.instance),
                    );
                }
            }
        }
    }

    if diags.has_errors() {
        (diags, None)
    } else {
        (diags, Some(graph))
    }
}
