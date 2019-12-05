use thiserror::Error;

/// Module data (de)serialization errors.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Sparse data contained a page with length other than 4096")]
    IncorrectPageSize,
    #[error("Deserialization error")]
    DeserializationError(#[source] bincode::Error),
    #[error("Serialization error")]
    SerializationError(#[source] bincode::Error),
    #[error("Module signature error")]
    ModuleSignatureError(#[source] minisign::PError),
    #[error("I/O error")]
    IOError(#[source] std::io::Error),
}
