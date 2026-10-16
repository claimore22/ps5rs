use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::fprx_nids::{Collected, FailedFile, collect_prx_files};

const PRIMARY_CSV_NAME: &str = "fsce-nids.csv";
const STATUS_VERIFIED: &str = "verified";

#[derive(Debug, Error)]
pub(crate) enum FsceError {
    #[error("not a directory: {}", .0.display())]
    RootNotDirectory(PathBuf),
    #[error("no matching module files found under {}", .0.display())]
    NoModules(PathBuf),
    #[error("cannot create report file {}: {source}", .path.display())]
    ReportCreate {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "refusing to write report to NID catalog path {}: choose a different --output",
        .0.display()
    )]
    CatalogOverwriteRefused(PathBuf),
    #[error("cannot write report to {}: {source}", .path.display())]
    ReportWrite { path: PathBuf, source: csv::Error },
}

#[derive(Debug, Clone)]
pub(crate) struct SceOccurrence {
    pub symbol: String,
    pub offset: u64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FsceStats {
    pub modules_found: usize,
    pub modules_scanned: usize,
    pub modules_failed: usize,
    pub modules_skipped_backup: usize,
    pub traversal_errors: Vec<String>,
    pub failed_files: Vec<FailedFile>,
    pub occurrences: usize,
}

impl FsceStats {
    pub fn incomplete(&self) -> bool {
        self.modules_failed > 0 || !self.traversal_errors.is_empty()
    }
}

fn is_sym_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_glue_byte(b: u8) -> bool {
    matches!(b, b'#' | b'@' | b'.' | b':' | b'$' | b'~')
}

fn match_at(data: &[u8], start: usize) -> Option<SceOccurrence> {
    let head = data.get(start..start + 4)?;
    if (head[0] != b's' && head[0] != b'S')
        || head[1] != b'c'
        || head[2] != b'e'
        || !head[3].is_ascii_uppercase()
    {
        return None;
    }
    if start > 0 {
        let prev = data[start - 1];
        if is_sym_byte(prev) || is_glue_byte(prev) {
            return None;
        }
    }
    let mut end = start + 4;
    while end < data.len() && is_sym_byte(data[end]) {
        end += 1;
    }
    if end < data.len() {
        let next = data[end];
        if is_sym_byte(next) || is_glue_byte(next) {
            return None;
        }
    }
    let symbol = std::str::from_utf8(&data[start..end]).ok()?;
    Some(SceOccurrence {
        symbol: symbol.to_string(),
        offset: start as u64,
    })
}

pub(crate) fn scan_sce_symbols(data: &[u8]) -> Vec<SceOccurrence> {
    let mut out = Vec::new();
    if data.len() < 4 {
        return out;
    }
    let mut i = 0;
    while i + 4 <= data.len() {
        if (data[i] == b's' || data[i] == b'S')
            && let Some(occ) = match_at(data, i)
        {
            out.push(occ);
        }
        i += 1;
    }
    out
}

pub(crate) fn nid_forms(symbol: &str) -> (String, String) {
    let encoding = ps5_nid::hash(symbol);
    let nid = ps5_nid::nid_to_u64(&encoding)
        .map(|v| format!("0x{v:016x}"))
        .unwrap_or_default();
    debug_assert!(!nid.is_empty());
    (nid, encoding)
}

fn write_header(writer: &mut csv::Writer<std::fs::File>, path: &Path) -> Result<(), FsceError> {
    writer
        .write_record([
            "symbol",
            "nid",
            "nid_encoding",
            "module_name",
            "relative_path",
            "file_offset",
            "algorithm_status",
        ])
        .map_err(|source| FsceError::ReportWrite {
            path: path.to_path_buf(),
            source,
        })?;
    Ok(())
}

pub(crate) fn run_scan(
    root: &Path,
    output: &Path,
    include_eboot: bool,
    read_file: impl for<'a> Fn(&'a Path) -> std::io::Result<Vec<u8>>,
) -> Result<FsceStats, FsceError> {
    if !root.is_dir() {
        return Err(FsceError::RootNotDirectory(root.to_path_buf()));
    }
    if output
        .file_name()
        .is_some_and(|n| n.eq_ignore_ascii_case("nids.csv"))
    {
        return Err(FsceError::CatalogOverwriteRefused(output.to_path_buf()));
    }

    let mut collected = Collected {
        files: Vec::new(),
        skipped_backups: 0,
        traversal_errors: Vec::new(),
    };
    collect_prx_files(root, &mut collected, 0, include_eboot);
    collected.files.sort();
    if collected.files.is_empty() {
        return Err(FsceError::NoModules(root.to_path_buf()));
    }

    let file = std::fs::File::create(output).map_err(|source| FsceError::ReportCreate {
        path: output.to_path_buf(),
        source,
    })?;
    let mut writer = csv::Writer::from_writer(file);
    write_header(&mut writer, output)?;

    let mut stats = FsceStats {
        modules_found: collected.files.len(),
        modules_skipped_backup: collected.skipped_backups,
        traversal_errors: collected.traversal_errors,
        ..Default::default()
    };

    for path in &collected.files {
        let relative_path = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());
        let module_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| relative_path.clone());
        let data = match read_file(path) {
            Ok(d) => d,
            Err(e) => {
                stats.modules_failed += 1;
                stats.failed_files.push(FailedFile {
                    relative_path,
                    reason: e.to_string(),
                });
                continue;
            }
        };
        for occ in scan_sce_symbols(&data) {
            let (nid, encoding) = nid_forms(&occ.symbol);
            let offset = format!("0x{:x}", occ.offset);
            writer
                .write_record([
                    occ.symbol.as_str(),
                    nid.as_str(),
                    encoding.as_str(),
                    module_name.as_str(),
                    relative_path.as_str(),
                    offset.as_str(),
                    STATUS_VERIFIED,
                ])
                .map_err(|source| FsceError::ReportWrite {
                    path: output.to_path_buf(),
                    source,
                })?;
            stats.occurrences += 1;
        }
        stats.modules_scanned += 1;
    }

    writer.flush().map_err(|source| FsceError::ReportWrite {
        path: output.to_path_buf(),
        source: source.into(),
    })?;
    Ok(stats)
}

