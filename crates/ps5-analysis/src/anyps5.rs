use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::scanner::{sanitize_filename, utc_now_iso8601};

pub const MAX_WALK_DEPTH: usize = 32;
const BACKUP_SUFFIXES: &[&str] = &[".esbak", ".bak", ".orig"];
const AUTO_MODULE_DIRS: &[&str] = &["sce_module", "sce_modules", "prx"];
const MANIFEST_NAME: &str = "anyps5-manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertedModule {
    pub relative_path: String,
    pub source_sha256: String,
    pub elf_sha256: String,
    pub was_self: bool,
    pub source_size: u64,
    pub elf_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutFailure {
    pub relative_path: String,
    pub phase: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameLayoutManifest {
    pub tool: String,
    pub generated_at: String,
    pub game: String,
    pub eboot: Option<String>,
    pub converted: Vec<ConvertedModule>,
    pub copied_files: usize,
    pub copied_bytes: u64,
    pub module_dirs: Vec<String>,
    pub failures: Vec<LayoutFailure>,
    pub complete: bool,
}

#[derive(Debug)]
pub struct GameLayoutReport {
    pub manifest: GameLayoutManifest,
    pub out_dir: PathBuf,
}

#[derive(Debug)]
pub struct BatchGameLayout {
    pub name: String,
    pub out_dir: PathBuf,
    pub manifest: GameLayoutManifest,
}

#[derive(Debug, Default)]
pub struct BatchLayoutReport {
    pub games: Vec<BatchGameLayout>,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub failures: Vec<(String, String)>,
}

struct TreeEntry {
    path: PathBuf,
    rel: String,
    is_dir: bool,
}

fn rel_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .map(|r| {
            r.components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn is_backup_name(lower: &str) -> bool {
    BACKUP_SUFFIXES.iter().any(|s| lower.ends_with(s))
}

fn is_module_name(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    if is_backup_name(&lower) {
        return false;
    }
    lower.ends_with(".prx") || lower.ends_with(".sprx")
}

fn is_eboot_name(file_name: &str) -> bool {
    file_name == "eboot.bin"
}

fn under_excluded_dir(rel: &str) -> bool {
    let top = rel.split('/').next().unwrap_or("");
    top == "fakelib" || top == "sce_sys"
}

fn collect_tree(root: &Path, out: &mut Vec<TreeEntry>, notes: &mut Vec<String>) {
    collect_dir(root, root, out, notes, 0);
}

fn collect_dir(
    root: &Path,
    dir: &Path,
    out: &mut Vec<TreeEntry>,
    notes: &mut Vec<String>,
    depth: usize,
) {
    if depth > MAX_WALK_DEPTH {
        notes.push(format!("depth limit reached at {}", rel_string(root, dir)));
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            notes.push(format!("cannot list {}: {e}", rel_string(root, dir)));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let rel = rel_string(root, &path);
        let kind = match entry.file_type() {
            Ok(k) => k,
            Err(e) => {
                notes.push(format!("cannot stat {rel}: {e}"));
                continue;
            }
        };
        if kind.is_symlink() {
            notes.push(format!("skipped symlink: {rel}"));
            continue;
        }
        if kind.is_dir() {
            out.push(TreeEntry {
                path: path.clone(),
                rel,
                is_dir: true,
            });
            collect_dir(root, &path, out, notes, depth + 1);
        } else if kind.is_file() {
            out.push(TreeEntry {
                path,
                rel,
                is_dir: false,
            });
        } else {
            notes.push(format!("skipped special file: {rel}"));
        }
    }
}

fn atomic_write(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp_name = format!(
        "{}.partial-{}",
        dest.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string()),
        std::process::id()
    );
    let tmp = dest.with_file_name(tmp_name);
    if let Err(e) = std::fs::write(&tmp, bytes) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("write error: {e}"));
    }
    std::fs::rename(&tmp, dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("rename error: {e}")
    })?;
    Ok(())
}

fn convert_one(data: &[u8]) -> Result<(Vec<u8>, bool), String> {
    let result = ps5_self::extract::extract_elf(data).map_err(|e| format!("extract error: {e}"))?;
    Ok((result.elf, result.was_self))
}

