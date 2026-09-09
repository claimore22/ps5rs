use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub relative_path: String,
    pub file_name: String,
    pub extension: String,
    pub size: u64,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameArtifactsRecord {
    pub game: String,
    pub game_dir: String,
    pub total_files: usize,
    pub total_bytes: u64,
    pub by_extension: HashMap<String, usize>,
    pub by_category: HashMap<String, usize>,
    pub artifacts: Vec<ArtifactRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactReportRecord {
    pub schema_version: u32,
    pub games: Vec<GameArtifactsRecord>,
    pub total_games: usize,
    pub total_files: usize,
    pub by_extension: HashMap<String, usize>,
    pub by_category: HashMap<String, usize>,
}

impl ArtifactReportRecord {
    pub fn new(
        games: Vec<GameArtifactsRecord>,
        total_games: usize,
        total_files: usize,
        by_extension: HashMap<String, usize>,
        by_category: HashMap<String, usize>,
    ) -> Self {
        Self {
            schema_version: crate::SCHEMA_VERSION,
            games,
            total_games,
            total_files,
            by_extension,
            by_category,
        }
    }
}
