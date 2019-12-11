use thiserror::Error;

/// Module data (de)serialization errors.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Deserialization error")]
    DeserializationError(#[source] bincode::Error),
    #[error("I/O error")]
    IOError(#[source] std::io::Error),
    #[error("Sparse data contained a page with length other than 4096")]
    IncorrectPageSize,
    #[error("Module signature error")]
    ModuleSignatureError(#[source] minisign::PError),
    #[error("Parse error at {key}::{value:?}")]
    ParseError { key: String, value: String },
    #[error("Parse json error")]
    ParseJsonError(#[from] serde_json::error::Error),
    #[error("Top-level json must be an object")]
    ParseJsonObjError,
    #[error("Parse string error")]
    ParseStringError(#[from] std::io::Error),
    #[error("Cannot re-bind {key} from {binding} to {attempt}")]
    RebindError {
        key: String,
        binding: String,
        attempt: String,
    },
    #[error("Serialization error")]
    SerializationError(#[source] bincode::Error),
    #[error("Unknown module for symbol `{module}::{symbol}")]
    UnknownModule { module: String, symbol: String },
    #[error("Unknown symbol `{module}::{symbol}`")]
    UnknownSymbol { module: String, symbol: String },
}
