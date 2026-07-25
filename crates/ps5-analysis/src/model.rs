use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Platform {
    Ps4,
    Ps5,
    RawElf,
    Unknown,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Ps4 => write!(f, "PS4"),
            Platform::Ps5 => write!(f, "PS5"),
            Platform::RawElf => write!(f, "RawElf"),
            Platform::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub nid_hash: String,
    pub resolved_name: String,
    pub library_id: u16,
    pub library_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAnalysis {
    pub name: String,
    pub path: String,
    pub sha256: String,
    pub file_size: u64,
    pub platform: Platform,
    pub entry_point: u64,
    pub is_self: bool,
    pub imports: Vec<ImportInfo>,
    pub import_libs: Vec<LibInfo>,
    pub needed_files: Vec<String>,
    pub num_relocations: usize,
    pub num_symbols: usize,
    pub has_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibInfo {
    pub id: u16,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisDatabase {
    pub schema_version: u32,
    pub tool: String,
    pub games: Vec<GameAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NidFrequencyEntry {
    pub nid_hash: String,
    pub name: String,
    pub count: usize,
    pub games: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NidFrequency {
    pub entries: Vec<NidFrequencyEntry>,
    pub total_imports: usize,
    pub unique_nids: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryHeatmap {
    pub libraries: Vec<String>,
    pub games: Vec<String>,
    pub matrix: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub game_nodes: Vec<String>,
    pub lib_nodes: Vec<String>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub weight: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisStats {
    pub total_games: usize,
    pub total_imports: usize,
    pub unique_nids: usize,
    pub unique_libs: usize,
    pub resolution_rate: f64,
    pub most_common_nid: Option<String>,
    pub most_common_nid_name: Option<String>,
    pub most_common_nid_count: usize,
    pub most_used_lib: Option<String>,
    pub most_used_lib_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedEntry {
    pub game: String,
    pub library: String,
    pub nid_hash: String,
}

#[cfg(test)]
pub(crate) fn make_import(nid: &str, resolved: &str, lib_id: u16, lib_name: &str) -> ImportInfo {
    ImportInfo {
        nid_hash: nid.to_string(),
        resolved_name: resolved.to_string(),
        library_id: lib_id,
        library_name: lib_name.to_string(),
    }
}

#[cfg(test)]
pub(crate) fn make_game(name: &str, imports: Vec<ImportInfo>) -> GameAnalysis {
    GameAnalysis {
        name: name.to_string(),
        path: format!("/fake/{}.bin", name),
        sha256: "aabb".to_string(),
        file_size: 1024,
        platform: Platform::Ps5,
        entry_point: 0x80000000,
        is_self: true,
        import_libs: vec![],
        needed_files: vec![],
        num_relocations: 0,
        num_symbols: imports.len(),
        has_tls: false,
        imports,
    }
}

#[cfg(test)]
pub(crate) fn make_db(games: Vec<GameAnalysis>) -> AnalysisDatabase {
    AnalysisDatabase {
        schema_version: 1,
        tool: "ps5rs-test".to_string(),
        games,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_display() {
        assert_eq!(Platform::Ps4.to_string(), "PS4");
        assert_eq!(Platform::Ps5.to_string(), "PS5");
        assert_eq!(Platform::RawElf.to_string(), "RawElf");
        assert_eq!(Platform::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn platform_roundtrip_serde() {
        for p in [Platform::Ps4, Platform::Ps5, Platform::RawElf, Platform::Unknown] {
            let json = serde_json::to_string(&p).unwrap();
            let back: Platform = serde_json::from_str(&json).unwrap();
            assert_eq!(p, back);
        }
    }

    #[test]
    fn analysis_database_serde_roundtrip() {
        let db = make_db(vec![
            make_game("GameA", vec![
                make_import("abc123", "sceKernelLoadStartModule", 1, "libkernel"),
                make_import("def456", "sceDisplaySetFrameBuf", 2, "libSceDisplay"),
            ]),
            make_game("GameB", vec![
                make_import("abc123", "sceKernelLoadStartModule", 1, "libkernel"),
            ]),
        ]);
        let json = serde_json::to_string(&db).unwrap();
        let back: AnalysisDatabase = serde_json::from_str(&json).unwrap();
        assert_eq!(back.schema_version, 1);
        assert_eq!(back.tool, "ps5rs-test");
        assert_eq!(back.games.len(), 2);
        assert_eq!(back.games[0].name, "GameA");
        assert_eq!(back.games[0].imports.len(), 2);
        assert_eq!(back.games[1].name, "GameB");
        assert_eq!(back.games[1].imports[0].nid_hash, "abc123");
    }

    #[test]
    fn import_info_serde_roundtrip() {
        let imp = make_import("abc#libkernel", "sceKernelLoadStartModule", 1, "libkernel");
        let json = serde_json::to_string(&imp).unwrap();
        let back: ImportInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.nid_hash, "abc#libkernel");
        assert_eq!(back.resolved_name, "sceKernelLoadStartModule");
        assert_eq!(back.library_id, 1);
        assert_eq!(back.library_name, "libkernel");
    }

    #[test]
    fn game_analysis_has_all_fields() {
        let g = make_game("TestGame", vec![make_import("x", "y", 1, "lib")]);
        assert_eq!(g.name, "TestGame");
        assert!(g.is_self);
        assert_eq!(g.platform, Platform::Ps5);
        assert_eq!(g.entry_point, 0x80000000);
        assert_eq!(g.file_size, 1024);
        assert_eq!(g.imports.len(), 1);
        assert!(!g.has_tls);
    }

    #[test]
    fn analysis_database_schema_version() {
        let db = make_db(vec![]);
        assert_eq!(db.schema_version, 1);
    }
}
