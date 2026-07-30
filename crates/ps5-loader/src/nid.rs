use sha1::{Digest, Sha1};

/// Sony's base64 alphabet used for NID strings.
const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-";

/// Salt bytes appended to symbol names before SHA1 hashing.
const SALT: [u8; 16] = [
    0x51, 0x8D, 0x64, 0xA6, 0x35, 0xDE, 0xD8, 0xC1, 0xE6, 0xB0, 0x39, 0xB1, 0xC3, 0xE5, 0x52, 0x30,
];

/// Decode an 11-character Sony-style base64 NID to a u64.
pub fn nid_to_u64(nid: &str) -> Option<u64> {
    if nid.len() != 11 {
        return None;
    }
    let mut val: u64 = 0;
    for &c in nid.as_bytes() {
        let idx = B64.iter().position(|&b| b == c)?;
        val = val.wrapping_mul(64).wrapping_add(idx as u64);
    }
    Some(val)
}

/// Compute the u64 NID for a human-readable SCE symbol name.
///
/// Uses the same SHA1+SALT algorithm as `ps5-nid::algorithm::hash()`.
pub fn compute_nid(name: &str) -> Option<u64> {
    let mut hasher = Sha1::new();
    hasher.update(name.as_bytes());
    hasher.update(SALT);
    let result = hasher.finalize();

    // Reverse first 8 bytes: result[7], result[6], ..., result[0]
    let mut buf = [0u8; 8];
    for i in 0..8 {
        buf[i] = result[7 - i];
    }

    // Base64-encode the reversed bytes into an 11-char NID string.
    let mut nid = String::with_capacity(11);
    for chunk in buf.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        nid.push(B64[((triple >> 18) & 63) as usize] as char);
        nid.push(B64[((triple >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            nid.push(B64[((triple >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            nid.push(B64[(triple & 63) as usize] as char);
        }
    }
    nid.truncate(11);

    nid_to_u64(&nid)
}

/// Resolves a symbol name to a numeric NID.
pub trait NidResolver {
    fn resolve(&self, name: &str) -> Option<u64>;
}

/// Default resolver that handles both `#NID` format and readable SCE names.
///
/// Resolution order:
/// 1. If the name contains `#`, parse the first part as an 11-char base64 NID.
/// 2. Otherwise, compute the SHA1+SALT hash of the full name.
#[derive(Debug, Clone, Copy)]
pub struct SymbolNidResolver;

impl NidResolver for SymbolNidResolver {
    fn resolve(&self, name: &str) -> Option<u64> {
        let candidate = name.split('#').next().unwrap_or(name);
        if let Some(nid) = nid_to_u64(candidate) {
            return Some(nid);
        }
        compute_nid(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nid_to_u64_empty() {
        assert_eq!(nid_to_u64(""), None);
    }

    #[test]
    fn nid_to_u64_short() {
        assert_eq!(nid_to_u64("AAAA"), None);
    }

    #[test]
    fn nid_to_u64_all_zeros() {
        assert_eq!(nid_to_u64("AAAAAAAAAAA"), Some(0));
    }

    #[test]
    fn nid_to_u64_invalid_char() {
        assert_eq!(nid_to_u64("AAAAAAAAAA!"), None);
    }

    #[test]
    fn nid_to_u64_known_input() {
        let nid = nid_to_u64("J6h9iA2kL7M").unwrap();
        assert!(nid > 0);
    }

    #[test]
    fn compute_nid_memcpy_matches_ps5_nid() {
        let nid = compute_nid("memcpy").unwrap();
        let expected = nid_to_u64("Q3VBxCXhUHs").unwrap();
        assert_eq!(nid, expected);
    }

    #[test]
    fn compute_nid_sce_kernel_sleep() {
        let nid = compute_nid("sceKernelSleep").unwrap();
        let expected = nid_to_u64("-ZR+hG7aDHw").unwrap();
        assert_eq!(nid, expected);
    }

    #[test]
    fn compute_nid_sce_pthread_create() {
        let nid = compute_nid("scePthreadCreate").unwrap();
        let expected = nid_to_u64("6UgtwV+0zb4").unwrap();
        assert_eq!(nid, expected);
    }

    #[test]
    fn compute_nid_deterministic() {
        let a = compute_nid("sceKernelOpen").unwrap();
        let b = compute_nid("sceKernelOpen").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn compute_nid_different_inputs_differ() {
        assert_ne!(compute_nid("memcpy"), compute_nid("memset"));
    }

    #[test]
    fn symbol_resolver_extracts_nid_from_hash_format() {
        let resolver = SymbolNidResolver;
        let nid = resolver.resolve("J6h9iA2kL7M#libkernel").unwrap();
        let expected = nid_to_u64("J6h9iA2kL7M").unwrap();
        assert_eq!(nid, expected);
    }

    #[test]
    fn symbol_resolver_computes_nid_for_readable_name() {
        let resolver = SymbolNidResolver;
        let nid = resolver.resolve("sceKernelSleep").unwrap();
        let expected = nid_to_u64("-ZR+hG7aDHw").unwrap();
        assert_eq!(nid, expected);
    }

    #[test]
    fn symbol_resolver_handles_name_without_hash_mark() {
        let resolver = SymbolNidResolver;
        let nid = resolver.resolve("memcpy").unwrap();
        assert!(nid > 0);
    }
}
