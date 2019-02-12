use crate::{
    globals::GlobalSpec,
    linear_memory::{HeapSpec, SparseData},
    lucet_module_data_capnp::module_data,
};

use capnp::{
    message,
    serialize::{read_message_from_words, write_message, SliceSegments},
    Word,
};
use failure::Error;
use std::io::{self, Write};
use std::mem;
use std::ops::Deref;
use serde::{Serialize, Deserialize};

/// Encapsulates details about reading ModuleData out of a serialized message.
pub struct ModuleDataBox<'a> {
    /// Capnproto requires we keep a message::Reader alive for as long as module_data.
    _message: message::Reader<SliceSegments<'static>>,
    /// The user only cares about ModuleData
    module_data: ModuleData<'a>,
}

impl<'a> ModuleDataBox<'a> {
    pub fn deserialize(buf: &'a [Word]) -> Result<ModuleDataBox<'a>, Error> {
        unsafe {
            let message: message::Reader<SliceSegments<'a>> =
                read_message_from_words(buf, message::ReaderOptions::default())?;
            let message: message::Reader<SliceSegments<'static>> = mem::transmute(message);
            let module_data = ModuleData::deserialize(&message)?;
            let module_data: ModuleData<'a> = mem::transmute(module_data);
            Ok(Self {
                _message: message,
                module_data,
            })
        }
    }

    pub fn data(&self) -> &ModuleData {
        &self.module_data
    }
}

impl<'a> Deref for ModuleDataBox<'a> {
    type Target = ModuleData<'a>;
    fn deref(&self) -> &ModuleData<'a> {
        &self.module_data
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleData<'a> {
    heap_spec: HeapSpec,
    #[serde(borrow)]
    sparse_data: SparseData<'a>,
    #[serde(borrow)]
    globals_spec: Vec<GlobalSpec<'a>>,
}

impl<'a> ModuleData<'a> {
    pub fn new(
        heap_spec: HeapSpec,
        sparse_data: SparseData<'a>,
        globals_spec: Vec<GlobalSpec<'a>>,
    ) -> Self {
        Self {
            heap_spec,
            sparse_data,
            globals_spec,
        }
    }

    pub fn heap_spec(&self) -> &HeapSpec {
        &self.heap_spec
    }

    pub fn sparse_data(&self) -> &SparseData<'a> {
        &self.sparse_data
    }

    pub fn globals_spec(&self) -> &[GlobalSpec<'a>] {
        &self.globals_spec
    }

    pub fn serialize<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let message = self.build();
        write_message(w, &message)
    }

    fn build(&self) -> message::Builder<message::HeapAllocator> {
        let mut message = message::Builder::new_default();
        {
            let mut module_data = message.init_root::<module_data::Builder>();
            self.heap_spec
                .build(module_data.reborrow().init_heap_spec());
            self.sparse_data.build(module_data.init_sparse_data());
        }
        message
    }

    pub fn deserialize(
        message: &'a message::Reader<SliceSegments<'static>>,
    ) -> Result<Self, Error> {
        let reader = message.get_root::<module_data::Reader>()?;

        let heap_spec =
            HeapSpec::read(reader.get_heap_spec().map_err(|e| {
                format_err!("module_data missing required field heap_spec: {}", e)
            })?)?;
        let sparse_data =
            SparseData::read(reader.get_sparse_data().map_err(|e| {
                format_err!("module_data missing required field sparse_data: {}", e)
            })?)?;

        let mut globals_spec = Vec::new();
        for gs in reader
            .get_globals_spec()
            .map_err(|e| format_err!("module_data missing required field globals_spec: {}", e))?
            .iter()
        {
            globals_spec.push(GlobalSpec::read(gs)?)
        }

        Ok(Self::new(heap_spec, sparse_data, globals_spec))
    }
}
