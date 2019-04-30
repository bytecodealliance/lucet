#![allow(dead_code)]
#![allow(unused_variables)]

use crate::backend::*;
use crate::cache::*;
use crate::errors::*;
use crate::generator::{Generator, Hierarchy};
use crate::module::{DataTypeEntry, DataTypeRef, Module};
use crate::pretty_writer::*;
use crate::target::*;
use std::io::prelude::*;

#[derive(Clone, Debug)]
struct CTypeInfo<'t> {
    /// The native type name
    type_name: String,
    /// Alignment rules for that type
    type_align: usize,
    /// The native type size
    type_size: usize,
    /// How many pointer indirections are required to get to the atomic type
    indirections: usize,
    /// The leaf type node
    leaf_data_type_ref: &'t DataTypeRef,
}

/// Generator for the C backend
pub struct RustGenerator {
    pub target: Target,
    pub backend_config: BackendConfig,
}

impl<W: Write> Generator<W> for RustGenerator {
    fn gen_prelude(&mut self, pretty_writer: &mut PrettyWriter<W>) -> Result<(), IDLError> {
        unimplemented!()
    }

    fn gen_type_header(
        &mut self,
        _module: &Module,
        _cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }

    // The most important thing in alias generation is to cache the size
    // and alignment rules of what it ultimately points to
    fn gen_alias(
        &mut self,
        module: &Module,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }

    fn gen_struct(
        &mut self,
        module: &Module,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }

    // Enums generate both a specific typedef, and a traditional C-style enum
    // The typedef is required to use a native type which is consistent across all architectures
    fn gen_enum(
        &mut self,
        module: &Module,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }

    fn gen_accessors_struct(
        &mut self,
        module: &Module,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }

    fn gen_accessors_enum(
        &mut self,
        module: &Module,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }

    fn gen_accessors_alias(
        &mut self,
        module: &Module,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }
}
