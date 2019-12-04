use thiserror::Error;

/// Module data (de)serialization errors.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Sparse data contained a page with length other than 4096")]
    IncorrectPageSize,
    #[error("Deserialization error: {}", _0)]
    DeserializationError(#[source] bincode::Error),
    #[error("Serialization error: {}", _0)]
    SerializationError(#[source] bincode::Error),
    #[error("Module signature error: {}", _0)]
    ModuleSignatureError(#[source] minisign::PError),
    #[error("I/O error: {}", _0)]
    IOError(#[source] std::io::Error),
}
