//! manifest → 実体化 → 実行、の一連 (M1: ローカル manifest のみ)。

use anyhow::{anyhow, bail, Context};
use ipemu_core::builtins::{CpuVirtual, CpuVirtualParams};
use ipemu_core::hal::{Firmware, LogLevel};
use ipemu_core::manifest::{ArtifactRef, ComponentSpec, Manifest};
use ipemu_core::validate::{validate, validate_schema, Diagnostics, ValidateOptions};
use ipemu_core::{Engine, Part, PartDesc};
use ipemu_wasm::{NativeFirmware, Runtime, WasmFirmware, WasmPart};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// ファームウェアの指定。
#[derive(Debug, Clone)]
pub enum FirmwareSource {
    /// manifest の firmware.default (アーティファクト)
    Default,
    Wasm(PathBuf),
    Native(PathBuf),
}

impl FirmwareSource {
    /// `wasm:path` / `native:path` / `artifact:name@ver` を解釈する。
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        if let Some(p) = s.strip_prefix("wasm:") {
            Ok(FirmwareSource::Wasm(p.into()))
        } else if let Some(p) = s.strip_prefix("native:") {
            Ok(FirmwareSource::Native(p.into()))
        } else if s == "default" || s.starts_with("artifact:") {
            Ok(FirmwareSource::Default)
        } else {
            bail!("ファームウェア指定 `{s}` は wasm:<path> / native:<path> / default のいずれか")
        }
    }
}

/// アーティファクト名 (`name@version`) と参照からローカルパスを引くフック。
/// bootstrap がキャッシュ (`sha256/ab/cd/<hex>`) を指すために使う。None を返すと既定の解決 (path / file:) に戻る。
pub type ArtifactResolverFn =
    dyn Fn(&str, &ArtifactRef) -> anyhow::Result<Option<PathBuf>> + Send + Sync;

#[derive(Clone)]
pub struct ArtifactResolver(pub Arc<ArtifactResolverFn>);

impl std::fmt::Debug for ArtifactResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ArtifactResolver(..)")
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionOptions {
    /// slot 名 → ファームウェアの上書き
    pub firmware_overrides: HashMap<String, FirmwareSource>,
    /// ローカル manifest: sha256 なしを許す
    pub allow_unhashed: bool,
    pub runtime: Option<ipemu_wasm::runtime::RuntimeOptions>,
    /// `instance.key=value` 形式のパラメータ上書き
    pub param_overrides: Vec<(String, String, serde_json::Value)>,
    /// アーティファクトの解決フック (配信 manifest をキャッシュから組み立てるとき)
    pub artifact_resolver: Option<ArtifactResolver>,
}

/// 実行中のセッション。
pub struct Session {
    pub manifest: Manifest,
    pub engine: Engine,
    pub diagnostics: Diagnostics,
    pub snapshot_ports: Vec<String>,
    snapshot_refs: Vec<(usize, usize)>,
    cpu_indices: Vec<usize>,
    _runtime: Runtime,
}

fn resolve_artifact_path(
    manifest_dir: &Path,
    manifest: &Manifest,
    name: &str,
    opts: &SessionOptions,
) -> anyhow::Result<PathBuf> {
    let a = manifest
        .artifacts
        .get(name)
        .ok_or_else(|| anyhow!("アーティファクト `{name}` が manifest にありません"))?;
    if let Some(r) = &opts.artifact_resolver {
        if let Some(p) = (r.0)(name, a)? {
            return Ok(p);
        }
    }
    if let Some(p) = &a.path {
        return Ok(manifest_dir.join(p));
    }
    if let Some(u) = &a.url {
        if let Some(p) = u.strip_prefix("file:") {
            return Ok(manifest_dir.join(p));
        }
        bail!("アーティファクト `{name}` は url ({u}) でしか指定されていません。サーバから取得するには --server で起動してください")
    }
    bail!("アーティファクト `{name}` に path も url もありません")
}

fn verify_sha256(path: &Path, expected: Option<&str>) -> anyhow::Result<Vec<u8>> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).with_context(|| format!("{} を読めません", path.display()))?;
    if let Some(exp) = expected {
        let actual = hex::encode(Sha256::digest(&bytes));
        anyhow::ensure!(
            actual == exp,
            "{} の sha256 が一致しません (manifest {exp}, 実体 {actual})",
            path.display()
        );
    }
    Ok(bytes)
}

