use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    NpCommerceModule,
    "libSceNpCommerce",
    [
        "sceNpCommerceDialogGetStatus",
        "sceNpCommerceDialogInitialize",
        "sceNpCommerceDialogOpen",
        "sceNpCommerceDialogTerminate",
        "sceNpCommerceDialogUpdateStatus",
        "sceNpCommerceHidePsStoreIcon",
        "sceNpCommerceShowPsStoreIcon"
    ]
);
