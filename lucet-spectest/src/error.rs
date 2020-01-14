use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Parse error")]
    ParseError(#[from] wabt::script::Error),
    #[error("Read error")]
    ReadError(#[from] std::io::Error),
    #[error("Run failed with {0} failures")]
    RunError(usize),
    #[error("Unsupported command: {0}")]
    UnsupportedCommand(String),
    #[error("Unexpected success")]
    UnexpectedSuccess,
    #[error("Unexpected failure: {0}")]
    UnexpectedFailure(String),
    #[error("Incorrect result: {0}")]
    IncorrectResult(String),
    #[error("Unsupported by lucetc")]
    UnsupportedLucetc,
}
