#![allow(dead_code)]
#![allow(unused_variables)]

use crate::backend::BackendConfig;
use crate::cache::Cache;
use crate::errors::IDLError;
use crate::generator::{Generator, Hierarchy};
use crate::module::{DataTypeEntry, DataTypeRef, DataType, Module};
use crate::pretty_writer::PrettyWriter;
use crate::target::Target;
use std::io::Write;
use heck::CamelCase;

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
    fn gen_prelude(&mut self, _pretty_writer: &mut PrettyWriter<W>) -> Result<(), IDLError> {
        Ok(())
    }

    fn gen_type_header(
        &mut self,
        _module: &Module,
        _cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        pretty_writer
            .eob()?
            .write_line(
                format!("/// {}: {:?}", data_type_entry.name.name, data_type_entry).as_bytes(),
            )?;
        Ok(())
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
        let (pointee, _attrs) = if let DataType::Alias { to: pointee, attrs } = &data_type_entry.data_type {
            (pointee, attrs)
        } else {
            unreachable!()
        };
        pretty_writer
            .write_line(
                format!("type {} = {:?};", data_type_entry.name.name.to_camel_case(), pointee).as_bytes(),
            )?
            .eob()?;
        Ok(())
    }

    fn gen_struct(
        &mut self,
        module: &Module,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        pretty_writer
            .write_line(
                "#[repr(C)]".as_bytes()
            )?
            .write_line(
                format!("struct {} {{", data_type_entry.name.name.to_camel_case()).as_bytes(),
            )?
            .write_line(
                "}".as_bytes(),
            )?
            .eob()?;
        Ok(())
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
        let (named_members, _attrs) = if let DataType::Enum {
            members: named_members,
            attrs,
        } = &data_type_entry.data_type
        {
            (named_members, attrs)
        } else {
            unreachable!()
        };


        pretty_writer
            .write_line(
                "#[repr(C)]".as_bytes()
            )?
            .write_line(
                "#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]".as_bytes()
            )?
            .write_line(
                format!("enum {} {{", data_type_entry.name.name.to_camel_case()).as_bytes(),
            )?;

        for m in named_members {
            pretty_writer.write_line(format!("    {},", m.name.to_camel_case()).as_bytes())?;
        }

        pretty_writer
            .write_line(
                "}".as_bytes(),
            )?
            .eob()?;
        Ok(())
    }

    fn gen_accessors_struct(
        &mut self,
        _module: &Module,
        _cache: &Cache,
        _pretty_writer: &mut PrettyWriter<W>,
        _data_type_entry: &DataTypeEntry<'_>,
        _hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        Ok(())
    }

    fn gen_accessors_enum(
        &mut self,
        _module: &Module,
        _cache: &Cache,
        _pretty_writer: &mut PrettyWriter<W>,
        _data_type_entry: &DataTypeEntry<'_>,
        _hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        Ok(())
    }

    fn gen_accessors_alias(
        &mut self,
        _module: &Module,
        _cache: &Cache,
        _pretty_writer: &mut PrettyWriter<W>,
        _data_type_entry: &DataTypeEntry<'_>,
        _hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        Ok(())
    }
}
