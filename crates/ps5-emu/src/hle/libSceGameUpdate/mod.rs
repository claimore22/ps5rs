use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    GameUpdateModule,
    "libSceGameUpdate",
    [
        "sceGameUpdateCheck",
        "sceGameUpdateCreateRequest",
        "sceGameUpdateDeleteRequest",
        "sceGameUpdateInitialize",
        "sceGameUpdateTerminate"
    ]
);
