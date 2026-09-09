use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    FiberModule,
    "libSceFiber",
    [
        "_sceFiberInitializeImpl",
        "sceFiberFinalize",
        "sceFiberReturnToThread",
        "sceFiberRun",
        "sceFiberSwitch"
    ]
);