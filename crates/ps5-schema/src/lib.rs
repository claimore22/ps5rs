pub const SCHEMA_VERSION: u32 = 1;

pub mod binary_image;
pub mod dependency_graph;
pub mod game_record;
pub mod migrations;
pub mod module_record;
pub mod nid_record;
pub mod shader_record;

pub use binary_image::BinaryImageDocument;
pub use dependency_graph::DependencyGraphSnapshot;
pub use game_record::GameRecord;
pub use module_record::ModuleRecord;
pub use nid_record::NidRecord;
pub use shader_record::ShaderRecord;
