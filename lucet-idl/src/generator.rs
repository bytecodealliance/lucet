use crate::cache::Cache;
use crate::error::IDLError;
use crate::module::Module;
use crate::pretty_writer::PrettyWriter;
use crate::types::{DataType, DataTypeEntry, Ident};
use std::io::Write;
use std::rc::Rc;

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

    fn gen_accessors_struct(
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
        id: Ident,
    ) -> Result<(), IDLError> {
        let data_type_entry = module.get_datatype(id);
        self.gen_type_header(module, cache, pretty_writer, &data_type_entry)?;
        match &data_type_entry.data_type {
            DataType::Struct { .. } => {
                self.gen_struct(module, cache, pretty_writer, &data_type_entry)
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
        id: Ident,
        hierarchy: &Hierarchy,
    ) -> Result<(), IDLError> {
        let data_type_entry = module.get_datatype(id);
        match &data_type_entry.data_type {
            DataType::Struct { .. } => self.gen_accessors_struct(
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

#[derive(Debug, Clone)]
pub struct HierarchyEntry {
    name: Rc<String>,
    offset: usize,
}

impl HierarchyEntry {
    pub fn new(name: String, offset: usize) -> Self {
        HierarchyEntry {
            name: Rc::new(name),
            offset,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Hierarchy(Vec<HierarchyEntry>);

impl Hierarchy {
    pub fn new(name: String, offset: usize) -> Self {
        Hierarchy(vec![HierarchyEntry::new(name, offset)])
    }

    pub fn push(&self, name: String, offset: usize) -> Self {
        let mut cloned = self.clone();
        cloned.0.push(HierarchyEntry::new(name, offset));
        cloned
    }

    pub fn depth(&self) -> usize {
        self.0.len()
    }

    pub fn idl_name(&self) -> String {
        self.0
            .iter()
            .map(|x| x.name.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn fn_name(&self) -> String {
        self.0
            .iter()
            .map(|x| x.name.as_str())
            .collect::<Vec<_>>()
            .join("_")
    }

    #[allow(dead_code)]
    pub fn parent_name(&self) -> String {
        let len = self.0.len();
        assert!(len > 1);
        self.0
            .iter()
            .take(len - 1)
            .map(|x| x.name.as_str())
            .collect::<Vec<_>>()
            .join("_")
    }

    pub fn root_name(&self) -> String {
        self.0
            .iter()
            .take(1)
            .map(|x| x.name.as_str())
            .collect::<Vec<_>>()
            .join("_")
    }

    pub fn current_offset(&self) -> usize {
        self.0.last().expect("Empty hierarchy").offset
    }
}
