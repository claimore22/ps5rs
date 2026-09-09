use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalShaderCache {
    pub num_maps: u32,
    pub raw_header: Vec<u8>,
    pub data: Vec<u8>,
}

impl GlobalShaderCache {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 {
            return Err("too short for global shader cache".to_string());
        }
        let num_maps = u32::from_le_bytes(data[0..4].try_into().unwrap());
        if num_maps == 0 || num_maps > 100_000 {
            return Err(format!("implausible num_maps {num_maps}"));
        }
        if data.len() < 8 {
            return Err("truncated header".to_string());
        }
        let header_end = find_string_table_offset(data).unwrap_or(usize::min(4096, data.len()));
        let raw_header = data[..header_end].to_vec();
        Ok(Self {
            num_maps,
            raw_header,
            data: data.to_vec(),
        })
    }

    pub fn from_roms(roms_path: &str) -> Vec<Self> {
        let mut out = Vec::new();
        let path = std::path::Path::new(roms_path);
        if !path.exists() {
            return out;
        }
        for file in walk_cache_files(path) {
            if let Ok(data) = std::fs::read(&file)
                && let Ok(cache) = Self::parse(&data)
            {
                out.push(cache);
            }
        }
        out
    }
}

fn find_string_table_offset(data: &[u8]) -> Option<usize> {
    let needle = b"FGlobalShaderMapContent";
    data.windows(needle.len())
        .position(|w| w == needle)
        .map(|pos| pos.saturating_sub(16))
}

fn walk_cache_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.is_file()
                    && let Some(name) = p.file_name().and_then(|n| n.to_str())
                    && name.starts_with("GlobalShaderCache")
                    && name.ends_with(".bin")
                {
                    files.push(p);
                }
            }
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_minimal_cache(num_maps: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&num_maps.to_le_bytes());
        data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
        data.extend_from_slice(b"FGlobalShaderMapContent");
        data.extend_from_slice(&[0u8; 32]);
        data
    }

    #[test]
    fn parse_minimal() {
        let d = build_minimal_cache(167);
        let c = GlobalShaderCache::parse(&d).unwrap();
        assert_eq!(c.num_maps, 167);
    }

    #[test]
    fn parse_jusant_count() {
        let d = build_minimal_cache(318);
        let c = GlobalShaderCache::parse(&d).unwrap();
        assert_eq!(c.num_maps, 318);
    }

    #[test]
    fn parse_empty_fails() {
        assert!(GlobalShaderCache::parse(&[]).is_err());
    }

    #[test]
    fn zero_maps_fails() {
        let d = build_minimal_cache(0);
        assert!(GlobalShaderCache::parse(&d).is_err());
    }

    #[test]
    fn from_roms_missing_is_empty() {
        let v = GlobalShaderCache::from_roms("/nonexistent/path/xyz");
        assert!(v.is_empty());
    }
}
