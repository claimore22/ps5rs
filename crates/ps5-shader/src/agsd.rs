use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agsd {
    pub version: u32,
    pub data: Vec<u8>,
    pub debug_names: Vec<String>,
}

impl Agsd {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 8 {
            return Err("too short for agsd".to_string());
        }
        let version = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let name_count = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        let mut names = Vec::new();
        let mut offset = 8;
        for _ in 0..name_count.min(32) {
            if offset + 4 > data.len() {
                break;
            }
            let len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;
            if offset + len > data.len() {
                break;
            }
            if let Ok(s) = std::str::from_utf8(&data[offset..offset + len]) {
                names.push(s.to_string());
            }
            offset += len;
        }
        Ok(Self {
            version,
            data: data.to_vec(),
            debug_names: names,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_too_short_fails() {
        assert!(Agsd::parse(&[0u8; 4]).is_err());
    }

    #[test]
    fn parse_minimal() {
        let mut data = Vec::new();
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        let a = Agsd::parse(&data).unwrap();
        assert_eq!(a.version, 1);
    }
}
