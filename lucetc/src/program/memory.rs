#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemorySpec {
    pub initial_pages: u32,
    pub max_pages: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeapSettings {
    pub reserved_size: u64,
    pub guard_size: u64,
}

impl Default for HeapSettings {
    fn default() -> Self {
        Self {
            reserved_size: 4 * 1024 * 1024,
            guard_size: 4 * 1024 * 1024,
        }
    }
}

pub use lucet_module_data::HeapSpec;
pub fn create_heap_spec(mem: &MemorySpec, heap: &HeapSettings) -> HeapSpec {
    let wasm_page: u64 = 64 * 1024;

    let initial_size = mem.initial_pages as u64 * wasm_page;
    // Find the max size permitted by the heap and the memory spec
    let max_size = mem.max_pages.map(|pages| pages as u64 * wasm_page);
    HeapSpec {
        reserved_size: heap.reserved_size,
        guard_size: heap.guard_size,
        initial_size: initial_size,
        max_size: max_size,
    }
}
pub fn empty_heap_spec() -> HeapSpec {
    HeapSpec {
        reserved_size: 0,
        guard_size: 0,
        initial_size: 0,
        max_size: None,
    }
}
