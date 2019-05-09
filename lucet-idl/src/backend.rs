#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Backend {
    C,
    Rust,
}

impl Default for Backend {
    fn default() -> Self {
        Backend::C
    }
}

impl<T: AsRef<str>> From<T> for Backend {
    fn from(s: T) -> Self {
        match s.as_ref() {
            "c" => Backend::C,
            "rust" => Backend::Rust,
            _ => Backend::default(),
        }
    }
}
