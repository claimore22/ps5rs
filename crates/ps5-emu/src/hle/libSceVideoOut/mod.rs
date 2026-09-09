use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    VideoOutModule,
    "libSceVideoOut",
    [
        "sceVideoOutAdjustColor_",
        "sceVideoOutColorSettingsSetGamma_",
        "sceVideoOutOpen",
        "sceVideoOutRegisterBuffers2",
        "sceVideoOutSetBufferAttribute2",
        "sceVideoOutSetFlipRate"
    ]
);
