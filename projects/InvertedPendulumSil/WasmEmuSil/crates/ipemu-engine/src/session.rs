//! manifest → 実体化 → 実行、の一連 (M1: ローカル manifest のみ)。
//!
//! M5: 実体化の前に capability の計画 ([`CapabilityPlan`]) を立て、同意 ([`crate::consent`]) が
//! 揃ってから、宣言・同意済みの capability だけを付与して実体化する。

use crate::capability::{CapabilityPlan, DataDirs, PlanEntry};
use crate::consent::{resolve_consent, ConsentOutcome, ConsentProvider, ConsentStore, NoConsent};
use anyhow::{anyhow, bail, Context};
use ipemu_core::builtins::{CpuVirtual, CpuVirtualParams};
use ipemu_core::capability::ImportInfo;
use ipemu_core::consent::ConsentRequest;
use ipemu_core::hal::{Firmware, LogLevel};
use ipemu_core::manifest::{ComponentSpec, Manifest};
use ipemu_core::validate::{validate, validate_schema, DescResolver, Diagnostics, ValidateOptions};
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

#[derive(Clone, Default)]
pub struct SessionOptions {
    /// slot 名 → ファームウェアの上書き
    pub firmware_overrides: HashMap<String, FirmwareSource>,
    /// ローカル manifest: sha256 なしを許す
    pub allow_unhashed: bool,
    pub runtime: Option<ipemu_wasm::runtime::RuntimeOptions>,
    /// `instance.key=value` 形式のパラメータ上書き
    pub param_overrides: Vec<(String, String, serde_json::Value)>,
    /// データディレクトリ (同意ストアと fs capability の実体)。None なら OS 既定
    pub data_dir: Option<PathBuf>,
    /// 同意の問い合わせ先。None なら何も答えない (未同意があれば起動不可)
    pub consent: Option<Arc<dyn ConsentProvider>>,
    /// 同意ストアを使わない (テスト用)。false なら `<data dir>/ipemu/consent.json`
    pub no_consent_store: bool,
    /// 検証のみ (`validate`): 同意を求めず、capability は deny-all 形で付与する (D34)。
    /// 実体化・describe・init は走るが、接続・名前解決・ファイル操作はすべて拒否される
    pub validate_only: bool,
}

impl std::fmt::Debug for SessionOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionOptions")
            .field("firmware_overrides", &self.firmware_overrides)
            .field("allow_unhashed", &self.allow_unhashed)
            .field("runtime", &self.runtime)
            .field("param_overrides", &self.param_overrides)
            .field("data_dir", &self.data_dir)
            .field("consent", &self.consent.as_ref().map(|c| c.name()))
            .field("no_consent_store", &self.no_consent_store)
            .field("validate_only", &self.validate_only)
            .finish()
    }
}

/// 起動できない理由のうち、利用者の判断に関わるもの (CLI は終了コードで区別する)。
#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("同意されていない capability があります ({} 件)。--consent-file で事前同意を渡すか、シェルの同意ダイアログで回答してください:\n{}", .0.len(), format_requests(.0))]
    ConsentRequired(Vec<ConsentRequest>),
    #[error("拒否された capability があるため起動しません ({} 件。部分的には起動しません):\n{}", .0.len(), format_requests(.0))]
    ConsentDenied(Vec<ConsentRequest>),
}

