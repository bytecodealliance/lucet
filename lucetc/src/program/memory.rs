use failure::{bail, Error};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemorySpec {
    pub initial_pages: u32,
    pub max_pages: Option<u32>,
}

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
pub fn create_heap_spec(mem: &MemorySpec, heap: &HeapSettings) -> Result<HeapSpec, Error> {
    let wasm_page: u64 = 64 * 1024;

    let initial_size = mem.initial_pages as u64 * wasm_page;

    let reserved_size = std::cmp::max(initial_size, heap.min_reserved_size);

    if reserved_size > heap.max_reserved_size {
        bail!(
            "module reserved size ({}) exceeds max reserved size ({})",
            initial_size,
            heap.max_reserved_size,
        );
    }

    // Find the max size permitted by the heap and the memory spec
    let max_size = mem.max_pages.map(|pages| pages as u64 * wasm_page);
    Ok(HeapSpec {
        reserved_size,
        guard_size: heap.guard_size,
        initial_size: initial_size,
        max_size: max_size,
    })
}
pub fn empty_heap_spec() -> HeapSpec {
    HeapSpec {
        reserved_size: 0,
        guard_size: 0,
        initial_size: 0,
        max_size: None,
    }
}
