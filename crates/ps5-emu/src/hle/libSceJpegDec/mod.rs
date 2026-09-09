use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    JpegDecModule,
    "libSceJpegDec",
    [
        "sceJpegDecCreate",
        "sceJpegDecDecode",
        "sceJpegDecDelete",
        "sceJpegDecQueryMemorySize"
    ]
);
