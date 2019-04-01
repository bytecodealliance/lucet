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

pub use lucet_module_data::HeapSpec;

pub fn empty_heap_spec() -> HeapSpec {
    HeapSpec {
        reserved_size: 0,
        guard_size: 0,
        initial_size: 0,
        max_size: None,
    }
}
