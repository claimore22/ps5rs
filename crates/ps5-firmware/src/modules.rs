use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FirmwareModule {
    pub name: String,
    pub path: String,
    pub version: String,
    pub exports_count: usize,
}

impl FirmwareModule {
    pub fn new(
        name: impl Into<String>,
        path: impl Into<String>,
        version: impl Into<String>,
        exports_count: usize,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            version: version.into(),
            exports_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation() {
        let m = FirmwareModule::new("libkernel.prx", "/path", "1.0", 10);
        assert_eq!(m.name, "libkernel.prx");
    }
}
