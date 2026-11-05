//! 部品の自己記述 (WIT の `part-desc` と同型)。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PortKind {
    Scalar,
    Bytes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortDesc {
    pub name: String,
    #[serde(default = "PortKind::scalar")]
    pub kind: PortKind,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub default: f64,
    #[serde(default)]
    pub doc: String,
}

impl PortKind {
    fn scalar() -> Self {
        PortKind::Scalar
    }
}

impl PortDesc {
    pub fn scalar(name: &str, unit: &str, doc: &str) -> Self {
        Self {
            name: name.into(),
            kind: PortKind::Scalar,
            unit: unit.into(),
            default: 0.0,
            doc: doc.into(),
        }
    }
    pub fn with_default(mut self, v: f64) -> Self {
        self.default = v;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ParamDesc {
    pub name: String,
    #[serde(default)]
    pub schema_json: String,
    #[serde(default)]
    pub doc: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PartDesc {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub inputs: Vec<PortDesc>,
    #[serde(default)]
    pub outputs: Vec<PortDesc>,
    #[serde(default)]
    pub params: Vec<ParamDesc>,
    #[serde(default = "default_true")]
    pub deterministic: bool,
    #[serde(default)]
    pub doc: String,
}

fn default_true() -> bool {
    true
}

impl PartDesc {
    pub fn input_index(&self, name: &str) -> Option<usize> {
        self.inputs.iter().position(|p| p.name == name)
    }
    pub fn output_index(&self, name: &str) -> Option<usize> {
        self.outputs.iter().position(|p| p.name == name)
    }
    pub fn scalar_input_count(&self) -> usize {
        self.inputs
            .iter()
            .filter(|p| p.kind == PortKind::Scalar)
            .count()
    }
    pub fn scalar_output_count(&self) -> usize {
        self.outputs
            .iter()
            .filter(|p| p.kind == PortKind::Scalar)
            .count()
    }
}
