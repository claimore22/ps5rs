use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    CdlgPlayerReviewModule,
    "libSceCdlgPlayerReview",
    [
        "scePlayerReviewDialogClose",
        "scePlayerReviewDialogGetResult",
        "scePlayerReviewDialogGetStatus",
        "scePlayerReviewDialogInitialize",
        "scePlayerReviewDialogOpen",
        "scePlayerReviewDialogTerminate",
        "scePlayerReviewDialogUpdateStatus"
    ]
);