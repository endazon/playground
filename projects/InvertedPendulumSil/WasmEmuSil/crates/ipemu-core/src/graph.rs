//! 配線グラフと実行順。

use crate::desc::{PartDesc, PortKind};
use crate::manifest::{split_port_ref, Manifest, Wire};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("配線 `{0}` の書式が不正です (instance.port)")]
    BadPortRef(String),
    #[error("配線 `{0}`: 部品 `{1}` がありません")]
    UnknownInstance(String, String),
    #[error("配線 `{0}`: 部品 `{1}` に出力ポート `{2}` がありません")]
    UnknownOutput(String, String, String),
    #[error("配線 `{0}`: 部品 `{1}` に入力ポート `{2}` がありません")]
    UnknownInput(String, String, String),
    #[error("入力ポート `{0}` に複数の配線があります")]
    DuplicateInput(String),
    #[error("latency 0 の配線だけで閉路ができています: {0}")]
    ZeroLatencyCycle(String),
    #[error("部品名 `{0}` が重複しています")]
    DuplicateInstance(String),
    #[error("bytes ポートの配線 `{0}` は v1 では未対応です")]
    BytesPortUnsupported(String),
}

/// 解決済みの配線。
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedWire {
    pub from_inst: usize,
    pub from_port: usize,
    pub to_inst: usize,
    pub to_port: usize,
    pub latency: u8,
    pub gain: f64,
    pub offset: f64,
    pub label: String,
}

/// manifest と各部品の記述から作る実行計画。
#[derive(Debug, Clone)]
pub struct Graph {
    pub instances: Vec<String>,
    pub descs: Vec<PartDesc>,
    pub wires: Vec<ResolvedWire>,
    /// latency 0 の辺でトポロジカルソートした実行順 (manifest 順で安定化)
    pub order: Vec<usize>,
    pub rate_dividers: Vec<u32>,
}

impl Graph {
    pub fn build(manifest: &Manifest, descs: Vec<PartDesc>) -> Result<Graph, GraphError> {
        assert_eq!(manifest.components.len(), descs.len());
        let mut index: HashMap<&str, usize> = HashMap::new();
        for (i, c) in manifest.components.iter().enumerate() {
            if index.insert(c.instance.as_str(), i).is_some() {
                return Err(GraphError::DuplicateInstance(c.instance.clone()));
            }
        }

        let mut wires = Vec::with_capacity(manifest.wires.len());
        let mut used_inputs: HashMap<(usize, usize), ()> = HashMap::new();
        for w in &manifest.wires {
            let r = resolve_wire(w, &index, &descs)?;
            if used_inputs.insert((r.to_inst, r.to_port), ()).is_some() {
                return Err(GraphError::DuplicateInput(w.to.clone()));
            }
            wires.push(r);
        }

        let order = topological_order(
            manifest.components.len(),
            &wires,
            &manifest
                .components
                .iter()
                .map(|c| c.instance.clone())
                .collect::<Vec<_>>(),
        )?;

        Ok(Graph {
            instances: manifest
                .components
                .iter()
                .map(|c| c.instance.clone())
                .collect(),
            descs,
            wires,
            order,
            rate_dividers: manifest
                .components
                .iter()
                .map(|c| c.rate_divider.max(1))
                .collect(),
        })
    }

    pub fn instance_index(&self, name: &str) -> Option<usize> {
        self.instances.iter().position(|n| n == name)
    }

    /// `instance.port` を (instance index, output index) に解決する (スナップショット・記録用)。
    pub fn resolve_output(&self, port_ref: &str) -> Option<(usize, usize)> {
        let (inst, port) = split_port_ref(port_ref)?;
        let i = self.instance_index(inst)?;
        let p = self.descs[i].output_index(port)?;
        Some((i, p))
    }
}

fn resolve_wire(
    w: &Wire,
    index: &HashMap<&str, usize>,
    descs: &[PartDesc],
) -> Result<ResolvedWire, GraphError> {
    let label = format!("{} -> {}", w.from, w.to);
    let (fi, fp) = split_port_ref(&w.from).ok_or_else(|| GraphError::BadPortRef(w.from.clone()))?;
    let (ti, tp) = split_port_ref(&w.to).ok_or_else(|| GraphError::BadPortRef(w.to.clone()))?;
    let from_inst = *index
        .get(fi)
        .ok_or_else(|| GraphError::UnknownInstance(label.clone(), fi.into()))?;
    let to_inst = *index
        .get(ti)
        .ok_or_else(|| GraphError::UnknownInstance(label.clone(), ti.into()))?;
    let from_port = descs[from_inst]
        .output_index(fp)
        .ok_or_else(|| GraphError::UnknownOutput(label.clone(), fi.into(), fp.into()))?;
    let to_port = descs[to_inst]
        .input_index(tp)
        .ok_or_else(|| GraphError::UnknownInput(label.clone(), ti.into(), tp.into()))?;
    if descs[from_inst].outputs[from_port].kind == PortKind::Bytes
        || descs[to_inst].inputs[to_port].kind == PortKind::Bytes
    {
        return Err(GraphError::BytesPortUnsupported(label));
    }
    Ok(ResolvedWire {
        from_inst,
        from_port,
        to_inst,
        to_port,
        latency: if w.latency == 0 { 0 } else { 1 },
        gain: w.gain,
        offset: w.offset,
        label,
    })
}

/// Kahn 法。同順位は manifest の記載順。閉路があればその構成部品名を返す。
fn topological_order(
    n: usize,
    wires: &[ResolvedWire],
    names: &[String],
) -> Result<Vec<usize>, GraphError> {
    let mut indegree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for w in wires.iter().filter(|w| w.latency == 0) {
        if w.from_inst == w.to_inst {
            return Err(GraphError::ZeroLatencyCycle(names[w.from_inst].clone()));
        }
        adj[w.from_inst].push(w.to_inst);
        indegree[w.to_inst] += 1;
    }
    let mut order = Vec::with_capacity(n);
    let mut done = vec![false; n];
    // manifest 順で最初の「入次数 0 かつ未処理」を取る (安定)
    while let Some(next) = (0..n).find(|&i| !done[i] && indegree[i] == 0) {
        done[next] = true;
        order.push(next);
        for &t in &adj[next] {
            indegree[t] -= 1;
        }
    }
    if order.len() != n {
        let cyclic: Vec<&str> = (0..n)
            .filter(|&i| !done[i])
            .map(|i| names[i].as_str())
            .collect();
        return Err(GraphError::ZeroLatencyCycle(cyclic.join(", ")));
    }
    Ok(order)
}
