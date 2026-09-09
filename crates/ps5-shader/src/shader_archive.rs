use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShaderMapEntry {
    pub shader_indices_offset: u32,
    pub num_shaders: u32,
    pub first_preload_index: u32,
    pub num_preload_entries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShaderCodeEntry {
    pub offset: u64,
    pub size: u32,
    pub uncompressed_size: u32,
    pub frequency: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileCachePreloadEntry {
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderArchive {
    pub version: u32,
    pub shader_map_hashes: Vec<[u8; 20]>,
    pub shader_hashes: Vec<[u8; 20]>,
    pub shader_map_entries: Vec<ShaderMapEntry>,
    pub shader_entries: Vec<ShaderCodeEntry>,
    pub preload_entries: Vec<FileCachePreloadEntry>,
    pub shader_indices: Vec<u32>,
    pub code: Vec<u8>,
}

impl ShaderArchive {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 {
            return Err("too short for shader archive".to_string());
        }
        let mut off = 0usize;
        let version = read_u32(data, &mut off)?;
        if version != 2 {
            return Err(format!("unsupported shader archive version {version}"));
        }
        let n_map = read_u32(data, &mut off)? as usize;
        if data.len() < off + n_map * 20 {
            return Err("truncated shader_map_hashes".to_string());
        }
        let mut shader_map_hashes = Vec::with_capacity(n_map);
        for _ in 0..n_map {
            let mut h = [0u8; 20];
            h.copy_from_slice(&data[off..off + 20]);
            off += 20;
            shader_map_hashes.push(h);
        }
        let n_shader = read_u32(data, &mut off)? as usize;
        if data.len() < off + n_shader * 20 {
            return Err("truncated shader_hashes".to_string());
        }
        let mut shader_hashes = Vec::with_capacity(n_shader);
        for _ in 0..n_shader {
            let mut h = [0u8; 20];
            h.copy_from_slice(&data[off..off + 20]);
            off += 20;
            shader_hashes.push(h);
        }
        let n_map_entries = read_u32(data, &mut off)? as usize;
        if data.len() < off + n_map_entries * 16 {
            return Err("truncated shader_map_entries".to_string());
        }
        let mut shader_map_entries = Vec::with_capacity(n_map_entries);
        for _ in 0..n_map_entries {
            let a = read_u32(data, &mut off)?;
            let b = read_u32(data, &mut off)?;
            let c = read_u32(data, &mut off)?;
            let d = read_u32(data, &mut off)?;
            shader_map_entries.push(ShaderMapEntry {
                shader_indices_offset: a,
                num_shaders: b,
                first_preload_index: c,
                num_preload_entries: d,
            });
        }
        let n_entries = read_u32(data, &mut off)? as usize;
        if data.len() < off + n_entries * 17 {
            return Err("truncated shader_entries".to_string());
        }
        let mut shader_entries = Vec::with_capacity(n_entries);
        for _ in 0..n_entries {
            let offset = read_u64(data, &mut off)?;
            let size = read_u32(data, &mut off)?;
            let uncompressed_size = read_u32(data, &mut off)?;
            let frequency = read_u8(data, &mut off)?;
            shader_entries.push(ShaderCodeEntry {
                offset,
                size,
                uncompressed_size,
                frequency,
            });
        }
        let n_preload = read_u32(data, &mut off)? as usize;
        if data.len() < off + n_preload * 16 {
            return Err("truncated preload_entries".to_string());
        }
        let mut preload_entries = Vec::with_capacity(n_preload);
        for _ in 0..n_preload {
            let a = read_u64(data, &mut off)?;
            let b = read_u64(data, &mut off)?;
            preload_entries.push(FileCachePreloadEntry { offset: a, size: b });
        }
        let n_indices = read_u32(data, &mut off)? as usize;
        if data.len() < off + n_indices * 4 {
            return Err("truncated shader_indices".to_string());
        }
        let mut shader_indices = Vec::with_capacity(n_indices);
        for _ in 0..n_indices {
            shader_indices.push(read_u32(data, &mut off)?);
        }
        if off > data.len() {
            return Err("code offset beyond file".to_string());
        }
        let code = data[off..].to_vec();
        if let Some(last) = shader_entries.last()
            && (last.offset as usize + last.size as usize) > code.len()
        {
            return Err("last shader entry exceeds code buffer".to_string());
        }
        Ok(Self {
            version,
            shader_map_hashes,
            shader_hashes,
            shader_map_entries,
            shader_entries,
            preload_entries,
            shader_indices,
            code,
        })
    }

    pub fn get_shader_bytes(&self, index: usize) -> Option<&[u8]> {
        let entry = self.shader_entries.get(index)?;
        let start = entry.offset as usize;
        let end = start + entry.size as usize;
        if end <= self.code.len() {
            Some(&self.code[start..end])
        } else {
            None
        }
    }

    pub fn shader_count(&self) -> usize {
        self.shader_entries.len()
    }

    pub fn map_count(&self) -> usize {
        self.shader_map_entries.len()
    }

    pub fn from_roms(roms_path: &str) -> Vec<Self> {
        let mut out = Vec::new();
        let path = std::path::Path::new(roms_path);
        if !path.exists() {
            return out;
        }
        for file in walk_ushader_files(path) {
            if let Ok(data) = std::fs::read(&file)
                && let Ok(archive) = Self::parse(&data)
            {
                out.push(archive);
            }
        }
        out
    }
}

fn read_u32(data: &[u8], off: &mut usize) -> Result<u32, String> {
    if *off + 4 > data.len() {
        return Err("unexpected eof reading u32".to_string());
    }
    let v = u32::from_le_bytes(data[*off..*off + 4].try_into().unwrap());
    *off += 4;
    Ok(v)
}

fn read_u64(data: &[u8], off: &mut usize) -> Result<u64, String> {
    if *off + 8 > data.len() {
        return Err("unexpected eof reading u64".to_string());
    }
    let v = u64::from_le_bytes(data[*off..*off + 8].try_into().unwrap());
    *off += 8;
    Ok(v)
}

fn read_u8(data: &[u8], off: &mut usize) -> Result<u8, String> {
    if *off + 1 > data.len() {
        return Err("unexpected eof reading u8".to_string());
    }
    let v = data[*off];
    *off += 1;
    Ok(v)
}

fn walk_ushader_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.is_file() && p.extension().is_some_and(|s| s == "ushaderbytecode") {
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

    fn build_minimal_archive() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&[0xAAu8; 20]);
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&[0xBBu8; 20]);
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&0u64.to_le_bytes());
        out.extend_from_slice(&4u32.to_le_bytes());
        out.extend_from_slice(&4u32.to_le_bytes());
        out.push(5u8);
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&0u64.to_le_bytes());
        out.extend_from_slice(&4u64.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]);
        out
    }

    #[test]
    fn parse_minimal() {
        let data = build_minimal_archive();
        let a = ShaderArchive::parse(&data).unwrap();
        assert_eq!(a.version, 2);
        assert_eq!(a.shader_map_hashes.len(), 1);
        assert_eq!(a.shader_hashes.len(), 1);
        assert_eq!(a.shader_entries.len(), 1);
        assert_eq!(a.code, vec![0x11, 0x22, 0x33, 0x44]);
        assert_eq!(a.get_shader_bytes(0).unwrap(), &[0x11, 0x22, 0x33, 0x44]);
    }

    #[test]
    fn parse_empty_fails() {
        assert!(ShaderArchive::parse(&[]).is_err());
    }

    #[test]
    fn wrong_version_fails() {
        let mut d = vec![0u8; 4];
        d[0] = 1;
        assert!(ShaderArchive::parse(&d).is_err());
    }

    #[test]
    fn truncated_fails() {
        let mut d = build_minimal_archive();
        d.truncate(10);
        assert!(ShaderArchive::parse(&d).is_err());
    }

    #[test]
    fn get_shader_bytes_out_of_range() {
        let data = build_minimal_archive();
        let a = ShaderArchive::parse(&data).unwrap();
        assert!(a.get_shader_bytes(99).is_none());
    }

    #[test]
    fn from_roms_missing_is_empty() {
        let v = ShaderArchive::from_roms("/nonexistent/path/xyz");
        assert!(v.is_empty());
    }

    #[test]
    fn map_and_shader_count() {
        let a = ShaderArchive::parse(&build_minimal_archive()).unwrap();
        assert_eq!(a.map_count(), 1);
        assert_eq!(a.shader_count(), 1);
    }
}
