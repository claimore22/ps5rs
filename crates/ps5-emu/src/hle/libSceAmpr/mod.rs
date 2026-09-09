use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    AmprModule,
    "libSceAmpr",
    [
        "sceAmprCommandBufferPushMarkerWithColor",
        "sceAmprCommandBufferSetBuffer",
        "sceAmprCommandBufferWriteAddressFromCounter_04_00",
        "sceAmprCommandBufferWriteAddressFromCounterPair_04_00",
        "sceAmprMeasureCommandSizeReadFile"
    ]
);