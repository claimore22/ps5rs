use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    PngDecModule,
    "libScePngDec",
    [
        "scePngDecCreate",
        "scePngDecDecode",
        "scePngDecDelete",
        "scePngDecParseHeader",
        "scePngDecQueryMemorySize"
    ]
);