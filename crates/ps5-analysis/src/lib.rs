pub mod artifacts;
pub mod batch_extract;
pub mod collector;
pub mod dataset;
pub mod dragon;
pub mod engine_fingerprints;
pub mod export;
pub mod middleware;
pub mod model;
pub mod names;
pub mod param_json;
pub mod reports;
pub mod scanner;
pub mod shader_inventory;
pub mod string_patterns;
pub mod unity;

pub use batch_extract::{
    BatchExtractOptions, BatchExtractResult, ExtractionEntry, ExtractionManifest, batch_extract,
};
pub use collector::{CollectorOptions, collect};
pub use dataset::{
    AnalysisDataset, DATASET_SCHEMA_VERSION, DatasetError, Manifest, extract_title_id,
};
pub use middleware::{
    GameMiddlewareReport, MiddlewareModule, MiddlewareReport, ModuleKind, build_middleware_report,
    classify_stem, merge_middleware_reports,
};
pub use model::*;
pub use names::{relative_display_path, scrub_scene_tags};
pub use param_json::GameParam;
pub use reports::EngineHintReport;
pub use reports::LibraryVersionReport;
pub use scanner::{ScanOptions, ScanResult, scan};
