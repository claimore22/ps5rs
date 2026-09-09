use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    PlayerInvitationDialogModule,
    "libScePlayerInvitationDialog",
    [
        "scePlayerInvitationDialogClose",
        "scePlayerInvitationDialogInitialize",
        "scePlayerInvitationDialogOpen",
        "scePlayerInvitationDialogTerminate",
        "scePlayerInvitationDialogUpdateStatus"
    ]
);
