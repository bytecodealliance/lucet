#[macro_use]
extern crate failure;

mod lucet_module_data_capnp {
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/lucet_module_data_capnp.rs"));
}

use crate::lucet_module_data_capnp::{heap_spec, module_data, sparse_chunk, sparse_data};
use failure::Error;
use std::io::{self, Write};

pub struct ModuleDataReader<'a> {
    buf: &'a [capnp::Word],
    message: capnp::message::Reader<capnp::serialize::SliceSegments<'static>>,
    module_data: ModuleData<'static>,
}

impl<'a> ModuleDataReader<'a> {
    pub fn new(buf: &'a [capnp::Word]) -> Result<Self, Error> {
        use capnp::message::Reader;
        use capnp::serialize::SliceSegments;
        unsafe {
            let message: Reader<SliceSegments<'static>> = capnp::serialize::read_message_from_words(
                buf,
                capnp::message::ReaderOptions::default(),
            )?;
            let module_data = ModuleData::deserialize(&message as &'static Reader<_>)?;
            Ok(Self {
                buf,
                message,
                module_data,
            })
        }
    }

    pub fn module_data(&self) -> &ModuleData {
        &self.module_data
    }
}

#[derive(Debug)]
pub struct ModuleData<'a> {
    heap_spec: HeapSpec,
    sparse_data: SparseData<'a>,
}

impl<'a> ModuleData<'a> {
    pub fn new(heap_spec: HeapSpec, sparse_data: SparseData<'a>) -> Self {
        Self {
            heap_spec,
            sparse_data,
        }
    }

    pub fn heap_spec(&self) -> &HeapSpec {
        &self.heap_spec
    }

    pub fn sparse_data(&self) -> &SparseData<'a> {
        &self.sparse_data
    }

    pub fn serialize<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let message = self.build();
        capnp::serialize::write_message(w, &message)
    }

    fn build(&self) -> capnp::message::Builder<capnp::message::HeapAllocator> {
        let mut message = capnp::message::Builder::new_default();
        {
            let mut module_data = message.init_root::<module_data::Builder>();
            self.heap_spec
                .build(module_data.reborrow().init_heap_spec());
            self.sparse_data.build(module_data.init_sparse_data());
        }
        message
    }

    pub fn deserialize(
        message: &'a capnp::message::Reader<capnp::serialize::SliceSegments<'a>>,
    ) -> Result<Self, Error> {
        let reader = message.get_root::<module_data::Reader>()?;

        let heap_spec = HeapSpec::read(
            reader
                .get_heap_spec()
                .map_err(|_| format_err!("module_data missing required field heap_spec"))?,
        )?;
        let sparse_data = SparseData::read(
            reader
                .get_sparse_data()
                .map_err(|_| format_err!("module_data missing required field sparse_data"))?,
        )?;
        Ok(Self {
            heap_spec,
            sparse_data,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    fn read<'a>(reader: heap_spec::Reader<'a>) -> Result<Self, Error> {
        let reserved_size = reader.get_reserved_size();
        let guard_size = reader.get_guard_size();
        let initial_size = reader.get_initial_size();
        let max_size = match reader.get_max_size().which() {
            Ok(heap_spec::max_size::MaxSize(ms)) => Some(ms),
            Ok(heap_spec::max_size::None(_)) => None,
            Err(_) => Err(format_err!("max_size missing from heap_spec"))?,
        };
        Ok(Self {
            reserved_size,
            guard_size,
            initial_size,
            max_size,
        })
    }

    fn build<'a>(&self, mut builder: heap_spec::Builder<'a>) {
        builder.set_reserved_size(self.reserved_size);
        builder.set_guard_size(self.guard_size);
        builder.set_initial_size(self.initial_size);
        let mut ms_builder = builder.reborrow().init_max_size();
        match self.max_size {
            Some(max_size) => ms_builder.set_max_size(max_size),
            None => ms_builder.set_none(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SparseData<'a> {
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

    fn read(reader: sparse_data::Reader<'a>) -> Result<Self, Error> {
        let mut chunks = Vec::new();
        for chunk in reader
            .get_chunks()
            .map_err(|_| format_err!("sparse_data missing required field chunks"))?
            .iter()
        {
            match chunk.get_contents().which() {
                Ok(sparse_chunk::contents::Which::Empty(_)) => {
                    chunks.push(None);
                }
                Ok(sparse_chunk::contents::Which::Full(data_reader)) => {
                    let data = data_reader.map_err(|_| format_err!("XXX"))?;
                    if data.len() == 4096 {
                        chunks.push(Some(data));
                    } else {
                        Err(format_err!(
                            "sparse_chunk contents are invalid length: {}",
                            data.len(),
                        ))?
                    }
                }
                Err(_) => Err(format_err!("sparse_chunk missing required field contents"))?,
            }
        }
        Ok(Self { chunks })
    }

    fn build<'b>(&self, builder: sparse_data::Builder<'b>) {
        let mut list_builder = builder.init_chunks(self.chunks.len() as u32);
        for (ix, chunk) in self.chunks.iter().enumerate() {
            let mut contents_builder = list_builder.reborrow().get(ix as u32).init_contents();
            match chunk {
                Some(contents) => {
                    let len: u32 = contents.len() as u32;
                    assert_eq!(len, 4096);
                    contents_builder.init_full(len).copy_from_slice(&contents);
                }
                None => {
                    contents_builder.set_empty(());
                }
            }
        }
    }
}