impl Session {
    /// manifest ファイルから組み立てる。
    pub fn load(manifest_path: &Path, opts: &SessionOptions) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(manifest_path)
            .with_context(|| format!("{} を読めません", manifest_path.display()))?;
        let json: serde_json::Value =
            serde_json::from_str(&text).context("manifest の JSON が不正")?;
        let mut diags = Diagnostics::default();
        validate_schema(&json, &mut diags);
        if diags.has_errors() {
            bail!("manifest がスキーマに適合しません:\n{diags}");
        }
        let mut manifest: Manifest =
            serde_json::from_value(json).context("manifest の構造が不正")?;
        let manifest_dir = manifest_path.parent().unwrap_or(Path::new("."));

        // パラメータ上書き
        for (inst, key, val) in &opts.param_overrides {
            let c = manifest
                .components
                .iter_mut()
                .find(|c| &c.instance == inst)
                .ok_or_else(|| anyhow!("--param: 部品 `{inst}` がありません"))?;
            if let Some(obj) = c.params.as_object_mut() {
                obj.insert(key.clone(), val.clone());
            }
        }

        let runtime = Runtime::new(opts.runtime.clone().unwrap_or_default())?;

        // 部品の記述を集めながら実体を作る (describe は実体化が要るので、ここで一度に行う)
        let mut compiled: HashMap<String, wasmtime_component::Component> = HashMap::new();
        let mut parts: Vec<Box<dyn Part>> = Vec::new();
        let mut descs: Vec<PartDesc> = Vec::new();
        let mut cpu_indices = Vec::new();
        for (i, c) in manifest.components.iter().enumerate() {
            match (&c.artifact, c.kind.as_deref()) {
                (Some(a), None) => {
                    let path = resolve_artifact_path(manifest_dir, &manifest, a, opts)?;
                    let component = match compiled.get(a) {
                        Some(cm) => cm.clone(),
                        None => {
                            let expected = manifest.artifacts[a].sha256.as_deref();
                            let bytes = verify_sha256(&path, expected)?;
                            let cm = WasmPart::compile(&runtime, &bytes)
                                .with_context(|| format!("部品 `{}` ({a})", c.instance))?;
                            compiled.insert(a.clone(), cm.clone());
                            cm
                        }
                    };
                    let mut part = WasmPart::instantiate(&runtime, &component, &c.instance)?;
                    part.init(&c.params)
                        .with_context(|| format!("部品 `{}` の init", c.instance))?;
                    descs.push(part.describe().clone());
                    parts.push(Box::new(part));
                }
                (None, Some("builtin:cpu-virtual")) => {
                    let p = CpuVirtualParams::from_json(&c.params)
                        .with_context(|| format!("部品 `{}` の params", c.instance))?;
                    let cfg_json = firmware_cfg_json(c, &p)?;
                    let mut cpu = CpuVirtual::new(p, cfg_json);
                    let fw = load_firmware(&runtime, manifest_dir, &manifest, c, opts)?;
                    cpu.attach_firmware(fw)
                        .with_context(|| format!("部品 `{}` へのファームウェア装着", c.instance))?;
                    descs.push(cpu.describe().clone());
                    cpu_indices.push(i);
                    parts.push(Box::new(cpu));
                }
                (None, Some("builtin:snapshot")) => {
                    let d = ipemu_core::builtins::describe_builtin("builtin:snapshot", &c.params)?;
                    descs.push(d.clone());
                    parts.push(Box::new(NullPart(d)));
                }
                (None, Some(other)) => bail!("部品 `{}`: 未知の組み込み部品 `{other}`", c.instance),
                _ => bail!(
                    "部品 `{}`: artifact と kind のどちらか一方が必要",
                    c.instance
                ),
            }
        }

        // 検証 (実体から得た記述で V2〜V11)
        let mut by_name: HashMap<String, PartDesc> = manifest
            .components
            .iter()
            .map(|c| c.instance.clone())
            .zip(descs.iter().cloned())
            .collect();
        let mut resolver = |spec: &ComponentSpec| -> anyhow::Result<PartDesc> {
            by_name
                .remove(&spec.instance)
                .ok_or_else(|| anyhow!("記述がありません"))
        };
        let (d2, graph) = validate(
            &manifest,
            &mut resolver,
            ValidateOptions {
                allow_unhashed: opts.allow_unhashed,
            },
        );
        diags.items.extend(d2.items);
        let graph = graph.ok_or_else(|| anyhow!("manifest の検証に失敗:\n{diags}"))?;

