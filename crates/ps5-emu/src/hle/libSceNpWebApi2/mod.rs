use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    NpWebApi2Module,
    "libSceNpWebApi2",
    [
        "sceNpWebApi2AddHttpRequestHeader",
        "sceNpWebApi2DeleteRequest",
        "sceNpWebApi2Initialize",
        "sceNpWebApi2PushEventRegisterCallback",
        "sceNpWebApi2PushEventUnregisterPushContextCallback",
        "sceNpWebApi2ReadData"
    ]
);
