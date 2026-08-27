use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FirmwareLibrary {
    pub name: String,
    pub version: String,
    pub modules: Vec<String>,
}

impl FirmwareLibrary {
    pub fn new(name: impl Into<String>, version: impl Into<String>, modules: Vec<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            modules,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation() {
        let lib = FirmwareLibrary::new("libScePad", "1.0", vec!["libScePad.prx".to_string()]);
        assert_eq!(lib.name, "libScePad");
    }
}
