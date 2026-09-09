use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    NpSessionSignalingModule,
    "libSceNpSessionSignaling",
    [
        "sceNpSessionSignalingActivateSession",
        "sceNpSessionSignalingCreateContext",
        "sceNpSessionSignalingCreateContext2",
        "sceNpSessionSignalingDeactivate",
        "sceNpSessionSignalingDestroyContext",
        "sceNpSessionSignalingGetConnectionFromPeerAddress2",
        "sceNpSessionSignalingGetConnectionInfo",
        "sceNpSessionSignalingGetConnectionStatus",
        "sceNpSessionSignalingGetGroupFromSessionId",
        "sceNpSessionSignalingGetLocalNetInfo",
        "sceNpSessionSignalingInitialize",
        "sceNpSessionSignalingTerminate"
    ]
);
