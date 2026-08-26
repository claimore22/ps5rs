use ps5_abi::{AbiType, calling_convention::CallingConvention, functions::FunctionSignature};

#[test]
fn fifty_signatures() {
    let mut sigs = Vec::new();
    for i in 0..50 {
        sigs.push(FunctionSignature {
            name: format!("func{}", i),
            return_type: AbiType::U64,
            params: vec![AbiType::U64, AbiType::U32],
            variadic: false,
        });
    }
    assert_eq!(sigs.len(), 50);
    assert_eq!(sigs[0].name, "func0");
}

#[test]
fn calling_convention_sysv() {
    let c = CallingConvention::SysV64;
    assert!(matches!(c, CallingConvention::SysV64));
}
