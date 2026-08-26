use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryImageDocument {
    pub schema_version: u32,
    pub tool: String,
    pub image: ps5_image::BinaryImage,
}