fn format_requests(rs: &[ConsentRequest]) -> String {
    rs.iter()
        .map(|r| format!("  {}  ({})", r.request_id, r.reason))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 実行中のセッション。
pub struct Session {
    pub manifest: Manifest,
    pub engine: Engine,
    pub diagnostics: Diagnostics,
    pub snapshot_ports: Vec<String>,
    /// 付与した capability (instance ごと)。同意状態の表示 (`session.describe`) に使う
    pub capability_plan: CapabilityPlan,
    /// manifest ファイルのあるディレクトリ (path 参照の基準)
    pub manifest_dir: PathBuf,
    /// 組み立てに使った選択肢。ホット差替・再起動時の上書きの元
    pub options: SessionOptions,
    snapshot_refs: Vec<(usize, usize)>,
    cpu_indices: Vec<usize>,
    /// slot 名 → cpu-virtual の部品 index
    slots: HashMap<String, usize>,
    runtime: Runtime,
}

/// `firmware.load` の結果 (IPC の FirmwareLoadResult に写す)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirmwareSwap {
    /// wasm をホットで差し替えた。プラントの状態はそのまま
    Hot { flavor: &'static str },
    /// native 形。プロセス内でのアンロードは信頼できないので、エンジンの再起動が必要 (D23)
    RestartRequired,
}

/// `param.set` の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamChange {
    /// 部品を `init` し直した (内部状態は初期化)
    Reinitialized,
    /// 組み込み部品 (cpu-virtual 等) は再 launch が要る
    RestartRequired,
}

/// 実体化前に読み込んだ wasm (sha256 と import は capability の判定に使う)。
struct LoadedWasm {
    label: String,
    bytes: Vec<u8>,
    sha256: String,
    imports: Vec<ImportInfo>,
}

fn load_wasm(path: &Path, label: &str, expected_sha: Option<&str>) -> anyhow::Result<LoadedWasm> {
    use sha2::{Digest, Sha256};
    let bytes = verify_sha256(path, expected_sha)?;
    let sha256 = hex::encode(Sha256::digest(&bytes));
    let imports = ipemu_core::imports::component_imports(&bytes)
        .with_context(|| format!("{label} の import を読めません"))?;
    Ok(LoadedWasm {
        label: label.to_owned(),
        bytes,
        sha256,
        imports,
    })
}

/// 実体化に必要な材料 (第 1 パスの結果)。
enum Prepared {
    Part(LoadedWasm),
    Cpu(FirmwareBytes),
    Null,
}

enum FirmwareBytes {
    Wasm(LoadedWasm),
    Native(PathBuf),
}

/// validate に渡す resolver: 実体から得た記述と、読み込んだ wasm の import。
struct SessionResolver {
    descs: HashMap<String, PartDesc>,
    imports: HashMap<String, Vec<ImportInfo>>,
}

impl DescResolver for SessionResolver {
    fn describe(&mut self, spec: &ComponentSpec) -> anyhow::Result<PartDesc> {
        self.descs
            .remove(&spec.instance)
            .ok_or_else(|| anyhow!("記述がありません"))
    }
    fn imports(&mut self, spec: &ComponentSpec) -> anyhow::Result<Option<Vec<ImportInfo>>> {
        Ok(self.imports.get(&spec.instance).cloned())
    }
}

