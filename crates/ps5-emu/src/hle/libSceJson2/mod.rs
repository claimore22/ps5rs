use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    Json2Module,
    "libSceJson2",
    [
        "_ZN3sce4Json11Initializer9terminateEv",
        "_ZN3sce4Json12MemAllocatorC2Ev",
        "_ZN3sce4Json12MemAllocatorD2Ev",
        "_ZN3sce4Json6ObjectC1Ev",
        "_ZNK3sce4Json5Value9getStringEv"
    ]
);