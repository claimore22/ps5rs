use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AbiType {
    U32,
    U64,
    I32,
    I64,
    F32,
    F64,
    Ptr,
    Void,
}
