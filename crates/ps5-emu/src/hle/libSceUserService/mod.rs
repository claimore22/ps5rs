use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    UserServiceModule,
    "libSceUserService",
    [
        "sceUserServiceGetInitialUser",
        "sceUserServiceGetLoginUserIdList",
        "sceUserServiceInitialize"
    ]
);
