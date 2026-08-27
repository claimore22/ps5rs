pub fn fuzz_elf(data: &[u8]) {
    let _ = ps5_elf::ElfImage::parse(data, None);
    if let Ok(img) = ps5_elf::ElfImage::parse(data, None) {
        let _ = ps5_nid::Catalog::new();
        let _ = img.symbols.len();
    }
}

pub fn fuzz_self(data: &[u8]) {
    let _ = ps5_self::SelfImage::parse(data);
}

pub fn fuzz_nid(data: &[u8]) {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = ps5_nid::hash(s);
        let mut cat = ps5_nid::Catalog::new();
        cat.add(s);
    }
}

pub fn fuzz_dynamic(data: &[u8]) {
    if data.len() < 16 {
        return;
    }
    let _ = ps5_elf::ElfImage::parse(data, None);
}

pub fn fuzz_relocations(data: &[u8]) {
    let _ = ps5_elf::ElfImage::parse(data, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzz_elf_empty() {
        fuzz_elf(b"");
        fuzz_elf(&[0x7f, b'E', b'L', b'F']);
    }

    #[test]
    fn fuzz_self_empty() {
        fuzz_self(b"");
    }

    #[test]
    fn fuzz_nid_basic() {
        fuzz_nid(b"test");
        fuzz_nid(b"memcpy");
    }
}