        // スナップショット列
        let mut snapshot_ports = Vec::new();
        for c in manifest
            .components
            .iter()
            .filter(|c| c.kind.as_deref() == Some("builtin:snapshot"))
        {
            if let Some(ps) = c.params.get("ports").and_then(|v| v.as_array()) {
                snapshot_ports.extend(ps.iter().filter_map(|p| p.as_str().map(str::to_owned)));
            }
        }
        if snapshot_ports.is_empty() {
            for (i, d) in graph.descs.iter().enumerate() {
                for o in &d.outputs {
                    snapshot_ports.push(format!("{}.{}", graph.instances[i], o.name));
                }
            }
        }
        let snapshot_refs = snapshot_ports
            .iter()
            .map(|p| {
                graph
                    .resolve_output(p)
                    .ok_or_else(|| anyhow!("ポート `{p}` がありません"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let base = manifest.scheduler.base_step_ns;
        let engine = Engine::new(graph, parts, base)?;
        Ok(Self {
            manifest,
            engine,
            diagnostics: diags,
            snapshot_ports,
            snapshot_refs,
            cpu_indices,
            _runtime: runtime,
        })
    }

    pub fn snapshot_values(&self, out: &mut Vec<f64>) {
        out.clear();
        for (i, p) in &self.snapshot_refs {
            out.push(self.engine.output(*i, *p));
        }
    }

    pub fn snapshot_divider(&self) -> u64 {
        self.manifest.scheduler.snapshot_divider.max(1) as u64
    }

    /// 仮想時間で `seconds` 進める。スナップショットのたびに `on_snapshot(t_ns, values)` を呼ぶ。
    pub fn run_for(
        &mut self,
        seconds: f64,
        mut on_snapshot: impl FnMut(i64, &[f64], &mut Vec<(String, LogLevel, String)>),
    ) -> anyhow::Result<()> {
        let base = self.engine.base_step_ns();
        let steps = (seconds * 1e9 / base as f64).round() as u64;
        let div = self.snapshot_divider();
        let mut buf = Vec::new();
        for _ in 0..steps {
            self.engine.step().map_err(|e| anyhow!("{e}"))?;
            if self.engine.tick().is_multiple_of(div) {
                self.snapshot_values(&mut buf);
                let mut logs = self.engine.drain_logs();
                on_snapshot(self.engine.time_ns(), &buf, &mut logs);
            }
        }
        Ok(())
    }

    pub fn cpu_indices(&self) -> &[usize] {
        &self.cpu_indices
    }
}

/// ポートを持たない組み込み部品 (snapshot) のダミー。
struct NullPart(PartDesc);
impl Part for NullPart {
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
    fn get_outputs(&mut self, _: &mut [f64]) -> anyhow::Result<()> {
        Ok(())
    }
}

/// ファームウェアに渡す cfg JSON: firmware.params に tickHz を補う。
fn firmware_cfg_json(c: &ComponentSpec, p: &CpuVirtualParams) -> anyhow::Result<String> {
    let mut params = c
        .firmware
        .as_ref()
        .map(|f| f.params.clone())
        .unwrap_or(serde_json::json!({}));
    if let Some(obj) = params.as_object_mut() {
        obj.entry("tickHz").or_insert(serde_json::json!(p.tick_hz));
    }
    Ok(serde_json::to_string(&params)?)
}

fn load_firmware(
    rt: &Runtime,
    manifest_dir: &Path,
    manifest: &Manifest,
    c: &ComponentSpec,
    opts: &SessionOptions,
) -> anyhow::Result<Box<dyn Firmware>> {
    let slot = c
        .firmware
        .as_ref()
        .ok_or_else(|| anyhow!("部品 `{}`: firmware の指定がありません", c.instance))?;
    let source = opts
        .firmware_overrides
        .get(&slot.slot)
        .cloned()
        .unwrap_or(FirmwareSource::Default);
    match source {
        FirmwareSource::Default => {
            let name = slot.default.as_ref().ok_or_else(|| {
                anyhow!(
                    "slot `{}` に既定のファームウェアがなく、上書きもありません",
                    slot.slot
                )
            })?;
            let path = resolve_artifact_path(manifest_dir, manifest, name, opts)?;
            let bytes = verify_sha256(&path, manifest.artifacts[name].sha256.as_deref())?;
            Ok(Box::new(
                WasmFirmware::load(rt, &bytes).with_context(|| format!("ファームウェア {name}"))?,
            ))
        }
        FirmwareSource::Wasm(p) => {
            anyhow::ensure!(
                slot.allow_override.iter().any(|a| a == "wasm") || opts.allow_unhashed,
                "slot `{}` は wasm の上書きを許していません",
                slot.slot
            );
            Ok(Box::new(WasmFirmware::load_file(rt, &p)?))
        }
        FirmwareSource::Native(p) => {
            anyhow::ensure!(
                slot.allow_override.iter().any(|a| a == "native") || opts.allow_unhashed,
                "slot `{}` は native の上書きを許していません",
                slot.slot
            );
            Ok(Box::new(NativeFirmware::load_file(&p)?))
        }
    }
}

// wasmtime の型名を短く
mod wasmtime_component {
    pub use ipemu_wasm::wasmtime::component::Component;
}
