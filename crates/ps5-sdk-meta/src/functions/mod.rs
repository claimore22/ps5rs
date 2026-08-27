use serde::{Deserialize, Serialize};

use crate::versions::VersionRange;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdkFunction {
    pub nid: String,
    pub name: String,
    pub library: String,
    pub module: Option<String>,
    pub sdk_versions: VersionRange,
    pub category: String,
}

impl SdkFunction {
    pub fn new(
        nid: impl Into<String>,
        name: impl Into<String>,
        library: impl Into<String>,
        module: Option<String>,
        sdk_versions: VersionRange,
        category: impl Into<String>,
    ) -> Self {
        Self {
            nid: nid.into(),
            name: name.into(),
            library: library.into(),
            module,
            sdk_versions: sdk_versions,
            category: category.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation() {
        let f = SdkFunction::new(
            "ABC",
            "sceKernelSleep",
            "libkernel",
            Some("libkernel.prx".to_string()),
            VersionRange::single("9.00"),
            "kernel",
        );
        assert_eq!(f.name, "sceKernelSleep");
    }
}
