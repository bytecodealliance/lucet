use super::cache::*;
use super::errors::*;
use super::generators::*;
use super::pretty_writer::*;
use lucet_idl::validate::*;
use std::io::prelude::*;

/// A convenient structure holding a data type, its name and
/// its internal IDL representation
#[derive(Debug, Clone)]
pub struct DataTypeEntry<'t> {
    pub id: DataTypeId,
    pub name: &'t Name,
    pub data_type: &'t DataType,
}

/// Transforms a `DataDescription`
/// We definitely need a better name for it
#[derive(Debug, Clone)]
pub struct DataDescriptionHelper {
    pub data_description: DataDescription,
}

impl DataDescriptionHelper {
    /// Retrieve information about a data type given its identifier
    pub fn get(&self, id: DataTypeId) -> DataTypeEntry<'_> {
        let name = &self.data_description.names[id.0];
        let data_type = &self.data_description.data_types[&id.0];
        DataTypeEntry {
            id,
            name,
            data_type,
        }
    }

    /// Generate native code for a data type whose identifier is `id`
    /// `Generator` is currently an alias for `CGenerator`, but will be turned into
    /// a trait for dynamic dispatch when the first backend gets a reasonably stable API.
    pub fn gen_for_id<W: Write>(
        &self,
        generator: &mut dyn Generator<W>,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        id: DataTypeId,
    ) -> Result<(), IDLError> {
        let data_type_entry = self.get(id);
        self.gen_type_header(generator, cache, pretty_writer, &data_type_entry)?;
        match &data_type_entry.data_type {
            DataType::Struct { .. } => {
                self.gen_struct(generator, cache, pretty_writer, &data_type_entry)
            }
            DataType::TaggedUnion { .. } => {
                self.gen_tagged_union(generator, cache, pretty_writer, &data_type_entry)
            }
            DataType::Alias { .. } => {
                self.gen_alias(generator, cache, pretty_writer, &data_type_entry)
            }
            DataType::Enum { .. } => {
                self.gen_enum(generator, cache, pretty_writer, &data_type_entry)
            }
        }?;
        self.gen_accessors_for_id(
            generator,
            cache,
            pretty_writer,
            id,
            &Hierarchy::new(data_type_entry.name.name.to_string(), 0),
        )?;
        Ok(())
    }

    /// Generate a comment describing the type being defined right after
    fn gen_type_header<W: Write>(
        &self,
        generator: &mut dyn Generator<W>,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        generator.gen_type_header(&self, cache, pretty_writer, data_type_entry)?;
        Ok(())
    }

    fn gen_enum<W: Write>(
        &self,
        generator: &mut dyn Generator<W>,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        generator.gen_enum(&self, cache, pretty_writer, data_type_entry)?;
        Ok(())
    }

    fn gen_struct<W: Write>(
        &self,
        generator: &mut dyn Generator<W>,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        generator.gen_struct(&self, cache, pretty_writer, data_type_entry)?;
        Ok(())
    }

    fn gen_tagged_union<W: Write>(
        &self,
        generator: &mut dyn Generator<W>,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        generator.gen_tagged_union(&self, cache, pretty_writer, data_type_entry)?;
        Ok(())
    }

    fn gen_alias<W: Write>(
        &self,
        generator: &mut dyn Generator<W>,
        cache: &mut Cache,
        pretty_writer: &mut PrettyWriter<W>,
        data_type_entry: &DataTypeEntry<'_>,
    ) -> Result<(), IDLError> {
        generator.gen_alias(&self, cache, pretty_writer, data_type_entry)?;
        Ok(())
    }

    /// Generate accessors for a data type whose identifier is `id`
    /// `hierarchy` is used to derive function names from nested types
    pub fn gen_accessors_for_id<W: Write>(
        &self,
        generator: &mut dyn Generator<W>,
        cache: &Cache,
        pretty_writer: &mut PrettyWriter<W>,
        id: DataTypeId,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        let data_type_entry = self.get(id);
        match &data_type_entry.data_type {
            DataType::Struct { .. } => generator.gen_accessors_struct(
                &self,
                cache,
                pretty_writer,
                &data_type_entry,
                hierarchy,
            ),
            DataType::TaggedUnion { .. } => generator.gen_accessors_tagged_union(
                &self,
                cache,
                pretty_writer,
                &data_type_entry,
                hierarchy,
            ),
            DataType::Alias { .. } => generator.gen_accessors_alias(
                &self,
                cache,
                pretty_writer,
                &data_type_entry,
                hierarchy,
            ),
            DataType::Enum { .. } => generator.gen_accessors_enum(
                &self,
                cache,
                pretty_writer,
                &data_type_entry,
                hierarchy,
            ),
        }?;
        Ok(())
    }
}
