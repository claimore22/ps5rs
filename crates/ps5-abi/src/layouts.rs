use crate::structs::StructLayout;

pub fn known_layouts() -> Vec<StructLayout> {
    vec![
        StructLayout {
            name: "SceKernelTimespec".to_string(),
            size: 16,
            align: 8,
        },
        StructLayout {
            name: "SceKernelScePthreadAttr".to_string(),
            size: 56,
            align: 8,
        },
        StructLayout {
            name: "SceVideoOutBufferAttribute".to_string(),
            size: 32,
            align: 8,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_not_empty() {
        assert!(!known_layouts().is_empty());
    }
}
