use crate::hle::stub::define_hle_stub_module;

define_hle_stub_module!(
    Videodec2Module,
    "libSceVideodec2",
    [
        "sceVideodec2AllocateComputeQueue",
        "sceVideodec2CreateDecoder",
        "sceVideodec2Decode",
        "sceVideodec2DeleteDecoder",
        "sceVideodec2Flush",
        "sceVideodec2QueryComputeMemoryInfo",
        "sceVideodec2QueryDecoderMemoryInfo",
        "sceVideodec2ReleaseComputeQueue",
        "sceVideodec2Reset"
    ]
);