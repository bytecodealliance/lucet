use crate::cache::Cache;
use crate::config::Config;
use crate::errors::IDLError;
use crate::pretty_writer::PrettyWriter;
use crate::rustgenerator::RustGenerator;
use crate::cgenerator::CGenerator;
use crate::module::{DataTypeEntry, DataType, DataTypeId, Module};
use crate::generators::hierarchy::Hierarchy;
use std::io::prelude::*;

pub trait Generator<W: Write> {
    fn gen_prelude(&mut self, pretty_writer: &mut PrettyWriter<W>) -> Result<(), IDLError>;

    fn gen_type_header(
        &mut self,
        module: &Module,
        _cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError>;

    fn gen_alias(
        &mut self,
        module: &Module,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError>;

    fn gen_struct(
        &mut self,
        module: &Module,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError>;

    fn gen_enum(
        &mut self,
        module: &Module,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError>;

    fn gen_tagged_union(
        &mut self,
        module: &Module,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError>;

    fn gen_accessors_struct(
        &mut self,
        module: &Module,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError>;

    fn gen_accessors_tagged_union(
        &mut self,
        module: &Module,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError>;

    fn gen_accessors_enum(
        &mut self,
        module: &Module,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError>;

    fn gen_accessors_alias(
        &mut self,
        module: &Module,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError>;

    fn gen_for_id(
        &mut self,
        module: &Module,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        id: DataTypeId,
    ) -> Result<(), IDLError> {
        let data_type_entry = module.get_datatype(id);
        self.gen_type_header(module, cache, pretty_writer, &data_type_entry)?;
        match &data_type_entry.data_type {
            DataType::Struct { .. } => {
                self.gen_struct(module, cache, pretty_writer, &data_type_entry)
            }
            DataType::TaggedUnion { .. } => {
                self.gen_tagged_union(module, cache, pretty_writer, &data_type_entry)
            }
            DataType::Alias { .. } => {
                self.gen_alias(module, cache, pretty_writer, &data_type_entry)
            }
            DataType::Enum { .. } => self.gen_enum(module, cache, pretty_writer, &data_type_entry),
        }?;
        self.gen_accessors_for_id(
            module,
            cache,
            pretty_writer,
            id,
            &Hierarchy::new(data_type_entry.name.name.to_string(), 0),
        )?;
        Ok(())
    }

    /// Generate accessors for a data type whose identifier is `id`
    /// `hierarchy` is used to derive function names from nested types
    fn gen_accessors_for_id(
        &mut self,
        module: &Module,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        id: DataTypeId,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        let data_type_entry = module.get_datatype(id);
        match &data_type_entry.data_type {
            DataType::Struct { .. } => {
                self.gen_accessors_struct(module, cache, pretty_writer, &data_type_entry, hierarchy)
            }
            DataType::TaggedUnion { .. } => self.gen_accessors_tagged_union(
                module,
                cache,
                pretty_writer,
                &data_type_entry,
                hierarchy,
            ),
            DataType::Alias { .. } => {
                self.gen_accessors_alias(module, cache, pretty_writer, &data_type_entry, hierarchy)
            }
            DataType::Enum { .. } => {
                self.gen_accessors_enum(module, cache, pretty_writer, &data_type_entry, hierarchy)
            }
        }?;
        Ok(())
    }
}

pub struct Generators;

impl Generators {
    pub fn c(config: &Config) -> CGenerator {
        CGenerator {
            target: config.target,
            backend_config: config.backend_config,
        }
    }

    #[allow(dead_code)]
    pub fn rust(config: &Config) -> RustGenerator {
        RustGenerator {
            target: config.target,
            backend_config: config.backend_config,
        }
    }
}
