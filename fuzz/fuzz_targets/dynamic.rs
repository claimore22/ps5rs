#![no_main]
use libfuzzer_sys::fuzz_target;

 fuzz_target!(|data: &[u8]| {
    if data.len() > 64 {
        let _ = ps5_elf::ElfHeader::parse(data, 0);
    }
});
