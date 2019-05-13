#![allow(dead_code)]
#![allow(unused_variables)]

mod alias;
mod cache;
mod catom;
mod r#enum;
mod macros;
mod prelude;
mod r#struct;

use self::cache::{Cache, CachedStructMemberEntry, CachedTypeEntry};
pub(crate) use self::catom::CAtom;
use crate::error::IDLError;
use crate::generator::Generator;
use crate::module::Module;
use crate::pretty_writer::PrettyWriter;
use crate::target::Target;
use crate::types::{
    AliasDataType, DataType, DataTypeRef, DataTypeVariant, EnumDataType, FuncDecl, Named,
    StructDataType,
};
use std::io::prelude::*;

#[derive(Clone, Debug)]
struct CTypeInfo<'t> {
    /// The native type name
    type_name: String,
    /// Alignment rules for that type
    type_align: usize,
    /// The native type size
    type_size: usize,
    /// The leaf type node
    leaf_data_type_ref: &'t DataTypeRef,
}

/// Generator for the C backend
pub struct CGenerator {
    pub target: Target,
    pub cache: Cache,
    pub w: PrettyWriter,
}

impl Generator for CGenerator {
    fn gen_type_header(
        &mut self,
        _module: &Module,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError> {
        self.w
            .eob()?
            .write_line(
                format!("// ---------- {} ----------", data_type_entry.name.name).as_bytes(),
            )?
            .eob()?;
        Ok(())
    }

    // The most important thing in alias generation is to cache the size
    // and alignment rules of what it ultimately points to
    fn gen_alias(
        &mut self,
        module: &Module,
        data_type_entry: &Named<DataType>,
        alias: &AliasDataType,
    ) -> Result<(), IDLError> {
        alias::generate(self, module, data_type_entry, alias)
    }

    fn gen_struct(
        &mut self,
        module: &Module,
        data_type_entry: &Named<DataType>,
        struct_: &StructDataType,
    ) -> Result<(), IDLError> {
        r#struct::generate(self, module, data_type_entry, struct_)
    }

    // Enums generate both a specific typedef, and a traditional C-style enum
    // The typedef is required to use a native type which is consistent across all architectures
    fn gen_enum(
        &mut self,
        module: &Module,
        data_type_entry: &Named<DataType>,
        enum_: &EnumDataType,
    ) -> Result<(), IDLError> {
        r#enum::generate(self, module, data_type_entry, enum_)
    }

    fn gen_function(
        &mut self,
        module: &Module,
        func_decl_entry: &Named<FuncDecl>,
    ) -> Result<(), IDLError> {
        unimplemented!();
    }
}

impl CGenerator {
    pub fn new(target: Target, w: Box<dyn Write>) -> Self {
        let mut w = PrettyWriter::new(w);
        prelude::generate(&mut w, target.clone()).expect("write prelude");
        Self {
            target,
            cache: Cache::default(),
            w,
        }
    }
    /// Traverse a `DataTypeRef` chain, and return information
    /// about the leaf node as well as the native type to use
    /// for this data type
    fn type_info<'t>(&self, module: &'t Module, mut type_: &'t DataTypeRef) -> CTypeInfo<'t> {
        let (mut type_align, mut type_size) = (None, None);
        let mut type_name = None;
        loop {
            match &type_ {
                DataTypeRef::Atom(atom_type) => {
                    let native_atom = CAtom::from(*atom_type);
                    type_align = type_align.or_else(|| Some(native_atom.native_type_align));
                    type_size = type_size.or_else(|| Some(native_atom.native_type_size));
                    type_name =
                        type_name.or_else(|| Some(native_atom.native_type_name.to_string()));
                }
                DataTypeRef::Defined(data_type_id) => {
                    let cached = self.cache.load_type(*data_type_id).unwrap();
                    type_align = type_align.or_else(|| Some(cached.type_align));
                    type_size = type_size.or_else(|| Some(cached.type_size));
                    let data_type_entry = module
                        .get_datatype(*data_type_id)
                        .expect("defined datatype");
                    match data_type_entry.entity.variant {
                        DataTypeVariant::Struct { .. } => {
                            type_name = type_name
                                .or_else(|| Some(format!("struct {}", data_type_entry.name.name)))
                        }
                        DataTypeVariant::Enum { .. } => {
                            type_name = type_name.or_else(|| {
                                Some(format!(
                                    "{} /* (enum ___{}) */",
                                    data_type_entry.name.name, data_type_entry.name.name
                                ))
                            })
                        }
                        DataTypeVariant::Alias(ref a) => {
                            type_name =
                                type_name.or_else(|| Some(data_type_entry.name.name.to_string()));
                            type_ = &a.to;
                            continue;
                        }
                    };
                }
            }
            break;
        }
        CTypeInfo {
            type_name: type_name.unwrap(),
            type_align: type_align.unwrap(),
            type_size: type_size.unwrap(),
            leaf_data_type_ref: type_,
        }
    }

    // Return `true` if the type is an atom, an emum, or an alias to one of these
    pub fn is_type_eventually_an_atom_or_enum(&self, module: &Module, type_: &DataTypeRef) -> bool {
        let inner_type = match type_ {
            DataTypeRef::Atom(_) => return true,
            DataTypeRef::Defined(inner_type) => inner_type,
        };
        let inner_data_type_entry = module.get_datatype(*inner_type).expect("defined datatype");
        match inner_data_type_entry.entity.variant {
            DataTypeVariant::Struct { .. } => false,
            DataTypeVariant::Enum { .. } => true,
            DataTypeVariant::Alias(ref a) => self.is_type_eventually_an_atom_or_enum(module, &a.to),
        }
    }

    /// Return the type refererence, with aliases being resolved
    pub fn unalias<'t>(&self, module: &'t Module, type_: &'t DataTypeRef) -> &'t DataTypeRef {
        let inner_type = match type_ {
            DataTypeRef::Atom(_) => return type_,
            DataTypeRef::Defined(inner_type) => inner_type,
        };
        let inner_data_type_entry = module.get_datatype(*inner_type).expect("defined datatype");
        if let DataTypeVariant::Alias(ref a) = inner_data_type_entry.entity.variant {
            self.unalias(module, &a.to)
        } else {
            type_
        }
    }
}
