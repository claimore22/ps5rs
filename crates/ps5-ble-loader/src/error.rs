use ps5_memory_safe::MemoryError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BleError {
    #[error("memory error: {0}")]
    Memory(String),
    #[error("loader error: {0}")]
    Loader(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unimplemented: {0}")]
    Unimplemented(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("not found: {0}")]
    NotFound(String),
}

impl From<MemoryError> for BleError {
    fn from(e: MemoryError) -> Self {
        BleError::Memory(e.to_string())
    }
}

pub type BleResult<T> = Result<T, BleError>;
