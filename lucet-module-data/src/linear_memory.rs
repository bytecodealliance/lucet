
use failure::Error;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub fn new(
        reserved_size: u64,
        guard_size: u64,
        initial_size: u64,
        max_size: Option<u64>,
    ) -> Self {
        Self {
            reserved_size,
            guard_size,
            initial_size,
            max_size,
        }
    }

    /// Some very small test programs dont specify a memory import or definition.
    pub fn empty() -> Self {
        Self {
            reserved_size: 0,
            guard_size: 0,
            initial_size: 0,
            max_size: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseData<'a> {
    /// Indices into the vector correspond to the offset, in host page (4k) increments, from the
    /// base of the instance heap.
    ///
    /// If the option at a given index is None, the page is initialized as zeros. Otherwise,
    /// the contents of the page are given as a slice of exactly 4k bytes.
    ///
    /// The deserializer of this datastructure does not make sure the 4k invariant holds,
    /// but the constructor on the serializier side does.
    #[serde(borrow)]
    chunks: Vec<Option<&'a [u8]>>,
}

impl<'a> SparseData<'a> {
    pub fn new(chunks: Vec<Option<&'a [u8]>>) -> Result<Self, Error> {
        for c in &chunks {
            if let Some(chunk) = c {
                if chunk.len() != 4096 {
                    return Err(format_err!(
                        "when creating SparseData, got chunk len {}, only 4096 is allowed",
                        chunk.len()
                    ));
                }
            }
        }
        Ok(Self { chunks })
    }

    pub fn chunks(&self) -> &[Option<&'a [u8]>] {
        &self.chunks
    }

    pub fn get_chunk(&self, offset: usize) -> &Option<&'a [u8]> {
        self.chunks.get(offset).unwrap_or(&None)
    }
}

pub struct OwnedSparseData {
    chunks: Vec<Option<Vec<u8>>>,
}

impl OwnedSparseData {
    pub fn new(chunks: Vec<Option<Vec<u8>>>) -> Result<Self, Error> {
        for c in &chunks {
            if let Some(chunk) = c {
                if chunk.len() != 4096 {
                    return Err(format_err!(
                        "when creating OwnedSparseData, got chunk len {}, only 4096 is allowed",
                        chunk.len()
                    ));
                }
            }
        }
        Ok(Self { chunks })
    }
    pub fn get_ref(&self) -> SparseData {
        SparseData::new(self.chunks.iter().map(|c| match c {
            Some(data) => Some(data.as_slice()),
            None => None,
        }).collect()).expect("sparsedata invariant enforced by ownedsparsedata constructor")
    }
}

