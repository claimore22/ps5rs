use crate::scanner::{sanitize_filename, utc_now_iso8601};
use ps5_self::extract::{extract_elf, ExtractResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionManifest {
    pub tool: String,
    pub generated_at: String,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub entries: Vec<ExtractionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionEntry {
    pub game: String,
    pub elf: String,
    pub source_sha256: String,
    pub elf_sha256: String,
    pub was_self: bool,
    pub source_size: u64,
    pub elf_size: u64,
    pub encrypted_segments: usize,
    pub compressed_segments: usize,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct BatchExtractResult {
    pub manifest: ExtractionManifest,
    pub output_dir: PathBuf,
    pub failures: Vec<(String, String)>,
}

#[derive(Default)]
pub struct BatchExtractOptions {
    pub include_modules: bool,
}

pub fn batch_extract(
    roms_dir: &Path,
    output_dir: &Path,
    _options: &BatchExtractOptions,
) -> Result<BatchExtractResult, std::io::Error> {
    let extracted_dir = output_dir.join("analysis").join("extracted");
    std::fs::create_dir_all(&extracted_dir)?;

    let mut entries = Vec::new();
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    let mut game_dirs = find_game_dirs(roms_dir);
    game_dirs.sort();

    for game_dir in &game_dirs {
        let game_name = game_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let safe_name = sanitize_filename(game_name);

        if seen_names.contains(&safe_name) {
            continue;
        }

        let binaries = find_eboot(game_dir);
        if binaries.is_empty() {
            continue;
        }

        let bin_path = &binaries[0];
        seen_names.insert(safe_name.clone());

        let elf_name = format!("{safe_name}.elf");
        let elf_path = extracted_dir.join(&elf_name);

        match extract_single(bin_path, &elf_path) {
            Ok(entry) => {
                succeeded += 1;
                entries.push(entry);
                eprintln!("  [OK]   {safe_name}.elf");
            }
            Err(err) => {
                failed += 1;
                let source_sha256 = std::fs::read(bin_path)
                    .map(|d| ps5_format::sha256_hex(&d))
                    .unwrap_or_default();
                entries.push(ExtractionEntry {
                    game: game_name.to_string(),
                    elf: elf_name,
                    source_sha256,
                    elf_sha256: String::new(),
                    was_self: false,
                    source_size: 0,
                    elf_size: 0,
                    encrypted_segments: 0,
                    compressed_segments: 0,
                    error: Some(err),
                });
                eprintln!("  [FAIL] {safe_name}: extraction failed");
            }
        }
    }

    let manifest = ExtractionManifest {
        tool: "ps5rs".to_string(),
        generated_at: utc_now_iso8601(),
        total: entries.len(),
        succeeded,
        failed,
        entries: entries.clone(),
    };

    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(extracted_dir.join("manifest.json"), format!("{manifest_json}\n"))?;

    let failures: Vec<(String, String)> = entries
        .iter()
        .filter_map(|e| {
            e.error
                .as_ref()
                .map(|err| (e.game.clone(), err.clone()))
        })
        .collect();

    Ok(BatchExtractResult {
        manifest,
        output_dir: extracted_dir,
        failures,
    })
}

fn extract_single(bin_path: &Path, elf_path: &Path) -> Result<ExtractionEntry, String> {
    let data = std::fs::read(bin_path).map_err(|e| format!("read error: {e}"))?;
    let source_sha256 = ps5_format::sha256_hex(&data);
    let source_size = data.len() as u64;

    let ExtractResult {
        elf,
        was_self,
        encrypted_segments,
        compressed_segments,
    } = extract_elf(&data).map_err(|e| format!("extract error: {e}"))?;

    let elf_sha256 = ps5_format::sha256_hex(&elf);
    let elf_size = elf.len() as u64;

    std::fs::write(elf_path, &elf).map_err(|e| format!("write error: {e}"))?;

    let game = bin_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let elf_name = elf_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.elf")
        .to_string();

    Ok(ExtractionEntry {
        game,
        elf: elf_name,
        source_sha256,
        elf_sha256,
        was_self,
        source_size,
        elf_size,
        encrypted_segments,
        compressed_segments,
        error: None,
    })
}

fn find_game_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            }
        }
    }
    dirs
}

fn find_eboot(game_dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    walk_for_eboot(game_dir, &mut result, 0);
    result
}

