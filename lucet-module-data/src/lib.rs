#[macro_use]
extern crate failure;

mod lucet_module_data_capnp {
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/lucet_module_data_capnp.rs"));
}

pub mod reader {

    use crate::lucet_module_data_capnp::{heap_spec, module_data, sparse_chunk, sparse_data};
    use failure::Error;

    pub struct ModuleData<'a> {
        heap_spec: HeapSpec<'a>,
        sparse_data: SparseData<'a>,
    }

    impl<'a> ModuleData<'a> {
        pub fn from_message(
            message: &'a capnp::message::Reader<capnp::serialize::OwnedSegments>,
        ) -> Result<Self, Error> {
            let module_reader = message.get_root::<module_data::Reader>()?;
            Self::read(module_reader)
        }
        fn read(reader: module_data::Reader<'a>) -> Result<Self, Error> {
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
        pub fn heap_spec(&self) -> &HeapSpec<'a> {
            &self.heap_spec
        }
        pub fn sparse_data(&self) -> &SparseData<'a> {
            &self.sparse_data
        }
    }

    pub struct HeapSpec<'a> {
        reader: heap_spec::Reader<'a>,
    }

    impl<'a> HeapSpec<'a> {
        fn read(reader: heap_spec::Reader<'a>) -> Result<Self, Error> {
            match reader.get_max_size().which() {
                Ok(_) => Ok(Self { reader }),
                Err(_) => Err(format_err!("max_size missing from heap_spec")),
            }
        }

        pub fn reserved_size(&self) -> u64 {
            self.reader.get_reserved_size()
        }
        pub fn guard_size(&self) -> u64 {
            self.reader.get_guard_size()
        }
        pub fn initial_size(&self) -> u64 {
            self.reader.get_initial_size()
        }
        pub fn max_size(&self) -> Option<u64> {
            match self.reader.get_max_size().which() {
                Ok(heap_spec::max_size::MaxSize(ms)) => Some(ms),
                Ok(heap_spec::max_size::None(_)) => None,
                _ => unreachable!(),
            }
        }
    }

    pub struct SparseData<'a> {
        chunks: Vec<Option<&'a [u8]>>,
    }
    impl<'a> SparseData<'a> {
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

        pub fn get_chunk(&self, offset: usize) -> &Option<&'a [u8]> {
            self.chunks.get(offset).unwrap_or(&None)
        }
    }
}

pub mod writer {

    use crate::lucet_module_data_capnp::{heap_spec, module_data, sparse_data};
    use failure::Error;

    pub struct ModuleData {
        pub heap_spec: HeapSpec,
        pub sparse_data: SparseData,
    }

    impl ModuleData {
        pub fn new(heap_spec: HeapSpec, sparse_data: SparseData) -> Self {
            Self {
                heap_spec,
                sparse_data,
            }
        }

        pub fn build(&self) -> capnp::message::Builder<capnp::message::HeapAllocator> {
            let mut message = capnp::message::Builder::new_default();
            {
                let mut module_data = message.init_root::<module_data::Builder>();
                self.heap_spec
                    .build(module_data.reborrow().init_heap_spec());
                self.sparse_data.build(module_data.init_sparse_data());
            }
            message
        }
    }

    pub struct HeapSpec {
        pub reserved_size: u64,
        pub guard_size: u64,
        pub initial_size: u64,
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

        pub fn build<'a>(&self, mut builder: heap_spec::Builder<'a>) {
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

    pub struct SparseData {
        chunks: Vec<Option<Vec<u8>>>,
    }

    impl SparseData {
        pub fn new(chunks: Vec<Option<Vec<u8>>>) -> Result<Self, Error> {
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

        pub fn chunks(&self) -> &[Option<Vec<u8>>] {
            &self.chunks
        }

        fn build<'a>(&self, builder: sparse_data::Builder<'a>) {
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
}
