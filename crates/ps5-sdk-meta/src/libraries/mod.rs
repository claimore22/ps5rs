use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleKind {
    System,
    User,
    Sprx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryInfo {
    pub name: String,
    pub kind: ModuleKind,
    pub category: String,
}

impl LibraryInfo {
    pub fn new(name: impl Into<String>, kind: ModuleKind, category: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind,
            category: category.into(),
        }
    }
}

pub fn known_libraries() -> Vec<LibraryInfo> {
    vec![
        LibraryInfo::new("libkernel", ModuleKind::System, "kernel"),
        LibraryInfo::new("libc", ModuleKind::System, "kernel"),
        LibraryInfo::new("libScePad", ModuleKind::System, "input"),
        LibraryInfo::new("libSceVideoOut", ModuleKind::System, "graphics"),
        LibraryInfo::new("libSceAudioOut", ModuleKind::System, "audio"),
        LibraryInfo::new("libSceGnmDriver", ModuleKind::System, "graphics"),
        LibraryInfo::new("libSceAgc", ModuleKind::System, "graphics"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_not_empty() {
        assert!(!known_libraries().is_empty());
    }
}
