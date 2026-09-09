use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    ShareModule,
    "libSceShare",
    [
        "sceShareFeaturePermit",
        "sceShareFeatureProhibit",
        "sceShareInitialize",
        "sceShareSetScreenshotOverlayImage",
        "sceShareTerminate"
    ]
);