fn resolve_artifact_path(
    manifest_dir: &Path,
    manifest: &Manifest,
    name: &str,
) -> anyhow::Result<PathBuf> {
    let a = manifest
        .artifacts
        .get(name)
        .ok_or_else(|| anyhow!("アーティファクト `{name}` が manifest にありません"))?;
    if let Some(p) = &a.path {
        return Ok(manifest_dir.join(p));
    }
    if let Some(u) = &a.url {
        if let Some(p) = u.strip_prefix("file:") {
            return Ok(manifest_dir.join(p));
        }
        bail!("アーティファクト `{name}` は url ({u}) でしか指定されていません。M1 のエンジンはローカル path のみ対応です")
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
        let dirs = DataDirs::resolve(opts.data_dir.as_deref());

        // 第 1 パス: wasm を読み (sha256 / import)、capability の計画を立てる。まだ実体化しない
        let mut prepared: Vec<Prepared> = Vec::with_capacity(manifest.components.len());
        let mut plan = CapabilityPlan::default();
        let mut imports_by_instance: HashMap<String, Vec<ImportInfo>> = HashMap::new();
        for c in &manifest.components {
            let p = match (&c.artifact, c.kind.as_deref()) {
                (Some(a), None) => {
                    let path = resolve_artifact_path(manifest_dir, &manifest, a)?;
                    let w = load_wasm(&path, a, manifest.artifacts[a].sha256.as_deref())
                        .with_context(|| format!("部品 `{}` ({a})", c.instance))?;
                    plan.push(PlanEntry {
                        instance: c.instance.clone(),
                        artifact: a.clone(),
                        artifact_sha256: w.sha256.clone(),
                        capabilities: c.capabilities(),
                    });
                    imports_by_instance.insert(c.instance.clone(), w.imports.clone());
                    Prepared::Part(w)
                }
                (None, Some("builtin:cpu-virtual")) => {
                    let fw = locate_firmware(manifest_dir, &manifest, c, opts)?;
                    if let FirmwareBytes::Wasm(w) = &fw {
                        plan.push(PlanEntry {
                            instance: c.instance.clone(),
                            artifact: w.label.clone(),
                            artifact_sha256: w.sha256.clone(),
                            capabilities: c.capabilities(),
                        });
                        imports_by_instance.insert(c.instance.clone(), w.imports.clone());
                    }
                    Prepared::Cpu(fw)
                }
                (None, Some("builtin:snapshot")) => Prepared::Null,
                (None, Some(other)) => bail!("部品 `{}`: 未知の組み込み部品 `{other}`", c.instance),
                _ => bail!(
                    "部品 `{}`: artifact と kind のどちらか一方が必要",
                    c.instance
                ),
            };
            prepared.push(p);
        }

        // V7 を実体化より前に出す (実体化の失敗より原因が分かりやすい)。警告 (V8) は後の validate が出す
        for c in &manifest.components {
            if let Some(imports) = imports_by_instance.get(&c.instance) {
                let (errors, _) = c.capabilities().check_imports(imports);
                if !errors.is_empty() {
                    let mut d = Diagnostics::default();
                    for e in errors {
                        d.error_pub("V7", format!("部品 `{}`: {e}", c.instance));
                    }
                    bail!("manifest の検証に失敗:\n{d}");
                }
            }
        }

        // 同意: 記憶 → 問い合わせ。拒否が 1 件でもあれば起動不可。
        // validate_only では deny-all で付与するので同意は要らない (I/O が起きない)
        let requests = plan.requests(&manifest.id, manifest.revision);
        if !requests.is_empty() && !opts.validate_only {
            let mut store = if opts.no_consent_store {
                None
            } else {
                Some(ConsentStore::open(&dirs.consent_file())?)
            };
            let no_consent = NoConsent;
            let provider: &dyn ConsentProvider = match &opts.consent {
                Some(p) => p.as_ref(),
                None => &no_consent,
            };
            match resolve_consent(requests, store.as_mut(), provider)? {
                ConsentOutcome::Ready => {}
                ConsentOutcome::Required(rs) => return Err(LaunchError::ConsentRequired(rs).into()),
                ConsentOutcome::Denied(rs) => return Err(LaunchError::ConsentDenied(rs).into()),
            }
        }

        // 第 2 パス: 同意済みの capability を付与して実体化する
        let mut compiled: HashMap<String, wasmtime_component::Component> = HashMap::new();
        let mut parts: Vec<Box<dyn Part>> = Vec::new();
        let mut descs: Vec<PartDesc> = Vec::new();
        let mut cpu_indices = Vec::new();
        for (i, (c, p)) in manifest.components.iter().zip(prepared).enumerate() {
            match p {
                Prepared::Part(w) => {
                    let component = match compiled.get(&w.label) {
                        Some(cm) => cm.clone(),
                        None => {
                            let cm = WasmPart::compile(&runtime, &w.bytes)
                                .with_context(|| format!("部品 `{}` ({})", c.instance, w.label))?;
                            compiled.insert(w.label.clone(), cm.clone());
                            cm
                        }
                    };
                    let grant = if opts.validate_only {
                        plan.grant_for_validate(&c.instance)
                    } else {
                        plan.grant_for(&c.instance, &dirs)?
                    };
                    let mut part =
                        WasmPart::instantiate_with(&runtime, &component, &c.instance, grant)?;
                    part.init(&c.params)
                        .with_context(|| format!("部品 `{}` の init", c.instance))?;
                    descs.push(part.describe().clone());
                    parts.push(Box::new(part));
                }
                Prepared::Cpu(fw) => {
                    let p = CpuVirtualParams::from_json(&c.params)
                        .with_context(|| format!("部品 `{}` の params", c.instance))?;
                    let cfg_json = firmware_cfg_json(c, &p)?;
                    let mut cpu = CpuVirtual::new(p, cfg_json);
                    let fw: Box<dyn Firmware> = match fw {
                        FirmwareBytes::Wasm(w) => {
                            let grant = if opts.validate_only {
                                plan.grant_for_validate(&c.instance)
                            } else {
                                plan.grant_for(&c.instance, &dirs)?
                            };
                            Box::new(
                                WasmFirmware::load_with(&runtime, &w.bytes, grant)
                                    .with_context(|| format!("ファームウェア {}", w.label))?,
                            )
                        }
                        FirmwareBytes::Native(path) => Box::new(NativeFirmware::load_file(&path)?),
                    };
                    cpu.attach_firmware(fw)
                        .with_context(|| format!("部品 `{}` へのファームウェア装着", c.instance))?;
                    descs.push(cpu.describe().clone());
                    cpu_indices.push(i);
                    parts.push(Box::new(cpu));
                }
                Prepared::Null => {
                    let d = ipemu_core::builtins::describe_builtin("builtin:snapshot", &c.params)?;
                    descs.push(d.clone());
                    parts.push(Box::new(NullPart(d)));
                }
            }
        }

        // 検証 (実体から得た記述と import で V2〜V12)
        let mut resolver = SessionResolver {
            descs: manifest
                .components
                .iter()
                .map(|c| c.instance.clone())
                .zip(descs.iter().cloned())
                .collect(),
            imports: imports_by_instance,
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
        let slots: HashMap<String, usize> = manifest
            .components
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.firmware.as_ref().map(|f| (f.slot.clone(), i)))
            .collect();
        let engine = Engine::new(graph, parts, base)?;
        Ok(Self {
            manifest,
            engine,
            diagnostics: diags,
            snapshot_ports,
            capability_plan: plan,
            manifest_dir: manifest_dir.to_path_buf(),
            options: opts.clone(),
            snapshot_refs,
            cpu_indices,
            slots,
            runtime,
        })
    }

    /// slot 名の一覧 (manifest 順)。
    pub fn slots(&self) -> impl Iterator<Item = (&str, usize)> {
        self.manifest
            .components
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.firmware.as_ref().map(|f| (f.slot.as_str(), i)))
    }

    /// slot に装着中のファームウェアの形 (`wasm` / `native`)。
    pub fn firmware_flavor(&mut self, slot: &str) -> Option<&'static str> {
        let i = *self.slots.get(slot)?;
        self.engine
            .part_mut(i)
            .as_any_mut()?
            .downcast_mut::<CpuVirtual>()?
            .firmware_flavor()
    }

    /// ファームウェアの差し替え (05-firmware.md「リビルド後の反映」)。
    /// wasm: 新しいファームウェアを `init` して slot だけ入れ替える。`init` に失敗したら古い slot を残して Err。
    /// native: 上書き指定を記録して `RestartRequired` を返す (プロセス再起動は呼び出し側 = シェルの責務)。
    ///
    /// capability は、その instance に宣言されたものを新しいファームウェアにもそのまま付与する
    /// (`CapabilityPlan::grant_for`)。差し替え先の sha256 は元と違うので、厳密には同意の鍵も変わる。
    /// ホット差替時の再同意をどう扱うかは M5 側と要整合 (08-decisions.md Q14)。それまでは
    /// 「宣言に同意済みの capability を同じ instance の別ビルドに引き継ぐ」挙動にしている。
    pub fn swap_firmware(
        &mut self,
        slot: &str,
        source: FirmwareSource,
    ) -> anyhow::Result<FirmwareSwap> {
        let i = *self
            .slots
            .get(slot)
            .ok_or_else(|| anyhow!("slot `{slot}` がありません"))?;
        if let FirmwareSource::Native(_) = &source {
            self.options
                .firmware_overrides
                .insert(slot.to_owned(), source);
            return Ok(FirmwareSwap::RestartRequired);
        }
        let mut opts = self.options.clone();
        opts.firmware_overrides
            .insert(slot.to_owned(), source.clone());
        let spec = &self.manifest.components[i];
        let w = match locate_firmware(&self.manifest_dir, &self.manifest, spec, &opts)? {
            FirmwareBytes::Wasm(w) => w,
            FirmwareBytes::Native(_) => unreachable!("native は上で返している"),
        };
        // 宣言と import の整合 (V7) をここでも確認する (launch 時と同じ規則)
        let (errors, _) = spec.capabilities().check_imports(&w.imports);
        if !errors.is_empty() {
            bail!(
                "slot `{slot}` のファームウェアは宣言にない import を持ちます: {}",
                errors.join("; ")
            );
        }
        let grant = if opts.validate_only {
            self.capability_plan.grant_for_validate(&spec.instance)
        } else {
            let dirs = DataDirs::resolve(opts.data_dir.as_deref());
            self.capability_plan.grant_for(&spec.instance, &dirs)?
        };
        let fw: Box<dyn Firmware> = Box::new(
            WasmFirmware::load_with(&self.runtime, &w.bytes, grant)
                .with_context(|| format!("ファームウェア {}", w.label))?,
        );
        let flavor = fw.flavor();
        let instance = spec.instance.clone();
        let caps = spec.capabilities();
        let cpu = self
            .engine
            .part_mut(i)
            .as_any_mut()
            .and_then(|a| a.downcast_mut::<CpuVirtual>())
            .ok_or_else(|| anyhow!("slot `{slot}` の部品は cpu-virtual ではありません"))?;
        cpu.attach_firmware(fw).with_context(|| {
            format!("slot `{slot}` のホット差替 (古いファームウェアを残しました)")
        })?;
        // describe の表示用に計画の entry も新しい実体に差し替える
        self.capability_plan
            .entries
            .retain(|e| e.instance != instance);
        self.capability_plan.push(PlanEntry {
            instance,
            artifact: w.label,
            artifact_sha256: w.sha256,
            capabilities: caps,
        });
        self.options = opts;
        Ok(FirmwareSwap::Hot { flavor })
    }

    /// 部品パラメータの変更。wasm 部品は manifest の params を更新して `init` し直す。
    pub fn set_param(
        &mut self,
        instance: &str,
        key: &str,
        value: serde_json::Value,
    ) -> anyhow::Result<(ParamChange, serde_json::Value)> {
        let i = self
            .engine
            .graph()
            .instance_index(instance)
            .ok_or_else(|| anyhow!("部品 `{instance}` がありません"))?;
        let spec = &mut self.manifest.components[i];
        let obj = spec
            .params
            .as_object_mut()
            .ok_or_else(|| anyhow!("部品 `{instance}` の params がオブジェクトではありません"))?;
        let previous = obj.insert(key.to_owned(), value.clone());
        if spec.is_builtin() {
            // 組み込み部品はコンストラクタで解釈するので、再 launch で反映する
            self.options
                .param_overrides
                .push((instance.to_owned(), key.to_owned(), value));
            return Ok((ParamChange::RestartRequired, spec.params.clone()));
        }
        let params = spec.params.clone();
        if let Err(e) = self.engine.part_mut(i).init(&params) {
            // 失敗したら元の値に戻す (部品側の状態は部品の init の実装に依る)
            let obj = self.manifest.components[i].params.as_object_mut().unwrap();
            match previous {
                Some(p) => {
                    obj.insert(key.to_owned(), p);
                }
                None => {
                    obj.remove(key);
                }
            }
            return Err(e.context(format!("部品 `{instance}` の init ({key})")));
        }
        self.options
            .param_overrides
            .retain(|(inst, k, _)| !(inst == instance && k == key));
        self.options
            .param_overrides
            .push((instance.to_owned(), key.to_owned(), value));
        Ok((ParamChange::Reinitialized, params))
    }

    /// 1 ステップ進める。
    pub fn step(&mut self) -> anyhow::Result<()> {
        self.engine.step().map_err(|e| anyhow!("{e}"))
    }

    /// 全部品を reset して時刻 0 へ (Epoch を進める)。
    pub fn reset(&mut self) -> anyhow::Result<()> {
        self.engine.reset()
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

/// ファームウェアの所在を決めて読む (実体化は capability の同意後)。
fn locate_firmware(
    manifest_dir: &Path,
    manifest: &Manifest,
    c: &ComponentSpec,
    opts: &SessionOptions,
) -> anyhow::Result<FirmwareBytes> {
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
            let path = resolve_artifact_path(manifest_dir, manifest, name)?;
            Ok(FirmwareBytes::Wasm(load_wasm(
                &path,
                name,
                manifest.artifacts[name].sha256.as_deref(),
            )?))
        }
        FirmwareSource::Wasm(p) => {
            anyhow::ensure!(
                slot.allow_override.iter().any(|a| a == "wasm") || opts.allow_unhashed,
                "slot `{}` は wasm の上書きを許していません",
                slot.slot
            );
            Ok(FirmwareBytes::Wasm(load_wasm(
                &p,
                &p.display().to_string(),
                None,
            )?))
        }
        FirmwareSource::Native(p) => {
            anyhow::ensure!(
                slot.allow_override.iter().any(|a| a == "native") || opts.allow_unhashed,
                "slot `{}` は native の上書きを許していません",
                slot.slot
            );
            Ok(FirmwareBytes::Native(p))
        }
    }
}

