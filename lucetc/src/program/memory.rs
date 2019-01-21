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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeapSpec {
    /// Total bytes of memory for the heap to possibly expand into, as told to Cretonne.  All of
    /// this memory is addressable. Only some part of it is accessible - from 0 to the initial
    /// size, guaranteed, and up to the max_size.  This size allows Cretonne to elide checks of the
    /// *base pointer*. At the moment that just means checking if it is greater than 4gb, in which
    /// case it can elide the base pointer check completely. In the future, Cretonne could use a
    /// solver to elide more base pointer checks if it can prove the calculation will always be
    /// less than this bound.
    pub reserved_size: u64,
    /// Total bytes of memory *after* the reserved area, as told to Cretonne. All of this memory is
    /// addressable, but it is never accessible - it is guaranteed to trap if an access happens in
    /// this region. This size allows Cretonne to use *common subexpression elimination* to reduce
    /// checks of the *sum of base pointer and offset* (where the offset is always rounded up to a
    /// multiple of the guard size, to be friendly to CSE).
    pub guard_size: u64,
    /// Total bytes of memory for the WebAssembly program's linear memory, on initialization.
    pub initial_size: u64,
    /// Total bytes of memory for the WebAssembly program's linear memory, at any time. This is not
    /// necessarily the same as reserved_size - we want to be able to tune the check bound there
    /// separately than the declaration of a max size in the client program. The program may
    /// optionally define this value. If it does, it must be less than the reserved_size. If it
    /// does not, the max size is left up to the runtime, and is allowed to be less than
    /// reserved_size.
    pub max_size: Option<u64>,
}

impl HeapSpec {
    pub fn new(mem: &MemorySpec, heap: &HeapSettings) -> Self {
        let wasm_page: u64 = 64 * 1024;

        let initial_size = mem.initial_pages as u64 * wasm_page;
        // Find the max size permitted by the heap and the memory spec
        let max_size = mem.max_pages.map(|pages| pages as u64 * wasm_page);
        Self {
            reserved_size: heap.reserved_size,
            guard_size: heap.guard_size,
            initial_size: initial_size,
            max_size: max_size,
        }
    }
    // Some very small test programs dont specify a memory import or definition.
    pub fn empty() -> Self {
        Self {
            reserved_size: 0,
            guard_size: 0,
            initial_size: 0,
            max_size: None,
        }
    }
}
