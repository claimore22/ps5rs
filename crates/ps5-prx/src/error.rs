use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrxError {
    #[error("ELF parse error: {0}")]
    Elf(String),
    #[error("SELF parse error: {0}")]
    SelfError(String),
    #[error("missing dynamic section")]
    MissingDynamic,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
