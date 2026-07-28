use std::collections::{BTreeSet, HashMap};
use std::path::Path;

#[derive(Default)]
pub struct NidEntry {
    pub names: BTreeSet<String>,
    pub libraries: BTreeSet<String>,
    pub tags: BTreeSet<String>,
    pub sources: BTreeSet<String>,
}

impl NidEntry {
    /// Returns an arbitrary name from the catalog.
    /// Name ranking is deferred to a future version.
    pub fn primary_name(&self) -> Option<&str> {
        self.names.iter().next().map(|s| s.as_str())
    }

    pub fn has_name(&self, name: &str) -> bool {
        self.names.contains(name)
    }
}

pub struct Catalog {
    by_nid: HashMap<String, NidEntry>,
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}

impl Catalog {
    pub fn new() -> Self {
        let mut cat = Self {
            by_nid: HashMap::new(),
        };
        cat.add_builtins();
        cat
    }

    pub fn add(&mut self, name: &str) {
        let nid = super::hash(name);
        let entry = self.by_nid.entry(nid).or_default();
        entry.names.insert(name.to_string());
    }

    pub fn insert(
        &mut self,
        name: &str,
        library: Option<&str>,
        tag: Option<&str>,
        source: Option<&str>,
    ) {
        let nid = super::hash(name);
        let entry = self.by_nid.entry(nid).or_default();
        entry.names.insert(name.to_string());
        if let Some(lib) = library {
            entry.libraries.insert(lib.to_string());
        }
        if let Some(tag) = tag {
            entry.tags.insert(tag.to_string());
        }
        if let Some(src) = source {
            entry.sources.insert(src.to_string());
        }
    }

    pub fn resolve(&self, nid: &str) -> Option<&NidEntry> {
        self.by_nid.get(nid)
    }

    pub fn size(&self) -> usize {
        self.by_nid.len()
    }

    pub fn load_names_file(&mut self, path: &str) -> usize {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return 0,
        };

