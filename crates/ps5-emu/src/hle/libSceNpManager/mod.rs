use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    NpManagerModule,
    "libSceNpManager",
    [
        "sceNpCheckPremium",
        "sceNpHasSignedUp",
        "sceNpNotifyPremiumFeature"
    ]
);
