#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = ps5_shader::ShaderBinary::parse(data);
    if let Ok(bin) = ps5_shader::ShaderBinary::parse(data) {
        let _ = ps5_shader::reflection::Reflection::from_binary(data);
        let _ = ps5_shader::disasm::disassemble(data);
        let _ = bin.stage;
    }
});