fn game_name_of(root: &Path) -> String {
    root.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn layout_game(game_root: &Path, out_dir: &Path) -> Result<GameLayoutReport, String> {
    if !game_root.is_dir() {
        return Err(format!("not a game directory: {}", game_root.display()));
    }
    if out_dir == game_root {
        return Err("output directory must not be the game directory".to_string());
    }
    if out_dir.starts_with(game_root) {
        return Err("output directory must not sit inside the game directory".to_string());
    }
    for tree in ["source", "app0"] {
        if game_root.starts_with(out_dir.join(tree)) {
            return Err(format!(
                "game directory sits inside the output {tree}/ tree that this run writes"
            ));
        }
    }

    let mut entries = Vec::new();
    let mut notes = Vec::new();
    collect_tree(game_root, &mut entries, &mut notes);
    entries.sort_by(|a, b| a.rel.cmp(&b.rel));

    let mut by_depth: Vec<&TreeEntry> = entries.iter().filter(|e| !e.is_dir).collect();
    by_depth.sort_by_key(|e| (e.rel.matches('/').count(), &e.rel));
    let eboot = by_depth.iter().find(|e| {
        e.path
            .file_name()
            .is_some_and(|n| is_eboot_name(&n.to_string_lossy()))
    });
    let Some(eboot) = eboot else {
        return Err(format!("no eboot.bin found under {}", game_root.display()));
    };
    let eboot_rel = eboot.rel.clone();

    let source_dir = out_dir.join("source");
    let app0_dir = out_dir.join("app0");
    std::fs::create_dir_all(&source_dir)
        .map_err(|e| format!("cannot create {}: {e}", source_dir.display()))?;
    std::fs::create_dir_all(&app0_dir)
        .map_err(|e| format!("cannot create {}: {e}", app0_dir.display()))?;

    let mut manifest = GameLayoutManifest {
        tool: "ps5rs".to_string(),
        generated_at: utc_now_iso8601(),
        game: game_name_of(game_root),
        eboot: Some(eboot_rel.clone()),
        converted: Vec::new(),
        copied_files: 0,
        copied_bytes: 0,
        module_dirs: Vec::new(),
        failures: Vec::new(),
        complete: false,
    };

    for entry in entries.iter().filter(|e| !e.is_dir) {
        let dest = app0_dir.join(entry.rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = dest.parent()
            && std::fs::create_dir_all(parent).is_err()
        {
            manifest.failures.push(LayoutFailure {
                relative_path: entry.rel.clone(),
                phase: "copy".to_string(),
                reason: format!("cannot create {}", parent.display()),
            });
            continue;
        }
        match std::fs::copy(&entry.path, &dest) {
            Ok(size) => {
                manifest.copied_files += 1;
                manifest.copied_bytes += size;
            }
            Err(e) => manifest.failures.push(LayoutFailure {
                relative_path: entry.rel.clone(),
                phase: "copy".to_string(),
                reason: format!("copy error: {e}"),
            }),
        }
    }
    for entry in entries.iter().filter(|e| e.is_dir) {
        let dest = app0_dir.join(entry.rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if std::fs::create_dir_all(&dest).is_err() {
            manifest.failures.push(LayoutFailure {
                relative_path: entry.rel.clone(),
                phase: "copy".to_string(),
                reason: format!("cannot create {}", dest.display()),
            });
        }
    }

    let mut module_dirs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let convert_at = |rel: &str, path: &Path, manifest: &mut GameLayoutManifest| {
        let dest = source_dir.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = dest.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            manifest.failures.push(LayoutFailure {
                relative_path: rel.to_string(),
                phase: "convert".to_string(),
                reason: format!("cannot create {}: {e}", parent.display()),
            });
            return;
        }
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                manifest.failures.push(LayoutFailure {
                    relative_path: rel.to_string(),
                    phase: "convert".to_string(),
                    reason: format!("read error: {e}"),
                });
                return;
            }
        };
        let source_size = data.len() as u64;
        let source_sha256 = ps5_format::sha256_hex(&data);
        match convert_one(&data) {
            Ok((elf, was_self)) => {
                let elf_sha256 = ps5_format::sha256_hex(&elf);
                let elf_size = elf.len() as u64;
                match atomic_write(&dest, &elf) {
                    Ok(()) => manifest.converted.push(ConvertedModule {
                        relative_path: rel.to_string(),
                        source_sha256,
                        elf_sha256,
                        was_self,
                        source_size,
                        elf_size,
                    }),
                    Err(reason) => manifest.failures.push(LayoutFailure {
                        relative_path: rel.to_string(),
                        phase: "convert".to_string(),
                        reason,
                    }),
                }
            }
            Err(reason) => manifest.failures.push(LayoutFailure {
                relative_path: rel.to_string(),
                phase: "convert".to_string(),
                reason,
            }),
        }
    };

    convert_at(&eboot_rel, &eboot.path, &mut manifest);
    for entry in by_depth.iter() {
        let name = entry
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if !is_module_name(&name) || under_excluded_dir(&entry.rel) {
            continue;
        }
        if let Some(parent) = entry.rel.rfind('/') {
            let dir = &entry.rel[..parent];
            let top = dir.split('/').next().unwrap_or("");
            if !AUTO_MODULE_DIRS.contains(&top) {
                module_dirs.insert(dir.to_string());
            }
        } else {
            module_dirs.insert(".".to_string());
        }
        convert_at(&entry.rel, &entry.path, &mut manifest);
    }
    manifest
        .converted
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    manifest.module_dirs = module_dirs.into_iter().collect();

    for note in notes {
        manifest.failures.push(LayoutFailure {
            relative_path: String::new(),
            phase: "walk".to_string(),
            reason: note,
        });
    }
    manifest.complete = manifest.failures.is_empty();

    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("manifest serialize error: {e}"))?;
    std::fs::write(out_dir.join(MANIFEST_NAME), format!("{manifest_json}\n"))
        .map_err(|e| format!("manifest write error: {e}"))?;

    Ok(GameLayoutReport {
        manifest,
        out_dir: out_dir.to_path_buf(),
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
    dirs.sort();
    dirs
}

pub fn layout_games(roms_dir: &Path, out_dir: &Path) -> Result<BatchLayoutReport, String> {
    if !roms_dir.is_dir() {
        return Err(format!("not a directory: {}", roms_dir.display()));
    }
    let mut report = BatchLayoutReport::default();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for game_dir in find_game_dirs(roms_dir) {
        let game_name = game_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let safe_name = sanitize_filename(game_name);
        if !seen.insert(safe_name.clone()) {
            continue;
        }
        report.total += 1;
        let game_out = out_dir.join(&safe_name);
        match layout_game(&game_dir, &game_out) {
            Ok(game) => {
                if game.manifest.complete {
                    report.succeeded += 1;
                } else {
                    report.failed += 1;
                    report.failures.push((
                        game_name.to_string(),
                        format!("{} file failure(s)", game.manifest.failures.len()),
                    ));
                }
                report.games.push(BatchGameLayout {
                    name: game_name.to_string(),
                    out_dir: game_out,
                    manifest: game.manifest,
                });
            }
            Err(reason) => {
                report.failed += 1;
                report.failures.push((game_name.to_string(), reason));
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ps5rs-anyps5-{}-{tag}", std::process::id()))
    }

    fn wu16_at(buf: &mut [u8], off: usize, v: u16) {
        buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }

    fn wu32_at(buf: &mut [u8], off: usize, v: u32) {
        buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn wu64_at(buf: &mut [u8], off: usize, v: u64) {
        buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
    }

    fn minimal_elf() -> Vec<u8> {
        use ps5_format::elf_constants::*;
        let mut file = vec![0u8; 0x1040];
        file[0..4].copy_from_slice(&ELF_MAGIC);
        file[EI_CLASS] = ELFCLASS64;
        file[EI_DATA] = ELFDATA2LSB;
        file[EI_VERSION] = 1;
        wu16_at(&mut file, 16, ET_EXEC);
        wu16_at(&mut file, 18, EM_X86_64);
        wu32_at(&mut file, 20, 1);
        wu64_at(&mut file, 24, 0x1000);
        wu64_at(&mut file, 32, 64);
        wu16_at(&mut file, 52, 64);
        wu16_at(&mut file, 54, 56);
        wu16_at(&mut file, 56, 1);
        wu32_at(&mut file, 64, PT_LOAD);
        wu32_at(&mut file, 68, PF_R | PF_X);
        wu64_at(&mut file, 72, 0x1000);
        wu64_at(&mut file, 80, 0x1000);
        wu64_at(&mut file, 88, 0);
        wu64_at(&mut file, 96, 0x40);
        wu64_at(&mut file, 104, 0x40);
        wu64_at(&mut file, 112, 0x1000);
        for b in file.iter_mut().skip(0x1000).take(0x40) {
            *b = 0xCC;
        }
        file
    }

    fn minimal_self() -> Vec<u8> {
        use ps5_format::self_constants::{SELF_MAGIC_PS5, SELF_SEGMENT_FLAG_DATA};
        let elf = minimal_elf();
        let elf_base: u64 = 64;
        let mut file = Vec::new();
        file.extend_from_slice(&SELF_MAGIC_PS5.to_be_bytes());
        file.push(0);
        file.push(1);
        file.push(1);
        file.push(0);
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&0x560u16.to_le_bytes());
        file.extend_from_slice(&0u16.to_le_bytes());
        file.extend_from_slice(&(64 + elf.len() as u64).to_le_bytes());
        file.extend_from_slice(&1u16.to_le_bytes());
        file.extend_from_slice(&0u16.to_le_bytes());
        file.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(&SELF_SEGMENT_FLAG_DATA.to_le_bytes());
        file.extend_from_slice(&elf_base.to_le_bytes());
        file.extend_from_slice(&(elf.len() as u64).to_le_bytes());
        file.extend_from_slice(&(elf.len() as u64).to_le_bytes());
        file.extend_from_slice(&elf);
        file
    }

    fn write_game(root: &Path) {
        std::fs::create_dir_all(root.join("sce_module")).unwrap();
        std::fs::create_dir_all(root.join("sce_sys")).unwrap();
        std::fs::create_dir_all(root.join("Media").join("Plugins")).unwrap();
        std::fs::create_dir_all(root.join("emptydir")).unwrap();
        std::fs::write(root.join("eboot.bin"), minimal_self()).unwrap();
        std::fs::write(root.join("sce_module").join("libc.prx"), minimal_elf()).unwrap();
        std::fs::write(
            root.join("Media").join("Plugins").join("plugin.prx"),
            minimal_self(),
        )
        .unwrap();
        std::fs::write(root.join("sce_sys").join("param.json"), b"{}").unwrap();
        std::fs::write(root.join("readme.txt"), b"hello").unwrap();
    }

    #[test]
    fn layout_preserves_originals_byte_identical() {
        let root = temp_root("preserve");
        let game = root.join("Game");
        write_game(&game);
        let out = root.join("out");
        let report = layout_game(&game, &out).unwrap();
        assert!(report.manifest.complete);
        for rel in [
            "eboot.bin",
            "sce_module/libc.prx",
            "Media/Plugins/plugin.prx",
            "sce_sys/param.json",
            "readme.txt",
        ] {
            let native = rel.replace('/', std::path::MAIN_SEPARATOR_STR);
            let original = std::fs::read(game.join(&native)).unwrap();
            let copied = std::fs::read(out.join("app0").join(&native)).unwrap();
            assert_eq!(original, copied, "mismatch for {rel}");
        }
        assert!(out.join("app0").join("emptydir").is_dir());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn layout_unwraps_to_elf_at_same_relative_paths() {
        let root = temp_root("unwrap");
        let game = root.join("Game");
        write_game(&game);
        let out = root.join("out");
        let report = layout_game(&game, &out).unwrap();
        assert!(report.manifest.complete);
        for rel in [
            "eboot.bin",
            "sce_module/libc.prx",
            "Media/Plugins/plugin.prx",
        ] {
            let native = rel.replace('/', std::path::MAIN_SEPARATOR_STR);
            let bytes = std::fs::read(out.join("source").join(&native)).unwrap();
            assert_eq!(&bytes[0..4], b"\x7fELF", "not ELF: {rel}");
        }
        let names: Vec<&str> = report
            .manifest
            .converted
            .iter()
            .map(|m| m.relative_path.as_str())
            .collect();
        assert!(names.contains(&"eboot.bin"));
        assert!(names.contains(&"sce_module/libc.prx"));
        assert!(names.contains(&"Media/Plugins/plugin.prx"));
        assert!(report.manifest.eboot.as_deref() == Some("eboot.bin"));
        assert!(report.manifest.module_dirs == vec!["Media/Plugins".to_string()]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn layout_nested_eboot_keeps_relative_path() {
        let root = temp_root("nested");
        let game = root.join("Game");
        std::fs::create_dir_all(game.join("inner")).unwrap();
        std::fs::write(game.join("inner").join("eboot.bin"), minimal_elf()).unwrap();
        let out = root.join("out");
        let report = layout_game(&game, &out).unwrap();
        assert!(report.manifest.complete);
        assert!(report.manifest.eboot.as_deref() == Some("inner/eboot.bin"));
        assert_eq!(
            std::fs::read(out.join("source").join("inner").join("eboot.bin")).unwrap(),
            minimal_elf()
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn layout_records_failure_without_going_silent() {
        let root = temp_root("failure");
        let game = root.join("Game");
        std::fs::create_dir_all(game.join("sce_module")).unwrap();
        std::fs::write(game.join("eboot.bin"), minimal_elf()).unwrap();
        std::fs::write(game.join("sce_module").join("broken.prx"), b"tiny").unwrap();
        let out = root.join("out");
        let report = layout_game(&game, &out).unwrap();
        assert!(!report.manifest.complete);
        assert_eq!(report.manifest.failures.len(), 1);
        assert_eq!(
            report.manifest.failures[0].relative_path,
            "sce_module/broken.prx"
        );
        assert_eq!(report.manifest.failures[0].phase, "convert");
        assert!(out.join("source").join("eboot.bin").exists());
        assert!(
            !out.join("source")
                .join("sce_module")
                .join("broken.prx")
                .exists()
        );
        assert!(
            !out.join("source")
                .join("sce_module")
                .join("broken.prx.partial")
                .exists()
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn layout_refuses_missing_eboot_and_bad_outputs() {
        let root = temp_root("guards");
        let game = root.join("Game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("readme.txt"), b"x").unwrap();
        let out = root.join("out");
        assert!(layout_game(&game, &out).is_err());
        std::fs::write(game.join("eboot.bin"), minimal_elf()).unwrap();
        assert!(layout_game(&game, &game).is_err());
        assert!(layout_game(&game, &game.join("sub")).is_err());
        assert!(layout_game(&root.join("missing"), &out).is_err());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn layout_games_builds_independent_trees() {
        let root = temp_root("batch");
        let a = root.join("games").join("GameA");
        let b = root.join("games").join("GameB");
        for game in [&a, &b] {
            std::fs::create_dir_all(game.join("sce_module")).unwrap();
            std::fs::write(game.join("eboot.bin"), minimal_elf()).unwrap();
            std::fs::write(game.join("sce_module").join("m.prx"), minimal_elf()).unwrap();
        }
        let out = root.join("out");
        let report = layout_games(&root.join("games"), &out).unwrap();
        assert_eq!(report.total, 2);
        assert_eq!(report.succeeded, 2);
        assert_eq!(report.failed, 0);
        assert!(report.failures.is_empty());
        for name in ["GameA", "GameB"] {
            let dir = out.join(name);
            assert!(dir.join("source").join("eboot.bin").exists());
            assert!(dir.join("app0").join("eboot.bin").exists());
            assert!(dir.join("anyps5-manifest.json").exists());
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn self_wrapped_modules_convert_with_was_self() {
        let root = temp_root("wasself");
        let game = root.join("Game");
        std::fs::create_dir_all(game.join("sce_module")).unwrap();
        std::fs::write(game.join("eboot.bin"), minimal_elf()).unwrap();
        std::fs::write(game.join("sce_module").join("wrapped.prx"), minimal_self()).unwrap();
        let out = root.join("out");
        let report = layout_game(&game, &out).unwrap();
        assert!(report.manifest.complete);
        let wrapped = report
            .manifest
            .converted
            .iter()
            .find(|m| m.relative_path == "sce_module/wrapped.prx")
            .unwrap();
        assert!(wrapped.was_self);
        let bytes =
            std::fs::read(out.join("source").join("sce_module").join("wrapped.prx")).unwrap();
        assert_eq!(&bytes[0..4], b"\x7fELF");
        std::fs::remove_dir_all(&root).unwrap();
    }
}
