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
