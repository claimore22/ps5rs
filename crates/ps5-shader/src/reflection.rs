use crate::resources::ResourceBinding;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VertexAttribute {
    pub name: String,
    pub location: u32,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub resources: Vec<ResourceBinding>,
    pub vertex_attributes: Vec<VertexAttribute>,
    pub render_targets: Vec<String>,
    pub samplers: Vec<ResourceBinding>,
    pub thread_group_size: Option<(u32, u32, u32)>,
}

impl Reflection {
    pub fn from_binary(data: &[u8]) -> Self {
        let resources = ResourceBinding::parse_from_binary(data);
        let samplers = resources
            .iter()
            .filter(|r| matches!(r.ty, crate::resources::ResourceType::Sampler))
            .cloned()
            .collect();
        let text = String::from_utf8_lossy(data);
        let mut attrs = Vec::new();
        for line in text.lines() {
            if line.contains("in ") && line.contains("POSITION") {
                attrs.push(VertexAttribute {
                    name: "POSITION".to_string(),
                    location: 0,
                    format: "R32G32B32_FLOAT".to_string(),
                });
            }
        }
        Self {
            resources,
            vertex_attributes: attrs,
            render_targets: vec![],
            samplers,
            thread_group_size: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.resources.is_empty() && self.vertex_attributes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflection_from_data() {
        let data = b"in float4 POSITION : POSITION;";
        let r = Reflection::from_binary(data);
        assert!(!r.vertex_attributes.is_empty());
    }

    #[test]
    fn empty_reflection() {
        let r = Reflection::from_binary(b"");
        assert!(r.vertex_attributes.is_empty());
    }
}
