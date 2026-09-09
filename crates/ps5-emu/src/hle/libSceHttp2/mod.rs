use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(Http2Module, "libSceHttp2", ["sceHttp2Init", "sceHttp2Term"]);