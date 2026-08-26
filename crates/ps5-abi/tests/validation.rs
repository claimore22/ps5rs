use ps5_abi::{AbiType, FunctionSignature, calling_convention::CallingConvention};

#[test]
fn abi_types_exist() {
    let _ = AbiType::U32;
    let _ = AbiType::U64;
}

#[test]
fn function_signature_creates() {
    let sig = FunctionSignature {
        name: "test".to_string(),
        return_type: AbiType::U32,
        params: vec![AbiType::U64],
        variadic: false,
    };
    assert_eq!(sig.name, "test");
}