pub(crate) fn cmd_fsce_nids(path: &Path, output: &Option<PathBuf>, include_eboot: bool) {
    fn read_bytes(path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    let out_path = output
        .clone()
        .unwrap_or_else(|| path.join(PRIMARY_CSV_NAME));
    eprintln!("Scanning {} for SCE symbols...", path.display());
    let stats = match run_scan(path, &out_path, include_eboot, read_bytes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    eprintln!(
        "Modules: {} found, {} scanned, {} failed, {} backup(s) skipped",
        stats.modules_found,
        stats.modules_scanned,
        stats.modules_failed,
        stats.modules_skipped_backup
    );
    for failed in &stats.failed_files {
        eprintln!("  failed: {} ({})", failed.relative_path, failed.reason);
    }
    for err in &stats.traversal_errors {
        eprintln!("  traversal: {err}");
    }
    eprintln!("SCE symbol occurrences: {}", stats.occurrences);
    eprintln!("Report written to {}", out_path.display());
    if stats.incomplete() {
        eprintln!("INCOMPLETE: some modules could not be read; results are partial");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ps5rs-fsce-{}-{tag}", std::process::id()))
    }

    fn read_bytes(path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    #[test]
    fn qualifies_sce_followed_by_uppercase() {
        for name in [
            "sceKernelAprResolveFilepathsWithPrefixToIds",
            "sceKernelWriteModifyMtypeProtectCommand",
            "sceKernelWriteUnmapCommand",
            "sceKernelMapperSetPageTablePoolOccupancyNotification",
            "sceKernelGiveDirectMemoryToMapper",
            "sceA",
            "SceLibcHeap",
            "SceKernelInternalMemory",
            "SceNpCppWebApiCallbackMutex",
            "SceGameRight",
        ] {
            let mut data = vec![0u8];
            data.extend_from_slice(name.as_bytes());
            data.push(0);
            let found = scan_sce_symbols(&data);
            assert_eq!(found.len(), 1, "should qualify {name}");
            assert_eq!(found[0].symbol, name);
            assert_eq!(found[0].offset, 1);
        }
    }

    #[test]
    fn rejects_sce_followed_by_lowercase_or_digit() {
        for bad in [
            "scekernelFoo",
            "sce1Foo",
            "sce_foo",
            "sce",
            "Scelibc",
            "Sce1Foo",
        ] {
            let mut data = vec![0u8];
            data.extend_from_slice(bad.as_bytes());
            data.push(0);
            assert!(scan_sce_symbols(&data).is_empty(), "should reject {bad}");
        }
    }

    #[test]
    fn rejects_strings_not_beginning_with_sce() {
        for bad in [
            "KernelFoo",
            "asceKernelFoo",
            "aSceKernelFoo",
            "xsce",
            "SCEFoo",
            "sCeFoo",
            "scEfoo",
            "SCELIBC",
        ] {
            let data = format!("\0{bad}\0").into_bytes();
            assert!(scan_sce_symbols(&data).is_empty(), "should reject {bad}");
        }
    }

    #[test]
    fn rejects_mid_token_and_glued_occurrences() {
        for bad in [
            "xsceKernelFoo",
            "xSceKernelFoo",
            "1sceKernelFoo",
            "_sceKernelFoo",
            "#sceKernelFoo",
            ".sceKernelFoo",
            "ns::sceKernelFoo",
            "sceKernelFoo#D",
            "sceKernelFoo.prx",
            "sceKernelFoo@1",
            "sceKernelFoo:bar",
        ] {
            let mut data = vec![0u8];
            data.extend_from_slice(bad.as_bytes());
            data.push(0);
            assert!(scan_sce_symbols(&data).is_empty(), "should reject {bad}");
        }
    }

    #[test]
    fn accepts_dash_delimited_tokens() {
        let found = scan_sce_symbols(b"\0-sceKernelFoo-\0");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].symbol, "sceKernelFoo");
        assert_eq!(found[0].offset, 2);
        let found = scan_sce_symbols(b"call sceKernelFoo now\0");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].symbol, "sceKernelFoo");
        assert_eq!(found[0].offset, 5);
    }

    #[test]
    fn handles_file_boundaries() {
        let found = scan_sce_symbols(b"sceKernelFoo");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].offset, 0);
        let found = scan_sce_symbols(b"--sceKernelFoo");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].offset, 2);
        assert!(scan_sce_symbols(b"sce").is_empty());
        assert!(scan_sce_symbols(b"").is_empty());
    }

    #[test]
    fn finds_multiple_symbols_and_keeps_every_repeat() {
        let data = b"\0sceKernelFoo\0pad\0sceKernelFoo\0scePthreadBar\0sceKernelFoo\0".to_vec();
        let found = scan_sce_symbols(&data);
        assert_eq!(found.len(), 4);
        assert_eq!(found[0].offset, 1);
        assert_eq!(found[1].offset, 18);
        assert_eq!(found[2].symbol, "scePthreadBar");
        assert_eq!(found[3].offset, 45);
        let repeats: Vec<&SceOccurrence> = found
            .iter()
            .filter(|o| o.symbol == "sceKernelFoo")
            .collect();
        assert_eq!(repeats.len(), 3);
        assert_ne!(repeats[0].offset, repeats[1].offset);
        assert_ne!(repeats[1].offset, repeats[2].offset);
    }

    #[test]
    fn nid_forms_match_trusted_vectors() {
        assert_eq!(ps5_nid::hash("memcpy"), "Q3VBxCXhUHs");
        assert_eq!(ps5_nid::hash("sceKernelSleep"), "-ZR+hG7aDHw");
        assert_eq!(ps5_nid::hash("scePthreadCreate"), "6UgtwV+0zb4");
        assert_eq!(ps5_nid::nid_to_u64("-ZR+hG7aDHw"), Some(0xFD947E846EDA0C7C));
        assert_eq!(ps5_nid::nid_to_u64("6UgtwV+0zb4"), Some(0xE9482DC15FB4CDBE));
        let (nid, encoding) = nid_forms("sceKernelSleep");
        assert_eq!(nid, "0xfd947e846eda0c7c");
        assert_eq!(encoding, "-ZR+hG7aDHw");
        assert_eq!(nid.len(), 18);
        assert_eq!(encoding.len(), 11);
    }

    #[test]
    fn refuses_nids_csv_output() {
        let root = temp_root("guard");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.prx"), b"\0sceKernelFoo\0").unwrap();
        let out = root.join("nids.csv");
        assert!(matches!(
            run_scan(&root, &out, false, read_bytes),
            Err(FsceError::CatalogOverwriteRefused(_))
        ));
        assert!(!out.exists());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn leaves_decoy_nids_csv_untouched() {
        let root = temp_root("decoy");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("nids.csv"), b"original-catalog-bytes").unwrap();
        std::fs::write(root.join("a.prx"), b"\0sceKernelFoo\0").unwrap();
        let out = root.join("fsce-nids.csv");
        let stats = run_scan(&root, &out, false, read_bytes).unwrap();
        assert_eq!(stats.occurrences, 1);
        assert_eq!(
            std::fs::read(root.join("nids.csv")).unwrap(),
            b"original-catalog-bytes"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn run_scan_rejects_bad_inputs() {
        let missing = PathBuf::from("definitely_missing_ps5rs_fsce_dir");
        assert!(matches!(
            run_scan(&missing, &PathBuf::from("o.csv"), false, read_bytes),
            Err(FsceError::RootNotDirectory(_))
        ));
        let root = temp_root("empty");
        std::fs::create_dir_all(&root).unwrap();
        assert!(matches!(
            run_scan(&root, &root.join("o.csv"), false, read_bytes),
            Err(FsceError::NoModules(_))
        ));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn run_scan_records_unreadable_files() {
        let root = temp_root("partial");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("good.prx"), b"\0sceKernelFoo\0").unwrap();
        std::fs::write(root.join("bad.prx"), b"x").unwrap();
        let reader = |p: &Path| -> std::io::Result<Vec<u8>> {
            if p.file_name().is_some_and(|n| n == "bad.prx") {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "denied",
                ))
            } else {
                std::fs::read(p)
            }
        };
        let out = root.join("o.csv");
        let stats = run_scan(&root, &out, false, reader).unwrap();
        assert_eq!(stats.modules_found, 2);
        assert_eq!(stats.modules_scanned, 1);
        assert_eq!(stats.modules_failed, 1);
        assert_eq!(stats.failed_files[0].relative_path, "bad.prx");
        assert!(stats.incomplete());
        assert_eq!(stats.occurrences, 1);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn include_eboot_scans_eboot_bin() {
        let root = temp_root("eboot");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("eboot.bin"), b"\0sceKernelFoo\0").unwrap();
        std::fs::write(root.join("eboot.bin.esbak"), b"\0sceKernelFoo\0").unwrap();
        let out = root.join("o.csv");
        assert!(matches!(
            run_scan(&root, &out, false, read_bytes),
            Err(FsceError::NoModules(_))
        ));
        let stats = run_scan(&root, &out, true, read_bytes).unwrap();
        assert_eq!(stats.modules_found, 1);
        assert_eq!(stats.modules_scanned, 1);
        assert_eq!(stats.occurrences, 1);
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(text.contains("sceKernelFoo"));
        assert!(text.contains("eboot.bin"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn header_written_when_no_symbols_found() {
        let root = temp_root("nosyms");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.prx"), b"\0no sce symbols here\0").unwrap();
        let out = root.join("o.csv");
        let stats = run_scan(&root, &out, false, read_bytes).unwrap();
        assert_eq!(stats.occurrences, 0);
        assert!(!stats.incomplete());
        let text = std::fs::read_to_string(&out).unwrap();
        assert_eq!(
            text.lines().next().unwrap(),
            "symbol,nid,nid_encoding,module_name,relative_path,file_offset,algorithm_status"
        );
        assert_eq!(text.lines().count(), 1);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn end_to_end_sorted_no_dedup_across_files() {
        let root = temp_root("e2e");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(
            root.join("b.prx"),
            b"\0sceKernelWriteUnmapCommand\0x\0sceKernelWriteUnmapCommand\0",
        )
        .unwrap();
        std::fs::write(
            root.join("sub").join("a.SPRX"),
            b"\0sceKernelWriteUnmapCommand\0sceKernelFoo\0",
        )
        .unwrap();
        std::fs::write(root.join("we,ird.PRX"), b"\0sceKernelFoo\0").unwrap();
        let out = root.join("o.csv");
        let stats = run_scan(&root, &out, false, read_bytes).unwrap();
        assert_eq!(stats.modules_found, 3);
        assert_eq!(stats.occurrences, 5);
        let text = std::fs::read_to_string(&out).unwrap();
        let mut records: Vec<csv::StringRecord> = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(text.as_bytes())
            .into_records()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let header = records.remove(0);
        assert_eq!(header.get(0).unwrap(), "symbol");
        assert_eq!(header.get(6).unwrap(), "algorithm_status");
        assert_eq!(records.len(), 5);
        let b_rows: Vec<_> = records
            .iter()
            .filter(|r| r.get(3).unwrap() == "b.prx")
            .collect();
        assert_eq!(b_rows.len(), 2);
        let sub_rows: Vec<_> = records
            .iter()
            .filter(|r| r.get(3).unwrap() == "a.SPRX")
            .collect();
        assert_eq!(sub_rows.len(), 2);
        let weird: Vec<_> = records
            .iter()
            .filter(|r| r.get(3).unwrap() == "we,ird.PRX")
            .collect();
        assert_eq!(weird.len(), 1);
        let keys: Vec<(&str, &str)> = records
            .iter()
            .map(|r| (r.get(4).unwrap(), r.get(5).unwrap()))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
        assert!(records.iter().all(|r| r.get(6).unwrap() == "verified"));
        let (nid, encoding) = nid_forms("sceKernelWriteUnmapCommand");
        for row in records
            .iter()
            .filter(|r| r.get(0).unwrap() == "sceKernelWriteUnmapCommand")
        {
            assert_eq!(row.get(1).unwrap(), nid);
            assert_eq!(row.get(2).unwrap(), encoding);
        }
        std::fs::remove_dir_all(&root).unwrap();
    }
}
