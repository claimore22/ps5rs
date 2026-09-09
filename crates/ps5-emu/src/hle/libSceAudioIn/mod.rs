use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    AudioInModule,
    "libSceAudioIn",
    [
        "sceAudioInClose",
        "sceAudioInGetSilentState",
        "sceAudioInInput",
        "sceAudioInOpen"
    ]
);
