#![allow(dead_code)]
#![allow(unused_variables)]

use crate::error::IDLError;
use crate::generator::Generator;
use crate::module::Module;
use crate::pretty_writer::PrettyWriter;
use crate::types::AtomType;
use crate::types::{
    AliasDataType, DataType, DataTypeRef, EnumDataType, FuncDecl, Ident, Named, StructDataType,
};
use heck::{CamelCase, SnakeCase};
use std::collections::HashMap;
use std::io::Write;

/// Generator for the Rust backend
pub struct RustGenerator {
    pub defined: HashMap<Ident, String>,
    pub w: PrettyWriter,
}

impl RustGenerator {
    pub fn new(w: Box<dyn Write>) -> Self {
        Self {
            defined: HashMap::new(),
            w: PrettyWriter::new(w),
        }
    }

    fn define_name(&mut self, dt: &Named<DataType>) -> String {
        let typename = dt.name.name.to_camel_case();
        self.defined.insert(dt.id, typename.clone());
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
    fn gen_type_header(&mut self, _module: &Module, dt: &Named<DataType>) -> Result<(), IDLError> {
        self.w
            .eob()?
            .writeln(format!("/// {}: {:?}", dt.name.name, dt))?;
        Ok(())
    }

    fn gen_alias(
        &mut self,
        module: &Module,
        dt: &Named<DataType>,
        alias: &AliasDataType,
    ) -> Result<(), IDLError> {
        let typename = self.define_name(dt);
        let pointee_name = self.get_defined_typename(&alias.to);

        self.w
            .writeln(format!("pub type {} = {};", typename, pointee_name))?
            .eob()?;

        gen_testcase(&mut self.w, &dt.name.name.to_snake_case(), |w| {
            w.writeln(format!(
                "assert_eq!({}, ::std::mem::size_of::<super::{}>());",
                dt.entity.repr_size, typename
            ))?;
            Ok(())
        })?;

        Ok(())
    }

    fn gen_struct(
        &mut self,
        module: &Module,
        dt: &Named<DataType>,
        struct_: &StructDataType,
    ) -> Result<(), IDLError> {
        let typename = self.define_name(dt);

        self.w
            .writeln("#[repr(C)]")?
            .writeln(format!("pub struct {} {{", typename))?;

        let mut w = self.w.new_block();
        for m in struct_.members.iter() {
            w.writeln(format!(
                "{}: {},",
                m.name.to_snake_case(),
                self.get_defined_typename(&m.type_)
            ))?;
        }

        self.w.writeln("}")?.eob()?;

        gen_testcase(&mut self.w, &dt.name.name.to_snake_case(), |w| {
            w.writeln(format!(
                "assert_eq!({}, ::std::mem::size_of::<super::{}>());",
                dt.entity.repr_size, typename
            ))?;

            for m in struct_.members.iter() {
                w.writeln(format!(
                    "assert_eq!({}, {{ let base = ::std::ptr::null::<super::{}>(); unsafe {{ (&(*base).{}) as *const _ as usize }} }});",
                    m.offset, typename, m.name,
                ))?;
            }
            Ok(())
        })?;

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
        let typename = self.define_name(dt);

        self.w
            .writeln("#[repr(C)]")?
            .writeln("#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]")?
            .writeln(format!("pub enum {} {{", typename))?;

        let mut w = self.w.new_block();
        for m in enum_.members.iter() {
            w.writeln(format!("{},", m.name.to_camel_case()))?;
        }

        self.w.writeln("}")?.eob()?;

        gen_testcase(&mut self.w, &dt.name.name.to_snake_case(), |w| {
            w.writeln(format!(
                "assert_eq!({}, ::std::mem::size_of::<super::{}>());",
                dt.entity.repr_size, typename
            ))?;
            Ok(())
        })?;

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
            .writeln(format!("extern \"C\" fn {}({}) -> {};", name, args, rets))?;

        Ok(())
    }
}

fn gen_testcase<F>(w: &mut PrettyWriter, name: &str, f: F) -> Result<(), IDLError>
where
    F: FnOnce(&mut PrettyWriter) -> Result<(), IDLError>,
{
    w.writeln("#[cfg(test)]")?;
    w.writeln(format!("mod {} {{", name))?;
    let mut ww = w.new_block();
    ww.writeln("#[test]")?;
    ww.writeln("fn test() {")?;
    let mut www = ww.new_block();
    f(&mut www)?;
    ww.writeln("}")?;
    ww.eob()?;
    w.writeln("}")?;
    w.eob()?;
    Ok(())
}
