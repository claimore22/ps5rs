use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    PsmlModule,
    "libScePsml",
    [
        "scePsmlMfsrCreateContext1100",
        "scePsmlMfsrCreateSharedResources",
        "scePsmlMfsrGetContextBufferRequirement1100",
        "scePsmlMfsrGetDispatchMfsrPacket1100",
        "scePsmlMfsrGetDispatchMfsrPacketSizeInDwords",
        "scePsmlMfsrGetSharedResourcesInitRequirement",
        "scePsmlMfsrInit",
        "scePsmlMfsrReleaseContext",
        "scePsmlMfsrReleaseSharedResources"
    ]
);
