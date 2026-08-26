use crate::types::AbiType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub name: String,
    pub return_type: AbiType,
    pub params: Vec<AbiType>,
    pub variadic: bool,
}