/// 実体化せずに同意の要否だけを列挙する (`ipemu-engine consent list`、シェルの事前表示用)。
pub fn list_consent_requests(
    manifest_path: &Path,
    opts: &SessionOptions,
) -> anyhow::Result<(Manifest, Vec<ConsentRequest>)> {
    let text = std::fs::read_to_string(manifest_path)
        .with_context(|| format!("{} を読めません", manifest_path.display()))?;
    let manifest: Manifest = Manifest::from_json(&text).context("manifest の構造が不正")?;
    let manifest_dir = manifest_path.parent().unwrap_or(Path::new("."));
    let mut plan = CapabilityPlan::default();
    for c in &manifest.components {
        match (&c.artifact, c.kind.as_deref()) {
            (Some(a), None) => {
                let path = resolve_artifact_path(manifest_dir, &manifest, a)?;
                let w = load_wasm(&path, a, manifest.artifacts[a].sha256.as_deref())?;
                plan.push(PlanEntry {
                    instance: c.instance.clone(),
                    artifact: a.clone(),
                    artifact_sha256: w.sha256,
                    capabilities: c.capabilities(),
                });
            }
            (None, Some("builtin:cpu-virtual")) => {
                if let FirmwareBytes::Wasm(w) = locate_firmware(manifest_dir, &manifest, c, opts)? {
                    plan.push(PlanEntry {
                        instance: c.instance.clone(),
                        artifact: w.label,
                        artifact_sha256: w.sha256,
                        capabilities: c.capabilities(),
                    });
                }
            }
            _ => {}
        }
    }
    let reqs = plan.requests(&manifest.id, manifest.revision);
    Ok((manifest, reqs))
}

// wasmtime の型名を短く
mod wasmtime_component {
    pub use ipemu_wasm::wasmtime::component::Component;
}
