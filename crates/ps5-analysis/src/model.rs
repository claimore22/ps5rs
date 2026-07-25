use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize)]
pub struct ImportInfo {
    pub nid_hash: String,
    pub resolved_name: String,
    pub library_id: u16,
    pub library_name: String,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct LibInfo {
    pub id: u16,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct AnalysisDatabase {
    pub schema_version: u32,
    pub tool: String,
    pub games: Vec<GameAnalysis>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NidFrequencyEntry {
    pub nid_hash: String,
    pub name: String,
    pub count: usize,
    pub games: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NidFrequency {
    pub entries: Vec<NidFrequencyEntry>,
    pub total_imports: usize,
    pub unique_nids: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryHeatmap {
    pub libraries: Vec<String>,
    pub games: Vec<String>,
    pub matrix: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyGraph {
    pub game_nodes: Vec<String>,
    pub lib_nodes: Vec<String>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub weight: usize,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct UnresolvedEntry {
    pub game: String,
    pub library: String,
    pub nid_hash: String,
}
