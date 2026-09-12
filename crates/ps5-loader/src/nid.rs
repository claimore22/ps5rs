/// Sony's base64 alphabet used for NID strings.
const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-";

/// Decode an 11-character Sony-style base64 NID to a u64.
pub fn nid_to_u64(nid: &str) -> Option<u64> {
    ps5_nid::nid_to_u64(nid)
}

/// Compute the u64 NID for a human-readable SCE symbol name.
///
/// Uses the canonical SHA1+SALT implementation from `ps5-nid`.
pub fn compute_nid(name: &str) -> Option<u64> {
    ps5_nid::nid_to_u64(&ps5_nid::hash(name))
}

/// Decode the base64 library id after `#` in a `nid#lib` symbol to a u16.
///
/// Mirrors `ps5_nid::lib_id_from_nid`; kept local so the loader stays
/// self-contained (it already vendors the NID hash algorithm above).
/// The id is the segment right after the first `#`, since masked binaries
/// may append a third segment (`nid#lib#extra`).
pub fn lib_id_from_nid(nid: &str) -> Option<u16> {
    let lib_str = nid.split('#').nth(1)?;
    let mut val: u16 = 0;
    for ch in lib_str.bytes() {
        let pos = B64.iter().position(|&b| b == ch)?;
        val = val.checked_mul(64)?.checked_add(pos as u16)?;
    }
    Some(val)
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
    fn compute_nid_cpp_operator_new_matches_catalog() {
        let nid = compute_nid("_Znwm").unwrap();
        assert_eq!(nid, 0x7c99_e9b9_5541_6ca9);
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
    fn lib_id_single_char() {
        assert_eq!(lib_id_from_nid("J6h9iA2kL7M#B"), Some(1));
    }

    #[test]
    fn lib_id_no_hash_returns_none() {
        assert_eq!(lib_id_from_nid("memcpy"), None);
    }

    #[test]
    fn lib_id_triple_segment_uses_middle() {
        assert_eq!(lib_id_from_nid("X#A#B"), Some(0));
        assert_eq!(lib_id_from_nid("MfDb+4Nln64#D#E"), Some(3));
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
