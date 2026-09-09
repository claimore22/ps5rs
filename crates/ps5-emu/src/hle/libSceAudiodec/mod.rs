use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    AudiodecModule,
    "libSceAudiodec",
    [
        "sceAudiodecClearContext",
        "sceAudiodecCreateDecoder",
        "sceAudiodecDecode",
        "sceAudiodecDeleteDecoder",
        "sceAudiodecInitLibrary",
        "sceAudiodecTermLibrary"
    ]
);