use crate::error::IDLError;

#[derive(Clone, Debug)]
pub struct Config {
    pub backend: Backend,
}

impl Config {
    pub fn parse(backend_opt: &str) -> Result<Self, IDLError> {
        let backend = Backend::from_str(backend_opt)
            .ok_or_else(|| IDLError::UsageError(format!("Invalid backend: {}", backend_opt)))?;
        Ok(Self { backend })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Backend {
    CGuest,
    RustGuest,
    RustHost,
}

impl Backend {
    pub fn from_str<T: AsRef<str>>(s: T) -> Option<Self> {
        match s.as_ref() {
            "c_guest" => Some(Backend::CGuest),
            "rust_guest" => Some(Backend::RustGuest),
            "rust_host" => Some(Backend::RustHost),
            _ => None,
        }
    }
}
