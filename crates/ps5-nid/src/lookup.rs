const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+-";

pub fn lib_id_from_nid(nid: &str) -> Option<u16> {
    let hash_end = nid.find('#')?;
    let lib_str = &nid[hash_end + 1..];
    let mut val: u16 = 0;
    for ch in lib_str.bytes() {
        let pos = B64.iter().position(|&b| b == ch)?;
        val = val.checked_mul(64)?.checked_add(pos as u16)?;
    }
    Some(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_char_a() {
        assert_eq!(lib_id_from_nid("abc123#A"), Some(0));
    }

    #[test]
    fn single_char_b() {
        assert_eq!(lib_id_from_nid("abc123#B"), Some(1));
    }

    #[test]
    fn two_chars_aa() {
        assert_eq!(lib_id_from_nid("abc123#AA"), Some(0));
    }

    #[test]
    fn two_chars_ab() {
        assert_eq!(lib_id_from_nid("abc123#AB"), Some(1));
    }

    #[test]
    fn no_hash_returns_none() {
        assert_eq!(lib_id_from_nid("abc123"), None);
    }

    #[test]
    fn invalid_char_returns_none() {
        assert_eq!(lib_id_from_nid("abc#@invalid"), None);
    }

    #[test]
    fn empty_lib_id() {
        assert_eq!(lib_id_from_nid("nid#"), Some(0));
    }

    #[test]
    fn overflow_returns_none() {
        let mut long_lib = String::from("nid#");
        for _ in 0..5 {
            long_lib.push('z');
        }
        assert_eq!(lib_id_from_nid(&long_lib), None);
    }
}
