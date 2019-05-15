#![allow(dead_code)]
#![allow(unused_variables)]

use crate::error::IDLError;
use crate::generator::Generator;
use crate::module::Module;
use crate::pretty_writer::PrettyWriter;
use crate::target::Target;
use crate::types::{
    AliasDataType, AtomType, DataType, DataTypeRef, DataTypeVariant, EnumDataType, FuncDecl, Named,
    StructDataType,
};
use std::io::prelude::*;

/// Generator for the C backend
pub struct CGenerator {
    pub target: Target,
    pub w: PrettyWriter,
}

impl Generator for CGenerator {
    fn gen_type_header(&mut self, _module: &Module, dt: &Named<DataType>) -> Result<(), IDLError> {
        self.w
            .eob()?
            .writeln(format!("// ---------- {} ----------", dt.name.name))?
            .eob()?;
        Ok(())
    }

    // The most important thing in alias generation is to cache the size
    // and alignment rules of what it ultimately points to
    fn gen_alias(
        &mut self,
        module: &Module,
        dt: &Named<DataType>,
        alias: &AliasDataType,
    ) -> Result<(), IDLError> {
        let dtname = self.type_name(dt);
        self.w.indent()?;
        self.w.writeln(format!(
            "typedef {} {};",
            self.type_ref_name(module, &alias.to),
            dtname
        ))?;
        self.w.eob()?;

        // Add an assertion to check that resolved size is the one we computed
        self.w.writeln(format!(
            "_Static_assert(sizeof({}) == {}, \"unexpected alias size\");",
            dtname, dt.entity.repr_size
        ))?;
        self.w.eob()?;

        Ok(())
    }

    fn gen_struct(
        &mut self,
        module: &Module,
        dt: &Named<DataType>,
        struct_: &StructDataType,
    ) -> Result<(), IDLError> {
        let dtname = self.type_name(dt);
        self.w.writeln(format!("{} {{", dtname))?;
        let mut w_block = self.w.new_block();
        for member in struct_.members.iter() {
            w_block.writeln(format!(
                "{} {};",
                self.type_ref_name(module, &member.type_),
                member.name
            ))?;
        }
        self.w.writeln("};")?;
        self.w.eob()?;

        // Add assertions to check that the target platform matches the expected alignment
        // Also add a macro definition for the structure size
        // Skip the first member, as it will always be at the beginning of the structure
        for (i, member) in struct_.members.iter().enumerate().skip(1) {
            self.w.writeln(format!(
                "_Static_assert(offsetof({}, {}) == {}, \"unexpected offset\");",
                dtname, member.name, member.offset
            ))?;
        }

        let struct_size = dt.entity.repr_size;
        self.w.writeln(format!(
            "_Static_assert(sizeof({}) == {}, \"unexpected structure size\");",
            dtname, struct_size,
        ))?;
        self.w.eob()?;

        Ok(())
    }

    // Enums generate both a specific typedef, and a traditional C-style enum
    // The typedef is required to use a native type which is consistent across all architectures
    fn gen_enum(
        &mut self,
        module: &Module,
        dt: &Named<DataType>,
        enum_: &EnumDataType,
    ) -> Result<(), IDLError> {
        let dtname = self.type_name(dt);
        let type_size = dt.entity.repr_size;
        self.w.writeln(format!("{} {{", dtname))?;
        let mut pretty_writer_i1 = self.w.new_block();
        for (i, named_member) in enum_.members.iter().enumerate() {
            pretty_writer_i1.writeln(format!(
                "{}, // {}",
                macro_for(&dt.name.name, &named_member.name),
                i
            ))?;
        }
        self.w.writeln("};")?;
        self.w.eob()?;
        self.w.writeln(format!(
            "_Static_assert(sizeof({}) == {}, \"unexpected enumeration size\");",
            dtname, type_size
        ))?;
        self.w.eob()?;
        Ok(())
    }

    fn gen_function(
        &mut self,
        _module: &Module,
        _func_decl_entry: &Named<FuncDecl>,
    ) -> Result<(), IDLError> {
        // UNIMPLEMENTED!!
        Ok(())
    }
}

impl CGenerator {
    pub fn new(target: Target, w: Box<dyn Write>) -> Self {
        let mut w = PrettyWriter::new(w);
        let prelude = r"
#include <assert.h>
#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>";
        for line in prelude.lines() {
            w.write_line(line.as_ref()).unwrap();
        }
        w.eob().unwrap();
        Self { target, w }
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

    fn type_name(&self, dt: &Named<DataType>) -> String {
        match dt.entity.variant {
            DataTypeVariant::Struct(_) => format!("struct {}", dt.name.name),
            DataTypeVariant::Enum(_) => format!("enum {}", dt.name.name),
            DataTypeVariant::Alias(_) => format!("{}", dt.name.name),
        }
    }

    fn type_ref_name(&self, module: &Module, type_: &DataTypeRef) -> String {
        match type_ {
            DataTypeRef::Atom(a) => atom_type_name(a).to_owned(),
            DataTypeRef::Defined(id) => {
                let dt = &module.get_datatype(*id).expect("type_name of valid ref");
                self.type_name(dt)
            }
        }
    }
}

fn atom_type_name(atom_type: &AtomType) -> &'static str {
    match atom_type {
        AtomType::Bool => "bool",
        AtomType::U8 => "uint8_t",
        AtomType::U16 => "uint16_t",
        AtomType::U32 => "uint32_t",
        AtomType::U64 => "uint64_t",
        AtomType::I8 => "int8_t",
        AtomType::I16 => "int16_t",
        AtomType::I32 => "int32_t",
        AtomType::I64 => "int64_t",
        AtomType::F32 => "float",
        AtomType::F64 => "double",
    }
}

fn macro_for(prefix: &str, name: &str) -> String {
    let mut macro_name = String::new();
    macro_name.push_str(&prefix.to_uppercase());
    macro_name.push('_');
    let mut previous_was_uppercase = name.chars().nth(0).expect("Empty name").is_uppercase();
    for c in name.chars() {
        let is_uppercase = c.is_uppercase();
        if is_uppercase != previous_was_uppercase {
            macro_name.push('_');
        }
        for uc in c.to_uppercase() {
            macro_name.push(uc);
        }
        previous_was_uppercase = is_uppercase;
    }
    macro_name
}
