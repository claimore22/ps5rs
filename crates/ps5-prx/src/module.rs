use crate::metadata::PrxMetadata;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleType {
    Eboot,
    Prx,
    Sprx,
    SelfModule,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrxModule {
    pub name: String,
    pub module_type: ModuleType,
    pub metadata: PrxMetadata,
    pub entry_point: u64,
}
