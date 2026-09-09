use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantInfo {
    pub name: String,
    pub value: i64,
    pub description: String,
}

pub fn known_constants() -> HashMap<String, ConstantInfo> {
    let mut m = HashMap::new();
    m.insert(
        "SCE_KERNEL_ERROR_EAGAIN".to_string(),
        ConstantInfo {
            name: "SCE_KERNEL_ERROR_EAGAIN".to_string(),
            value: 0x80020005,
            description: "Resource temporarily unavailable".to_string(),
        },
    );
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_present() {
        assert!(known_constants().contains_key("SCE_KERNEL_ERROR_EAGAIN"));
    }
}
