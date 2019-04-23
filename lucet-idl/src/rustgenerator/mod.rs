#![allow(dead_code)]
#![allow(unused_variables)]

use super::backend::*;
use super::cache::*;
use super::data_description_helper::*;
use super::errors::*;
use super::generators::*;
use super::pretty_writer::*;
use super::target::*;
use lucet_idl::validate::*;
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
        _data_description_helper: &DataDescriptionHelper,
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
        data_description_helper: &DataDescriptionHelper,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }

    fn gen_struct(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
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
        data_description_helper: &DataDescriptionHelper,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }

    fn gen_tagged_union(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }

    fn gen_accessors_struct(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }

    fn gen_accessors_tagged_union(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }

    fn gen_accessors_enum(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }

    fn gen_accessors_alias(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        unimplemented!()
    }
}
