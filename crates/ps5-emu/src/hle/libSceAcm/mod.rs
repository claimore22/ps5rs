use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(AcmModule, "libSceAcm", ["sceAcm_ConvReverb_SharedInput"]);