fn walk_for_eboot(dir: &Path, result: &mut Vec<PathBuf>, depth: usize) {
    if depth > 4 {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_for_eboot(&path, result, depth + 1);
                } else if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.eq_ignore_ascii_case("eboot.bin")
                {
                    result.push(path);
                }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraction_manifest_serde_roundtrip() {
        let manifest = ExtractionManifest {
            tool: "ps5rs-test".to_string(),
            generated_at: "2026-07-25T00:00:00Z".to_string(),
            total: 2,
            succeeded: 1,
            failed: 1,
            entries: vec![
                ExtractionEntry {
                    game: "GameA".to_string(),
                    elf: "GameA.elf".to_string(),
                    source_sha256: "abc123".to_string(),
                    elf_sha256: "def456".to_string(),
                    was_self: true,
                    source_size: 1024,
                    elf_size: 512,
                    encrypted_segments: 0,
                    compressed_segments: 0,
                    error: None,
                },
                ExtractionEntry {
                    game: "GameB".to_string(),
                    elf: "GameB.elf".to_string(),
                    source_sha256: "789abc".to_string(),
                    elf_sha256: String::new(),
                    was_self: true,
                    source_size: 2048,
                    elf_size: 0,
                    encrypted_segments: 1,
                    compressed_segments: 0,
                    error: Some("encrypted segments".to_string()),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let back: ExtractionManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total, 2);
        assert_eq!(back.succeeded, 1);
        assert_eq!(back.failed, 1);
        assert!(back.entries[0].error.is_none());
        assert_eq!(back.entries[1].error.as_deref(), Some("encrypted segments"));
    }

    #[test]
    fn extraction_entry_has_all_fields() {
        let entry = ExtractionEntry {
            game: "Test".to_string(),
            elf: "Test.elf".to_string(),
            source_sha256: "aaa".to_string(),
            elf_sha256: "bbb".to_string(),
            was_self: false,
            source_size: 100,
            elf_size: 100,
            encrypted_segments: 0,
            compressed_segments: 0,
            error: None,
        };
        assert_eq!(entry.game, "Test");
        assert!(!entry.was_self);
        assert_eq!(entry.source_size, 100);
    }

    #[test]
    fn batch_extract_empty_dir() {
        let tmp = std::env::temp_dir().join(format!(
            "ps5rs_batch_extract_empty_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let output = tmp.join("extracted");
        let result = batch_extract(&tmp, &output, &BatchExtractOptions::default()).unwrap();
        assert_eq!(result.manifest.total, 0);
        assert_eq!(result.manifest.succeeded, 0);
        assert!(result.failures.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn batch_extract_real_binary() {
        let tmp = std::env::temp_dir().join(format!(
            "ps5rs_batch_extract_real_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);

        let game_dir = tmp.join("TestGame");
        std::fs::create_dir_all(&game_dir).unwrap();

        let test_elf = build_minimal_elf();
        std::fs::write(game_dir.join("eboot.bin"), &test_elf).unwrap();

        let output = tmp.join("extracted");
        let result = batch_extract(&tmp, &output, &BatchExtractOptions::default()).unwrap();
        assert_eq!(result.manifest.total, 1);
        assert_eq!(result.manifest.succeeded, 1);
        assert!(result.failures.is_empty());
        assert!(output.join("analysis").join("extracted").join("TestGame.elf").exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    fn build_minimal_elf() -> Vec<u8> {
        use ps5_format::elf_constants::*;
        let entry = 0x1000u64;
        let load_vaddr = 0x1000u64;
        let load_offset = 0x1000u64;
        let load_data = vec![0xCCu8; 64];
        let phdr_count = 1u16;
        let e_phoff: u64 = 64;
        let mut file = vec![0u8; load_offset as usize + load_data.len()];

        let write_u16 = |data: &mut [u8], off: usize, v: u16| {
            data[off..off + 2].copy_from_slice(&v.to_le_bytes());
        };
        let write_u32 = |data: &mut [u8], off: usize, v: u32| {
            data[off..off + 4].copy_from_slice(&v.to_le_bytes());
        };
        let write_u64 = |data: &mut [u8], off: usize, v: u64| {
            data[off..off + 8].copy_from_slice(&v.to_le_bytes());
        };

        file[0..4].copy_from_slice(&ELF_MAGIC);
        file[EI_CLASS] = ELFCLASS64;
        file[EI_DATA] = ELFDATA2LSB;
        file[EI_VERSION] = 1;
        write_u16(&mut file, 16, ET_EXEC);
        write_u16(&mut file, 18, EM_X86_64);
        write_u32(&mut file, 20, 1);
        write_u64(&mut file, 24, entry);
        write_u64(&mut file, 32, e_phoff);
        write_u16(&mut file, 52, 64);
        write_u16(&mut file, 54, 56);
        write_u16(&mut file, 56, phdr_count);

        let phdr_offset = e_phoff as usize;
        write_u32(&mut file, phdr_offset, PT_LOAD);
        write_u32(&mut file, phdr_offset + 4, PF_R | PF_X);
        write_u64(&mut file, phdr_offset + 8, load_offset);
        write_u64(&mut file, phdr_offset + 16, load_vaddr);
        write_u64(&mut file, phdr_offset + 24, 0);
        write_u64(&mut file, phdr_offset + 32, load_data.len() as u64);
        write_u64(&mut file, phdr_offset + 40, load_data.len() as u64);
        write_u64(&mut file, phdr_offset + 48, 0x1000);

        file[load_offset as usize..].copy_from_slice(&load_data);
        file
    }
}
