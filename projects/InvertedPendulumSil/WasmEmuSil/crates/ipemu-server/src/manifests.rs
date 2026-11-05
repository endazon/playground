//! manifest の版管理ロジック: 登録済みアーティファクトでの補完と検証、承認、公開 (署名)。
//! 配信 API と管理 API の両方から使う。

use crate::auth::Principal;
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crate::util;
use ipemu_core::manifest::{ComponentSpec, Manifest};
use ipemu_core::validate::{validate, validate_schema, Diagnostics, Severity, ValidateOptions};
use ipemu_core::PartDesc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::collections::HashMap;

/// 検証結果 (API / 画面に返す形)。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub ok: bool,
    pub errors: Vec<DiagItem>,
    pub warnings: Vec<DiagItem>,
    /// 検証に使った部品の記述 (トポロジ図の描画用)
    pub descs: Vec<InstanceDesc>,
    pub validated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagItem {
    pub rule: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceDesc {
    pub instance: String,
    pub desc: PartDesc,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct ArtifactRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: String,
    pub sha256: String,
    pub size: i64,
    pub content_type: String,
    pub world: String,
    pub describe_json: Option<String>,
    pub wasi_imports: String,
    pub imports: String,
    pub uploaded_by: String,
    pub created_at: String,
}

pub async fn load_artifact(pool: &SqlitePool, name_ver: &str) -> ApiResult<Option<ArtifactRow>> {
    let Some((name, ver)) = name_ver.split_once('@') else {
        return Ok(None);
    };
    Ok(
        sqlx::query_as::<_, ArtifactRow>("SELECT * FROM artifacts WHERE name = ? AND version = ?")
            .bind(name)
            .bind(ver)
            .fetch_optional(pool)
            .await?,
    )
}

/// 本文の `artifacts` を登録済みアーティファクトで補完する (sha256 / url / size / kind)。
/// 本文が別の sha256 を主張していれば矛盾として弾く。未登録のものはそのまま残し、検証 (V2 / V3) に任せる。
pub async fn enrich_artifacts(pool: &SqlitePool, body: &mut Value) -> ApiResult<Vec<String>> {
    let mut notes = Vec::new();
    let Some(arts) = body.get_mut("artifacts").and_then(|a| a.as_object_mut()) else {
        return Ok(notes);
    };
    let keys: Vec<String> = arts.keys().cloned().collect();
    for key in keys {
        let Some(row) = load_artifact(pool, &key).await? else {
            notes.push(format!("アーティファクト `{key}` は未登録です"));
            continue;
        };
        let entry = arts.get_mut(&key).unwrap();
        let Some(obj) = entry.as_object_mut() else {
            continue;
        };
        if let Some(h) = obj.get("sha256").and_then(|v| v.as_str()) {
            if h != row.sha256 {
                return Err(ApiError::unprocessable(format!(
                    "アーティファクト `{key}` の sha256 が登録内容と一致しません (本文 {h}, 登録 {})",
                    row.sha256
                )));
            }
        }
        obj.insert("sha256".into(), json!(row.sha256));
        obj.insert("size".into(), json!(row.size));
        obj.insert("kind".into(), json!(row.kind));
        obj.remove("path");
        obj.insert(
            "url".into(),
            json!(format!("/api/v1/artifacts/{}/{}", row.name, row.version)),
        );
    }
    Ok(notes)
}

/// wasi import とcapability 宣言の対応 (V7 / V8)。ipemu-core の validate は WIT を読まないので、
/// サーバはアップロード時に保存した import 一覧でここを補う。
fn wasi_rules(
    c: &ComponentSpec,
    wasi_imports: &[String],
    diags: &mut Vec<DiagItem>,
    warns: &mut Vec<DiagItem>,
) {
    let caps = c.capabilities.clone().unwrap_or(json!({}));
    let declared = |group: &str| -> bool {
        match group {
            "clocks" => caps.get("clock").and_then(|v| v.as_bool()) == Some(true),
            "random" => caps.get("random").and_then(|v| v.as_bool()) == Some(true),
            "sockets" => caps
                .get("net")
                .and_then(|n| n.as_object())
                .map(|n| {
                    n.values()
                        .any(|v| v.as_array().is_some_and(|a| !a.is_empty()))
                })
                .unwrap_or(false),
            "filesystem" => caps
                .get("fs")
                .and_then(|v| v.as_array())
                .is_some_and(|a| !a.is_empty()),
            _ => false,
        }
    };
    let mut used = std::collections::BTreeSet::new();
    for imp in wasi_imports {
        // wasi:clocks/wall-clock@0.2.0 → clocks
        let group = imp
            .trim_start_matches("wasi:")
            .split(['/', '@'])
            .next()
            .unwrap_or("");
        let group = match group {
            "clocks" | "random" | "sockets" | "filesystem" => group,
            // io / cli は上の 4 つに付随して来るものとして扱う
            "io" | "cli" => continue,
            other => {
                diags.push(DiagItem {
                    rule: "V7".into(),
                    message: format!(
                        "部品 `{}`: import `{imp}` ({other}) は capability で表現できません",
                        c.instance
                    ),
                });
                continue;
            }
        };
        used.insert(group);
        if !declared(group) {
            diags.push(DiagItem {
                rule: "V7".into(),
                message: format!(
                    "部品 `{}` は `{imp}` を import しますが capabilities で宣言されていません",
                    c.instance
                ),
            });
        }
    }
    for g in ["clocks", "random", "sockets", "filesystem"] {
        if declared(g) && !used.contains(g) {
            warns.push(DiagItem {
                rule: "V8".into(),
                message: format!("部品 `{}`: capability `{g}` は宣言されていますが部品は import していません (過剰宣言)", c.instance),
            });
        }
    }
}

/// 本文を検証する。V1 (スキーマ) → V2〜V11 (ipemu-core) → V7/V8 (サーバ側の import 照合)。
pub async fn validate_body(pool: &SqlitePool, body: &Value) -> ApiResult<ValidationReport> {
    let mut diags = Diagnostics::default();
    validate_schema(body, &mut diags);
    let mut report = ValidationReport {
        validated_at: util::now_str(),
        ..Default::default()
    };
    let push = |report: &mut ValidationReport, d: &Diagnostics| {
        for i in &d.items {
            let item = DiagItem {
                rule: i.rule.to_owned(),
                message: i.message.clone(),
            };
            match i.severity {
                Severity::Error => report.errors.push(item),
                Severity::Warning => report.warnings.push(item),
            }
        }
    };
    if diags.has_errors() {
        push(&mut report, &diags);
        return Ok(report);
    }
    let manifest: Manifest = match serde_json::from_value(body.clone()) {
        Ok(m) => m,
        Err(e) => {
            report.errors.push(DiagItem {
                rule: "V1".into(),
                message: format!("構造が不正: {e}"),
            });
            return Ok(report);
        }
    };

    // 登録済みアーティファクトの describe を集める
    let mut descs: HashMap<String, (PartDesc, Vec<String>)> = HashMap::new();
    for c in &manifest.components {
        if let Some(a) = &c.artifact {
            if descs.contains_key(a) {
                continue;
            }
            if let Some(row) = load_artifact(pool, a).await? {
                if row.kind != "part" {
                    report.errors.push(DiagItem {
                        rule: "V3".into(),
                        message: format!(
                            "部品 `{}`: `{a}` は {} であり part ではありません",
                            c.instance, row.kind
                        ),
                    });
                    continue;
                }
                let wasi: Vec<String> = serde_json::from_str(&row.wasi_imports).unwrap_or_default();
                match row.describe_json.as_deref().map(serde_json::from_str::<PartDesc>) {
                    Some(Ok(d)) => {
                        descs.insert(a.clone(), (d, wasi));
                    }
                    _ => report.errors.push(DiagItem {
                        rule: "V3".into(),
                        message: format!("部品 `{}`: `{a}` の describe が保存されていません (wasi を import する部品の検証は M5)", c.instance),
                    }),
                }
            } else {
                report.errors.push(DiagItem {
                    rule: "V2".into(),
                    message: format!("部品 `{}`: アーティファクト `{a}` は未登録です", c.instance),
                });
            }
        }
        if let Some(fw) = &c.firmware {
            if let Some(def) = &fw.default {
                match load_artifact(pool, def).await? {
                    Some(row) if row.kind == "firmware" => {
                        if !row.world.is_empty() && row.world != fw.world {
                            report.errors.push(DiagItem {
                                rule: "V3".into(),
                                message: format!("部品 `{}`: firmware `{def}` のワールド {} が slot の {} と一致しません", c.instance, row.world, fw.world),
                            });
                        }
                    }
                    Some(row) => report.errors.push(DiagItem {
                        rule: "V3".into(),
                        message: format!(
                            "部品 `{}`: `{def}` は {} であり firmware ではありません",
                            c.instance, row.kind
                        ),
                    }),
                    None => report.errors.push(DiagItem {
                        rule: "V2".into(),
                        message: format!("部品 `{}`: firmware `{def}` は未登録です", c.instance),
                    }),
                }
            }
        }
    }
    if let Some(ui) = manifest
        .ui
        .as_ref()
        .and_then(|u| u.get("bundle"))
        .and_then(|b| b.as_str())
    {
        match load_artifact(pool, ui).await? {
            Some(row) if row.kind == "ui" => {}
            Some(_) => report.errors.push(DiagItem {
                rule: "V3".into(),
                message: format!("ui.bundle `{ui}` は ui ではありません"),
            }),
            None => report.errors.push(DiagItem {
                rule: "V2".into(),
                message: format!("ui.bundle `{ui}` は未登録です"),
            }),
        }
        if !manifest.artifacts.contains_key(ui) {
            report.errors.push(DiagItem {
                rule: "V2".into(),
                message: format!("ui.bundle `{ui}` が artifacts にありません"),
            });
        }
    }
    if !report.errors.is_empty() {
        return Ok(report);
    }

    let mut collected: Vec<InstanceDesc> = Vec::new();
    let mut resolver = |spec: &ComponentSpec| -> anyhow::Result<PartDesc> {
        let d = match (&spec.artifact, spec.kind.as_deref()) {
            (Some(a), None) => descs
                .get(a)
                .map(|(d, _)| d.clone())
                .ok_or_else(|| anyhow::anyhow!("`{a}` の記述がありません"))?,
            (None, Some("builtin:cpu-virtual")) => {
                let p = ipemu_core::builtins::CpuVirtualParams::from_json(&spec.params)?;
                ipemu_core::builtins::CpuVirtual::describe_for(&p)
            }
            (None, Some(k)) => ipemu_core::builtins::describe_builtin(k, &spec.params)?,
            _ => anyhow::bail!("artifact と kind のどちらか一方が必要"),
        };
        collected.push(InstanceDesc {
            instance: spec.instance.clone(),
            desc: d.clone(),
        });
        Ok(d)
    };
    let (d2, _graph) = validate(
        &manifest,
        &mut resolver,
        ValidateOptions {
            allow_unhashed: false,
        },
    );
    push(&mut report, &d2);
    report.descs = collected;

    // V7 / V8
    for c in &manifest.components {
        if let Some(a) = &c.artifact {
            if let Some((_, wasi)) = descs.get(a) {
                wasi_rules(c, wasi, &mut report.errors, &mut report.warnings);
            }
        }
    }
    report.ok = report.errors.is_empty();
    Ok(report)
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionRow {
    pub manifest_id: String,
    pub revision: i64,
    pub state: String,
    pub body: String,
    pub body_sha256: String,
    pub validation: Option<String>,
    pub created_by: String,
    pub updated_by: String,
    pub created_at: String,
    pub updated_at: String,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
    pub approval_reason: Option<String>,
    pub capabilities_hash: Option<String>,
    pub published_by: Option<String>,
    pub published_at: Option<String>,
    pub signed_body: Option<String>,
    pub key_id: Option<String>,
}

impl RevisionRow {
    /// API に返す形 (body / validation を JSON として埋め込む)。
    pub fn to_json(&self, include_body: bool) -> Value {
        let mut v = json!({
            "manifestId": self.manifest_id,
            "revision": self.revision,
            "state": self.state,
            "bodySha256": self.body_sha256,
            "createdBy": self.created_by,
            "updatedBy": self.updated_by,
            "createdAt": self.created_at,
            "updatedAt": self.updated_at,
            "approvedBy": self.approved_by,
            "approvedAt": self.approved_at,
            "approvalReason": self.approval_reason,
            "capabilitiesHash": self.capabilities_hash,
            "publishedBy": self.published_by,
            "publishedAt": self.published_at,
            "keyId": self.key_id,
            "validation": self.validation.as_deref().and_then(|s| serde_json::from_str::<Value>(s).ok()),
        });
        if include_body {
            v["body"] = serde_json::from_str(&self.body).unwrap_or(Value::Null);
            v["signedBody"] = self
                .signed_body
                .as_deref()
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .unwrap_or(Value::Null);
        }
        v
    }
    pub fn name(&self) -> String {
        serde_json::from_str::<Value>(&self.body)
            .ok()
            .and_then(|b| b.get("name").and_then(|n| n.as_str()).map(str::to_owned))
            .unwrap_or_default()
    }
}

pub async fn load_revision(pool: &SqlitePool, id: &str, rev: i64) -> ApiResult<RevisionRow> {
    sqlx::query_as::<_, RevisionRow>(
        "SELECT * FROM manifest_revisions WHERE manifest_id = ? AND revision = ?",
    )
    .bind(id)
    .bind(rev)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::not_found(format!("構成 {id} の revision {rev} がありません")))
}

/// 承認: 直近の検証が通っていること、承認者が最終編集者と別人であること (editor ≠ approver)。
pub async fn approve(
    state: &AppState,
    actor: &Principal,
    id: &str,
    rev: i64,
    reason: &str,
) -> ApiResult<RevisionRow> {
    actor.require(actor.can_approve(), "承認")?;
    let row = load_revision(&state.pool, id, rev).await?;
    if row.state != "draft" {
        return Err(ApiError::conflict(format!(
            "revision {rev} は {} のため承認できません",
            row.state
        )));
    }
    if row.updated_by == actor.id {
        return Err(ApiError::forbidden(
            "編集した本人は承認できません (editor ≠ approver)",
        ));
    }
    let body: Value = serde_json::from_str(&row.body)?;
    // 承認時点で改めて検証する (部品の登録状況が変わっているかもしれない)
    let report = validate_body(&state.pool, &body).await?;
    if !report.ok {
        return Err(ApiError::unprocessable(format!(
            "検証エラーがあるため承認できません: {}",
            report
                .errors
                .iter()
                .map(|e| format!("[{}] {}", e.rule, e.message))
                .collect::<Vec<_>>()
                .join("; ")
        )));
    }
    let cap_hash =
        ipemu_sign::capabilities_hash(&body).map_err(|e| ApiError::internal(e.to_string()))?;
    let now = util::now_str();
    sqlx::query(
        "UPDATE manifest_revisions SET state = 'approved', approved_by = ?, approved_at = ?, approval_reason = ?, capabilities_hash = ?, validation = ?, updated_at = ?
         WHERE manifest_id = ? AND revision = ?",
    )
    .bind(&actor.name)
    .bind(&now)
    .bind(reason)
    .bind(&cap_hash)
    .bind(serde_json::to_string(&report)?)
    .bind(&now)
    .bind(id)
    .bind(rev)
    .execute(&state.pool)
    .await?;
    crate::audit::record(
        &state.pool,
        actor,
        "manifest.approve",
        &format!("{id}@{rev}"),
        json!({"reason": reason, "capabilitiesHash": cap_hash}),
    )
    .await?;
    load_revision(&state.pool, id, rev).await
}

/// 公開: approval を本文に付け、JCS 正規化して Ed25519 署名し、signed_body に保存する。
/// 既定 revision が未設定なら、この revision を既定にする。
pub async fn publish(
    state: &AppState,
    actor: &Principal,
    id: &str,
    rev: i64,
) -> ApiResult<RevisionRow> {
    actor.require(actor.can_approve(), "公開")?;
    let row = load_revision(&state.pool, id, rev).await?;
    if row.state != "approved" {
        return Err(ApiError::conflict(format!(
            "revision {rev} は {} のため公開できません (承認が必要)",
            row.state
        )));
    }
    let mut body: Value = serde_json::from_str(&row.body)?;
    let cap_hash =
        ipemu_sign::capabilities_hash(&body).map_err(|e| ApiError::internal(e.to_string()))?;
    if Some(cap_hash.as_str()) != row.capabilities_hash.as_deref() {
        return Err(ApiError::conflict(
            "承認後に本文が変わっています。新しい下書きとして承認し直してください",
        ));
    }
    let mut approval = json!({
        "approvedBy": row.approved_by,
        "approvedAt": row.approved_at,
        "capabilitiesHash": cap_hash,
    });
    if let Some(r) = row.approval_reason.as_deref().filter(|r| !r.is_empty()) {
        approval["reason"] = json!(r);
    }
    body["approval"] = approval;
    let signed = ipemu_sign::sign_manifest(&body, &state.signing_key, &state.key_id)
        .map_err(|e| ApiError::internal(e.to_string()))?;
    // 署名後の全文がスキーマに適合することも確かめる (approval / signature を含めて)
    let mut diags = Diagnostics::default();
    validate_schema(&signed, &mut diags);
    if diags.has_errors() {
        return Err(ApiError::internal(format!(
            "署名後の manifest がスキーマに適合しません: {diags}"
        )));
    }
    let now = util::now_str();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE manifest_revisions SET state = 'published', published_by = ?, published_at = ?, signed_body = ?, key_id = ?, updated_at = ?
         WHERE manifest_id = ? AND revision = ?",
    )
    .bind(&actor.name)
    .bind(&now)
    .bind(signed.to_string())
    .bind(&state.key_id)
    .bind(&now)
    .bind(id)
    .bind(rev)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE manifests SET default_revision = COALESCE(default_revision, ?), name = ?, description = ?, updated_at = ? WHERE id = ?",
    )
    .bind(rev)
    .bind(body.get("name").and_then(|v| v.as_str()).unwrap_or(""))
    .bind(body.get("description").and_then(|v| v.as_str()).unwrap_or(""))
    .bind(&now)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    crate::audit::record(
        &state.pool,
        actor,
        "manifest.publish",
        &format!("{id}@{rev}"),
        json!({"keyId": state.key_id}),
    )
    .await?;
    load_revision(&state.pool, id, rev).await
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct ManifestRow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub visibility: String,
    pub visibility_groups: String,
    pub default_revision: Option<i64>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

