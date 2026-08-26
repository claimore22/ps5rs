use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraphSnapshot {
    pub nodes: Vec<String>,
    pub edges: Vec<(String, String)>,
}
