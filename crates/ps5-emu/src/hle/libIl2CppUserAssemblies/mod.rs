use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    Il2cppModule,
    "Il2cppUserAssemblies",
    [
        "il2cpp_api_lookup_symbol",
        "il2cpp_api_register_symbols",
        "SetDataFolder",
        "setenv",
        "unity_mono_set_user_malloc_mutex"
    ]
);
