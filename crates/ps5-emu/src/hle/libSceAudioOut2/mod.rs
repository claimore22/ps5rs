use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    AudioOut2Module,
    "libSceAudioOut2",
    [
        "sceAudioOut2ContextAdvance",
        "sceAudioOut2ContextResetParam"
    ]
);
