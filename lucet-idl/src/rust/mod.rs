#![allow(dead_code)]
#![allow(unused_variables)]

use crate::error::IDLError;
use crate::generator::Generator;
use crate::module::Module;
use crate::pretty_writer::PrettyWriter;
use crate::target::Target;
use crate::types::AtomType;
use crate::types::{DataType, DataTypeRef, FuncDecl, Ident, Named};
use heck::{CamelCase, SnakeCase};
use std::collections::HashMap;
use std::io::Write;

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
pub struct RustGenerator {
    pub target: Target,
    pub defined: HashMap<Ident, String>,
    pub w: PrettyWriter,
}

impl RustGenerator {
    pub fn new(target: Target, w: Box<dyn Write>) -> Self {
        Self {
            target,
            defined: HashMap::new(),
            w: PrettyWriter::new(w),
        }
    }

    fn define_name(&mut self, data_type_entry: &Named<DataType>) -> String {
        let typename = data_type_entry.name.name.to_camel_case();
        self.defined.insert(data_type_entry.id, typename.clone());
        typename
    }

    fn get_defined_typename(&self, data_type_ref: &DataTypeRef) -> &str {
        match data_type_ref {
            DataTypeRef::Defined(id) => self.defined.get(id).expect("definition exists"),
            DataTypeRef::Atom(a) => Self::atom_name(a),
        }
    }

    fn atom_name(atom_type: &AtomType) -> &'static str {
        use AtomType::*;
        match atom_type {
            Bool => "bool",
            U8 => "u8",
            U16 => "u16",
            U32 => "u32",
            U64 => "u64",
            I8 => "i8",
            I16 => "i16",
            I32 => "i32",
            I64 => "i64",
            F32 => "f32",
            F64 => "f64",
        }
    }
}

impl Generator for RustGenerator {
    fn gen_type_header(
        &mut self,
        _module: &Module,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError> {
        self.w.eob()?.writeln(format!(
            "/// {}: {:?}",
            data_type_entry.name.name, data_type_entry
        ))?;
        Ok(())
    }

    fn gen_alias(
        &mut self,
        module: &Module,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError> {
        let (pointee, _attrs) =
            if let DataType::Alias { to: pointee, attrs } = &data_type_entry.entity {
                (pointee, attrs)
            } else {
                unreachable!()
            };

        let typename = self.define_name(data_type_entry);
        let pointee_name = self.get_defined_typename(pointee);

        self.w
            .writeln(format!("pub type {} = {};", typename, pointee_name))?
            .eob()?;
        Ok(())
    }

    fn gen_struct(
        &mut self,
        module: &Module,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError> {
        let (named_members, _attrs) = if let DataType::Struct {
            members: named_members,
            attrs,
        } = &data_type_entry.entity
        {
            (named_members, attrs)
        } else {
            unreachable!()
        };

        let typename = data_type_entry.name.name.to_camel_case();
        self.defined.insert(data_type_entry.id, typename.clone());

        self.w
            .writeln("#[repr(C)]")?
            .writeln(format!("pub struct {} {{", typename))?;

        let mut w = self.w.new_block();
        for m in named_members {
            w.writeln(format!(
                "{}: {},",
                m.name.to_snake_case(),
                self.get_defined_typename(&m.type_)
            ))?;
        }

        self.w.writeln("}")?.eob()?;
        Ok(())
    }

    // Enums generate both a specific typedef, and a traditional C-style enum
    // The typedef is required to use a native type which is consistent across all architectures
    fn gen_enum(
        &mut self,
        module: &Module,
        data_type_entry: &Named<DataType>,
    ) -> Result<(), IDLError> {
        let (named_members, _attrs) = if let DataType::Enum {
            members: named_members,
            attrs,
        } = &data_type_entry.entity
        {
            (named_members, attrs)
        } else {
            unreachable!()
        };

        let typename = data_type_entry.name.name.to_camel_case();
        self.defined.insert(data_type_entry.id, typename.clone());

        self.w
            .writeln("#[repr(C)]")?
            .writeln("#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]")?
            .writeln(format!("pub enum {} {{", typename))?;

        let mut w = self.w.new_block();
        for m in named_members {
            w.writeln(format!("{},", m.name.to_camel_case()))?;
        }

        self.w.writeln("}")?.eob()?;
        Ok(())
    }

    fn gen_function(
        &mut self,
        module: &Module,
        func_decl_entry: &Named<FuncDecl>,
    ) -> Result<(), IDLError> {
        self.w
            .write_line(format!("// {:?}", func_decl_entry).as_bytes())?;

        let name = func_decl_entry.name.name.to_snake_case();
        let mut args = String::new();
        for a in func_decl_entry.entity.args.iter() {
            args += &format!(
                "{}: {},",
                a.name.to_snake_case(),
                self.get_defined_typename(&a.type_)
            );
        }

        let func_rets = &func_decl_entry.entity.rets;
        let rets = if func_rets.len() == 0 {
            "()".to_owned()
        } else {
            assert_eq!(func_rets.len(), 1);
            self.get_defined_typename(&func_rets[0].type_).to_owned()
        };

        self.w
            .writeln("#[no_mangle]")?
            .writeln(format!("pub fn {}({}) -> {} {{", name, args, rets))?;

        let mut w = self.w.new_block();
        w.writeln("unimplemented!()")?;

        self.w.writeln("}")?.eob()?;

        Ok(())
    }
}
