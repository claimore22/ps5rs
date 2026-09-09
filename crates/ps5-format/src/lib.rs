pub mod elf_constants;
pub mod error;
pub mod evidence;
pub mod hash;
pub mod self_constants;

pub use error::ParseError;
pub use error::Result;
pub use evidence::{Detection, Evidence, EvidenceKind};
pub use hash::sha256_hex;
