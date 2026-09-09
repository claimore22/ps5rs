use crate::types::AbiType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackSignature {
    pub params: Vec<AbiType>,
}
