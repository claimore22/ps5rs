use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    PlayGoModule,
    "libScePlayGo",
    [
        "scePlayGoGetEta",
        "scePlayGoGetLocus",
        "scePlayGoGetProgress",
        "scePlayGoPrefetch"
    ]
);