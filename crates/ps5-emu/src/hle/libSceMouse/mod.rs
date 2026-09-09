use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    MouseModule,
    "libSceMouse",
    ["sceMouseClose", "sceMouseInit", "sceMouseOpen", "sceMouseRead"]
);