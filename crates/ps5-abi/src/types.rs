use crate::calling_convention::CallingConvention;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AbiType {
    U32,
    U64,
    Ptr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub return_type: AbiType,
    pub params: Vec<AbiType>,
    pub convention: CallingConvention,
}
