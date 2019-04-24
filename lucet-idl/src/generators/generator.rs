use super::super::cache::*;
use super::super::cgenerator::*;
use super::super::config::*;
use super::super::data_description_helper::*;
use super::super::errors::*;
use super::super::pretty_writer::*;
use super::super::rustgenerator::*;
use super::hierarchy::*;
use std::io::prelude::*;

pub trait Generator<W: Write> {
    fn gen_prelude(&mut self, pretty_writer: &mut PrettyWriter<W>) -> Result<(), IDLError>;

    fn gen_type_header(
        &mut self,
        _data_description_helper: &DataDescriptionHelper,
        _cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError>;

    fn gen_alias(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError>;

    fn gen_struct(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError>;

    fn gen_enum(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError>;

    fn gen_tagged_union(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError>;

    fn gen_accessors_struct(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError>;

    fn gen_accessors_tagged_union(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError>;

    fn gen_accessors_enum(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError>;

    fn gen_accessors_alias(
        &mut self,
        data_description_helper: &DataDescriptionHelper,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError>;
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