        let mut count = 0;
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                self.add(line);
                count += 1;
            }
        }
        count
    }

    pub fn load_nids_csv(&mut self, content: &str) -> usize {
        let first_non_empty = content.lines().find(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        });

        match first_non_empty {
            Some(line) if line.contains(',') => self.load_nids_csv_rich(line, content),
            _ => self.load_nids_csv_legacy(content),
        }
    }

    fn load_nids_csv_rich(&mut self, first_line: &str, content: &str) -> usize {
        let first_col = first_line.split(',').next().unwrap_or("").trim();
        let skip_header = first_col == "nid";

        let mut count = 0;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if skip_header && line == first_line.trim() {
                continue;
            }

            let mut cols = line.splitn(5, ',');
            let nid = cols.next().unwrap_or("").trim();
            let name = cols.next().unwrap_or("").trim();
            let library = cols.next().unwrap_or("").trim();
            let tag = cols.next().unwrap_or("").trim();
            let source = cols.next().unwrap_or("").trim();

            if !nid.is_empty() && !name.is_empty() {
                let entry = self.by_nid.entry(nid.to_string()).or_default();
                entry.names.insert(name.to_string());
                if !library.is_empty() {
                    entry.libraries.insert(library.to_string());
                }
                if !tag.is_empty() {
                    entry.tags.insert(tag.to_string());
                }
                if !source.is_empty() {
                    entry.sources.insert(source.to_string());
                }
                count += 1;
            }
        }
        count
    }

    fn load_nids_csv_legacy(&mut self, content: &str) -> usize {
        let mut count = 0;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(space_pos) = line.find(' ') {
                let nid = &line[..space_pos];
                let name = &line[space_pos + 1..];
                if !nid.is_empty() && !name.is_empty() {
                    let entry = self.by_nid.entry(nid.to_string()).or_default();
                    entry.names.insert(name.to_string());
                    count += 1;
                }
            }
        }
        count
    }

    pub fn load_nids_csv_file<P: AsRef<Path>>(&mut self, path: P) -> Result<usize, std::io::Error> {
        let data = std::fs::read_to_string(path)?;
        Ok(self.load_nids_csv(&data))
    }

    fn add_builtins(&mut self) {
        let names = [
            "memcpy",
            "memmove",
            "memset",
            "memcmp",
            "memchr",
            "strlen",
            "strnlen",
            "strcmp",
            "strncmp",
            "strcpy",
            "strncpy",
            "strcat",
            "strncat",
            "strchr",
            "strrchr",
            "strstr",
            "strtol",
            "strtoul",
            "strtod",
            "strdup",
            "snprintf",
            "vsnprintf",
            "sprintf",
            "printf",
            "fprintf",
            "puts",
            "putchar",
            "malloc",
            "calloc",
            "realloc",
            "free",
            "aligned_alloc",
            "posix_memalign",
            "abort",
            "exit",
            "atexit",
            "__cxa_atexit",
            "__cxa_finalize",
            "qsort",
            "bsearch",
            "getenv",
            "rand",
            "srand",
            "fopen",
            "fclose",
            "fread",
            "fwrite",
            "fseek",
            "ftell",
            "fflush",
            "fgets",
            "fputs",
            "setvbuf",
            "pow",
            "sqrt",
            "sin",
            "cos",
            "tan",
            "atan2",
            "floor",
            "ceil",
            "fmod",
            "log",
            "exp",
            "__stack_chk_fail",
            "__stack_chk_guard",
            "__memcpy_chk",
            "__memset_chk",
            "__cxa_guard_acquire",
            "__cxa_guard_release",
            "__cxa_throw",
            "__cxa_begin_catch",
            "__cxa_end_catch",
            "_Znwm",
            "_Znam",
            "_ZdlPv",
            "_ZdaPv",
            "__gxx_personality_v0",
            "scePthreadCreate",
            "scePthreadJoin",
            "scePthreadExit",
            "scePthreadMutexInit",
            "scePthreadMutexLock",
            "scePthreadMutexUnlock",
            "scePthreadMutexDestroy",
            "scePthreadMutexTrylock",
            "scePthreadCondInit",
            "scePthreadCondWait",
            "scePthreadCondSignal",
            "scePthreadCondBroadcast",
            "scePthreadCondDestroy",
            "scePthreadSelf",
            "scePthreadOnce",
            "scePthreadKeyCreate",
            "scePthreadSetspecific",
            "scePthreadGetspecific",
            "scePthreadKeyDelete",
            "scePthreadEqual",
            "scePthreadYield",
            "scePthreadAttrInit",
            "scePthreadAttrDestroy",
            "scePthreadAttrSetstacksize",
            "scePthreadAttrSetdetachstate",
            "pthread_create",
            "pthread_join",
            "pthread_mutex_lock",
            "pthread_mutex_unlock",
            "pthread_cond_wait",
            "pthread_cond_signal",
            "pthread_self",
            "pthread_once",
            "sceKernelAllocateDirectMemory",
            "sceKernelReleaseDirectMemory",
            "sceKernelMapDirectMemory",
            "sceKernelMapNamedFlexibleMemory",
            "sceKernelMapFlexibleMemory",
            "sceKernelReserveVirtualRange",
            "sceKernelMunmap",
            "sceKernelMmap",
            "sceKernelVirtualQuery",
            "sceKernelSetVirtualRangeName",
            "sceKernelGetProcParam",
            "sceKernelLoadStartModule",
            "sceKernelDlsym",
            "sceKernelGetModuleInfo",
            "sceKernelError",
            "sceKernelUsleep",
            "sceKernelSleep",
            "sceKernelNanosleep",
            "sceKernelGettimeofday",
            "sceKernelClockGettime",
            "sceKernelGetProcessTime",
            "sceKernelGetTscFrequency",
            "sceKernelCreateEqueue",
            "sceKernelWaitEqueue",
            "sceKernelDeleteEqueue",
            "sceKernelCreateEventFlag",
            "sceKernelWaitEventFlag",
            "sceKernelSetEventFlag",
            "sceKernelCreateSema",
            "sceKernelWaitSema",
            "sceKernelSignalSema",
            "sceKernelOpen",
            "sceKernelClose",
            "sceKernelRead",
            "sceKernelWrite",
            "sceKernelLseek",
            "sceKernelStat",
            "sceKernelFstat",
            "open",
            "close",
            "read",
            "write",
            "lseek",
            "stat",
            "fstat",
            "mmap",
            "munmap",
            "clock_gettime",
            "gettimeofday",
            "nanosleep",
            "usleep",
        ];

        for name in &names {
            self.add(name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_catalog_has_builtins() {
        let cat = Catalog::new();
        assert!(
            cat.size() >= 90,
            "expected at least 90 builtins, got {}",
            cat.size()
        );
    }

    #[test]
    fn resolve_memcpy() {
        let cat = Catalog::new();
        let nid = crate::hash("memcpy");
        let entry = cat.resolve(&nid).unwrap();
        assert_eq!(entry.primary_name(), Some("memcpy"));
    }

    #[test]
    fn resolve_sce_pthread_create() {
        let cat = Catalog::new();
        let nid = crate::hash("scePthreadCreate");
        let entry = cat.resolve(&nid).unwrap();
        assert_eq!(entry.primary_name(), Some("scePthreadCreate"));
    }

    #[test]
    fn resolve_unknown_returns_none() {
        let cat = Catalog::new();
        assert!(cat.resolve("ZZZZZZZZZZZ").is_none());
    }

    #[test]
    fn add_custom_name() {
        let mut cat = Catalog::new();
        cat.add("myCustomFunction");
        let nid = crate::hash("myCustomFunction");
        let entry = cat.resolve(&nid).unwrap();
        assert!(entry.has_name("myCustomFunction"));
    }

    #[test]
    fn add_duplicate_does_not_grow() {
        let mut cat = Catalog::new();
        let before = cat.size();
        cat.add("memcpy");
        assert_eq!(cat.size(), before);
    }

    #[test]
    fn add_multiple_custom_names() {
        let mut cat = Catalog::new();
        let before = cat.size();
        cat.add("func_a");
        cat.add("func_b");
        cat.add("func_c");
        assert_eq!(cat.size(), before + 3);
        assert!(
            cat.resolve(&crate::hash("func_a"))
                .unwrap()
                .has_name("func_a")
        );
        assert!(
            cat.resolve(&crate::hash("func_b"))
                .unwrap()
                .has_name("func_b")
        );
        assert!(
            cat.resolve(&crate::hash("func_c"))
                .unwrap()
                .has_name("func_c")
        );
    }

    #[test]
    fn insert_with_metadata() {
        let mut cat = Catalog::new();
        cat.insert(
            "myFunc",
            Some("libSceMyLib"),
            Some("filesystem"),
            Some("sdk"),
        );
        let entry = cat.resolve(&crate::hash("myFunc")).unwrap();
        assert!(entry.has_name("myFunc"));
        assert!(entry.libraries.contains("libSceMyLib"));
        assert!(entry.tags.contains("filesystem"));
        assert!(entry.sources.contains("sdk"));
    }

    #[test]
    fn insert_merges_metadata() {
        let mut cat = Catalog::new();
        cat.insert("myFunc", Some("libA"), None, None);
        cat.insert("myFunc", Some("libB"), Some("audio"), None);
        let entry = cat.resolve(&crate::hash("myFunc")).unwrap();
        assert!(entry.has_name("myFunc"));
        assert!(entry.libraries.contains("libA"));
        assert!(entry.libraries.contains("libB"));
        assert!(entry.tags.contains("audio"));
        assert!(entry.sources.is_empty());
    }

    #[test]
    fn load_names_file_missing() {
        let mut cat = Catalog::new();
        let loaded = cat.load_names_file("nonexistent_file.txt");
        assert_eq!(loaded, 0);
    }

    #[test]
    fn load_names_file_valid() {
        let dir = std::env::temp_dir().join("ps5rs_test_catalog");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("nids.txt");
        std::fs::write(&path, "# comment line\nfoo\nbar\n\nbaz\n").unwrap();
        let mut cat = Catalog::new();
        let before = cat.size();
        let loaded = cat.load_names_file(path.to_str().unwrap());
        assert_eq!(loaded, 3);
        assert_eq!(cat.size(), before + 3);
        assert!(cat.resolve(&crate::hash("foo")).unwrap().has_name("foo"));
        assert!(cat.resolve(&crate::hash("bar")).unwrap().has_name("bar"));
        assert!(cat.resolve(&crate::hash("baz")).unwrap().has_name("baz"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_nids_csv_legacy_basic() {
        let mut cat = Catalog::new();
        let before = cat.size();
        let csv = "mFq1M6vw-JM ACProcessMain\n05Uo75yDn-s AES_cfb128_encrypt\n";
        let loaded = cat.load_nids_csv(csv);
        assert_eq!(loaded, 2);
        assert_eq!(cat.size(), before + 2);
        let e1 = cat.resolve("mFq1M6vw-JM").unwrap();
        assert!(e1.has_name("ACProcessMain"));
        let e2 = cat.resolve("05Uo75yDn-s").unwrap();
        assert!(e2.has_name("AES_cfb128_encrypt"));
    }

    #[test]
    fn load_nids_csv_legacy_skips_blank_and_comments() {
        let mut cat = Catalog::new();
        let before = cat.size();
        let csv = "# comment\n\n  \nAAAABBBCCCC name1\n";
        let loaded = cat.load_nids_csv(csv);
        assert_eq!(loaded, 1);
        assert_eq!(cat.size(), before + 1);
    }

    #[test]
    fn load_nids_csv_legacy_overwrites_builtins() {
        let mut cat = Catalog::new();
        let memcpy_nid = crate::hash("memcpy");
        assert!(cat.resolve(&memcpy_nid).unwrap().has_name("memcpy"));
        let csv = &format!("{memcpy_nid} not_memcpy\n");
        let loaded = cat.load_nids_csv(csv);
        assert_eq!(loaded, 1);
        let entry = cat.resolve(&memcpy_nid).unwrap();
        assert!(entry.has_name("not_memcpy"));
    }

    #[test]
    fn load_nids_csv_empty() {
        let mut cat = Catalog::new();
        let before = cat.size();
        let loaded = cat.load_nids_csv("");
        assert_eq!(loaded, 0);
        assert_eq!(cat.size(), before);
    }

    #[test]
    fn load_nids_csv_rich_with_header() {
        let mut cat = Catalog::new();
        let csv =
            "nid,name,library,tag,source\n246322a3edb52f87,posix_mkdir,libkernel,filesystem,sdk\n";
        let loaded = cat.load_nids_csv(csv);
        assert_eq!(loaded, 1);
        let entry = cat.resolve("246322a3edb52f87").unwrap();
        assert!(entry.has_name("posix_mkdir"));
        assert!(entry.libraries.contains("libkernel"));
        assert!(entry.tags.contains("filesystem"));
        assert!(entry.sources.contains("sdk"));
    }

    #[test]
    fn load_nids_csv_rich_without_header() {
        let mut cat = Catalog::new();
        let csv = "246322a3edb52f87,posix_mkdir,libkernel,filesystem,sdk\n";
        let loaded = cat.load_nids_csv(csv);
        assert_eq!(loaded, 1);
        let entry = cat.resolve("246322a3edb52f87").unwrap();
        assert!(entry.has_name("posix_mkdir"));
        assert!(entry.libraries.contains("libkernel"));
    }

    #[test]
    fn load_nids_csv_rich_partial_columns() {
        let mut cat = Catalog::new();
        let csv = "nid,name,library\nAAAABBBCCCC,myFunc,libFoo\n";
        let loaded = cat.load_nids_csv(csv);
        assert_eq!(loaded, 1);
        let entry = cat.resolve("AAAABBBCCCC").unwrap();
        assert!(entry.has_name("myFunc"));
        assert!(entry.libraries.contains("libFoo"));
        assert!(entry.tags.is_empty());
        assert!(entry.sources.is_empty());
    }

    #[test]
    fn load_nids_csv_rich_merges() {
        let mut cat = Catalog::new();
        let csv = "nid,name,library,tag,source\nAAAABBBCCCC,myFunc,libA,,sdk\nAAAABBBCCCC,myFunc,libB,audio,\n";
        let loaded = cat.load_nids_csv(csv);
        assert_eq!(loaded, 2);
        let entry = cat.resolve("AAAABBBCCCC").unwrap();
        assert!(entry.has_name("myFunc"));
        assert!(entry.libraries.contains("libA"));
        assert!(entry.libraries.contains("libB"));
        assert!(entry.tags.contains("audio"));
        assert!(entry.sources.contains("sdk"));
    }

    #[test]
    fn load_nids_csv_file_missing() {
        let mut cat = Catalog::new();
        let result = cat.load_nids_csv_file("nonexistent_file.csv");
        assert!(result.is_err());
    }

    #[test]
    fn load_nids_csv_file_valid() {
        let dir = std::env::temp_dir().join("ps5rs_test_catalog_csv");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("extra.csv");
        std::fs::write(&path, "AAAABBBCCCC funcX\nDDDDEEEEFFFF funcY\n").unwrap();
        let mut cat = Catalog::new();
        let before = cat.size();
        let result = cat.load_nids_csv_file(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
        assert_eq!(cat.size(), before + 2);
        let e1 = cat.resolve("AAAABBBCCCC").unwrap();
        assert!(e1.has_name("funcX"));
        let e2 = cat.resolve("DDDDEEEEFFFF").unwrap();
        assert!(e2.has_name("funcY"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_nids_csv_file_accepts_pathbuf() {
        let dir = std::env::temp_dir().join("ps5rs_test_catalog_pathbuf");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("extra.csv");
        std::fs::write(&path, "111122223333 testfunc\n").unwrap();
        let mut cat = Catalog::new();
        let before = cat.size();
        let result = cat.load_nids_csv_file(path.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
        assert_eq!(cat.size(), before + 1);
        let entry = cat.resolve("111122223333").unwrap();
        assert!(entry.has_name("testfunc"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn builtins_resolve_all_entries() {
        let cat = Catalog::new();
        let names = [
            "memcpy",
            "memset",
            "malloc",
            "free",
            "printf",
            "sceKernelSleep",
            "scePthreadCreate",
            "__cxa_atexit",
        ];
        for name in &names {
            let nid = crate::hash(name);
            let entry = cat.resolve(&nid).unwrap_or_else(|| {
                panic!("failed to resolve {name}");
            });
            assert!(entry.has_name(name), "entry for {name} missing name");
        }
    }
}
