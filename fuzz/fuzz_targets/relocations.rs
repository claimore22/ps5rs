#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(img) = ps5_elf::ElfImage::parse(data, None) {
        let _ = img.relocations.len();
        let _ = ps5_elf::ElfHeader::parse(data, 0);
    }
});
