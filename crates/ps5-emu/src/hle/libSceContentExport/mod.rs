use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    ContentExportModule,
    "libSceContentExport",
    [
        "sceContentExportFinish",
        "sceContentExportFromData",
        "sceContentExportInit2",
        "sceContentExportStart",
        "sceContentExportTerm"
    ]
);
