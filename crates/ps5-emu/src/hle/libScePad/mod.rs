use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    PadModule,
    "libScePad",
    [
        "scePadGetHandle",
        "scePadInit",
        "scePadOpen",
        "scePadReadState",
        "scePadResetLightBar",
        "scePadSetLightBar",
        "scePadSetTriggerEffect",
        "scePadSetVibration",
        "scePadSetVibrationMode"
    ]
);
