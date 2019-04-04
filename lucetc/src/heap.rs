#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeapSettings {
    pub min_reserved_size: u64,
    pub max_reserved_size: u64,
    pub guard_size: u64,
}

impl Default for HeapSettings {
    fn default() -> Self {
        Self {
            min_reserved_size: 4 * 1024 * 1024,
            max_reserved_size: 6 * 1024 * 1024 * 1024,
            guard_size: 4 * 1024 * 1024,
        }
    }
}