impl ManifestRow {
    /// 利用者の公開範囲に入っているか。
    pub fn visible_to(&self, p: &Principal) -> bool {
        match p.kind {
            crate::auth::PrincipalKind::ApiKey => p.api_key_allows(&self.id),
            crate::auth::PrincipalKind::User => {
                if self.visibility == "all" {
                    return true;
                }
                let groups: Vec<String> = util::json_vec(&self.visibility_groups);
                groups.iter().any(|g| p.groups.contains(g))
            }
        }
    }
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "visibility": self.visibility,
            "visibilityGroups": util::json_vec(&self.visibility_groups),
            "defaultRevision": self.default_revision,
            "createdBy": self.created_by,
            "createdAt": self.created_at,
            "updatedAt": self.updated_at,
        })
    }
}

pub async fn load_manifest(pool: &SqlitePool, id: &str) -> ApiResult<ManifestRow> {
    sqlx::query_as::<_, ManifestRow>("SELECT * FROM manifests WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("構成 {id} がありません")))
}

/// カタログに出す「要求 capability」の要約 (instance ごと)。
pub fn capability_summary(body: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    if let Some(comps) = body.get("components").and_then(|c| c.as_array()) {
        for c in comps {
            if let Some(cap) = c.get("capabilities") {
                out.push(json!({"instance": c.get("instance"), "capabilities": cap}));
            }
        }
    }
    out
}
