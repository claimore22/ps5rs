use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceType {
    Sampler,
    Texture,
    ConstantBuffer,
    UnorderedAccessView,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceBinding {
    pub name: String,
    pub slot: u32,
    pub ty: ResourceType,
    pub bind_count: u32,
}

impl ResourceBinding {
    pub fn new(name: impl Into<String>, slot: u32, ty: ResourceType) -> Self {
        Self {
            name: name.into(),
            slot,
            ty,
            bind_count: 1,
        }
    }

    pub fn parse_from_binary(data: &[u8]) -> Vec<Self> {
        let mut out = Vec::new();
        let text = String::from_utf8_lossy(data);
        for (idx, line) in text.lines().enumerate() {
            if line.contains("s_sampler") || line.contains("Sampler") {
                out.push(Self::new(
                    format!("sampler_{}", idx),
                    idx as u32,
                    ResourceType::Sampler,
                ));
            } else if line.contains("Texture") || line.contains("t_") {
                out.push(Self::new(
                    format!("tex_{}", idx),
                    idx as u32,
                    ResourceType::Texture,
                ));
            } else if line.contains("cbuffer") || line.contains("CB") {
                out.push(Self::new(
                    format!("cb_{}", idx),
                    idx as u32,
                    ResourceType::ConstantBuffer,
                ));
            }
        }
        if out.is_empty() && !data.is_empty() {
            out.push(Self::new("default_sampler", 0, ResourceType::Sampler));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_resources() {
        let data = b"cbuffer MyCB { float4 x; }\nTexture2D myTex : t0;";
        let res = ResourceBinding::parse_from_binary(data);
        assert!(!res.is_empty());
    }

    #[test]
    fn empty_gives_default() {
        let res = ResourceBinding::parse_from_binary(b"binary blob");
        assert_eq!(res.len(), 1);
    }
}
