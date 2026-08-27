use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub offset: usize,
    pub size: usize,
    pub ty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructInfo {
    pub name: String,
    pub size: usize,
    pub fields: Vec<FieldInfo>,
}

pub fn known_structs() -> Vec<StructInfo> {
    vec![
        StructInfo {
            name: "SceKernelEventFlag".to_string(),
            size: 0x20,
            fields: vec![
                FieldInfo {
                    name: "attr".to_string(),
                    offset: 0,
                    size: 4,
                    ty: "u32".to_string(),
                },
                FieldInfo {
                    name: "flags".to_string(),
                    offset: 4,
                    size: 4,
                    ty: "u32".to_string(),
                },
            ],
        },
        StructInfo {
            name: "SceKernelSema".to_string(),
            size: 0x18,
            fields: vec![],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structs_not_empty() {
        assert!(!known_structs().is_empty());
    }
}
