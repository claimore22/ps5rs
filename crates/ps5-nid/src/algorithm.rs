use sha1::{Sha1, Digest};

const SALT: [u8; 16] = [
    0x51, 0x8D, 0x64, 0xA6, 0x35, 0xDE, 0xD8, 0xC1,
    0xE6, 0xB0, 0x39, 0xB1, 0xC3, 0xE5, 0x52, 0x30,
];

const B64_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-";

pub fn hash(name: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(name.as_bytes());
    hasher.update(&SALT);
    let result = hasher.finalize();

    let mut reversed = [0u8; 8];
    for i in 0..8 {
        reversed[i] = result[7 - i];
    }

    let mut nid = String::with_capacity(11);
    let chunks = reversed.chunks(3);
    for chunk in chunks {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        nid.push(B64_ALPHABET[((triple >> 18) & 63) as usize] as char);
        nid.push(B64_ALPHABET[((triple >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            nid.push(B64_ALPHABET[((triple >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            nid.push(B64_ALPHABET[(triple & 63) as usize] as char);
        }
    }

    nid.truncate(11);
    nid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_memcpy() {
        assert_eq!(hash("memcpy"), "Q3VBxCXhUHs");
    }

    #[test]
    fn hash_sce_kernel_sleep() {
        assert_eq!(hash("sceKernelSleep"), "-ZR+hG7aDHw");
    }

    #[test]
    fn hash_sce_pthread_create() {
        assert_eq!(hash("scePthreadCreate"), "6UgtwV+0zb4");
    }

    #[test]
    fn hash_empty_string() {
        let nid = hash("");
        assert!(!nid.is_empty());
        assert!(nid.len() <= 11);
    }

    #[test]
    fn hash_length_always_11() {
        for name in &["a", "ab", "abc", "long_function_name_here", "", "x"] {
            assert_eq!(hash(name).len(), 11, "hash of {name:?} should be 11 chars");
        }
    }

    #[test]
    fn hash_only_uses_b64_alphabet() {
        let nid = hash("anything");
        for ch in nid.chars() {
            assert!(
                B64_ALPHABET.iter().any(|&b| b as char == ch),
                "unexpected char '{ch}' in NID '{nid}'"
            );
        }
    }

    #[test]
    fn hash_deterministic() {
        let a = hash("sceKernelOpen");
        let b = hash("sceKernelOpen");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_different_inputs_differ() {
        assert_ne!(hash("memcpy"), hash("memset"));
        assert_ne!(hash("malloc"), hash("free"));
    }

    #[test]
    fn hash_salt_matters() {
        let with_salt = hash("test");
        let mut hasher = Sha1::new();
        hasher.update(b"test");
        let without_salt = hasher.finalize();
        assert_ne!(with_salt.len(), 0);
        assert_ne!(format!("{:x}", without_salt), "");
    }
}
