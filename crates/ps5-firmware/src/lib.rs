use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareVersion {
    pub major: u32,
    pub minor: u32,
}
