use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    NpTrophy2Module,
    "libSceNpTrophy2",
    [
        "sceNpTrophy2CreateContext",
        "sceNpTrophy2CreateHandle",
        "sceNpTrophy2GetTrophyInfo",
        "sceNpTrophy2RegisterContext",
        "sceNpTrophy2ShowTrophyList"
    ]
);
