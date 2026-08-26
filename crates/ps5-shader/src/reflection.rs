use crate::resources::ResourceBinding;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub resources: Vec<ResourceBinding>,
}
