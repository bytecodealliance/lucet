#![allow(dead_code)]
#![allow(unused_variables)]

use crate::error::IDLError;
use crate::module::Module;
use crate::package::Package;
use crate::pretty_writer::PrettyWriter;
use crate::types::{
    AbiType, AliasDataType, AtomType, BindDirection, BindingRef, DataType, DataTypeRef,
    DataTypeVariant, EnumDataType, FuncDecl, Ident, Named, StructDataType,
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

    pub fn generate_guest(&mut self, package: &Package) -> Result<(), IDLError> {
        for (_ident, module) in package.modules.iter() {
            self.generate_datatypes(module)?;

            for fdecl in module.func_decls() {
                self.guest_idiomatic_def(module, &fdecl.entity)?;
            }

            self.w.writeln("mod abi {")?;
            self.w.indent();
            self.w.writeln(format!(
                "#[link(wasm_import_module=\"{}\")]",
                module.module_name
            ))?;
            self.w.writeln("extern \"C\" {")?;
            self.w.indent();
            for fdecl in module.func_decls() {
                self.guest_abi_import(module, &fdecl.entity)?;
            }
            self.w.eob()?;
            self.w.writeln("}")?;
            self.w.eob()?;
            self.w.writeln("}")?;
        }
        Ok(())
    }

    pub fn generate_host(&mut self, package: &Package) -> Result<(), IDLError> {
        for (_ident, module) in package.modules.iter() {
            self.w.writeln("use lucet_runtime::lucet_hostcalls;")?;
            self.generate_datatypes(module)?;
            self.w.writeln("lucet_hostcalls! {")?;
            self.w.indent();
            for fdecl in module.func_decls() {
                self.host_abi_definition(module, &fdecl.entity)?;
            }
            self.w.eob()?;
            self.w.writeln("}")?;
        }
        Ok(())
    }

    fn generate_datatypes(&mut self, module: &Module) -> Result<(), IDLError> {
        for ref dt in module.datatypes() {
            match &dt.entity.variant {
                DataTypeVariant::Struct(s) => self.gen_struct(module, dt, s)?,
                DataTypeVariant::Alias(a) => self.gen_alias(module, dt, a)?,
                DataTypeVariant::Enum(e) => self.gen_enum(module, dt, e)?,
            }
        }
        Ok(())
    }

    fn define_name(&mut self, dt: &Named<DataType>) -> String {
        let typename = dt.name.name.to_camel_case();
        self.defined.insert(dt.id, typename.clone());
        typename
    }

    fn get_defined_typename(&self, data_type_ref: &DataTypeRef) -> String {
        match data_type_ref {
            DataTypeRef::Defined(id) => self.defined.get(id).expect("definition exists"),
            DataTypeRef::Atom(a) => Self::atom_name(a),
        }
        .to_owned()
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

    fn abitype_name(abi_type: &AbiType) -> &'static str {
        use AbiType::*;
        match abi_type {
            I32 => "i32",
            I64 => "i64",
            F32 => "f32",
            F64 => "f64",
        }
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

        gen_testcase(&mut self.w, &dt.name.name.to_snake_case(), move |w| {
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

    fn guest_abi_import(&mut self, module: &Module, func: &FuncDecl) -> Result<(), IDLError> {
        let mut args = String::new();
        for a in func.args.iter() {
            args += &format!(
                "{}: {},",
                a.name.to_snake_case(),
                Self::abitype_name(&a.type_)
            );
        }

        let rets = if func.rets.len() == 0 {
            "()"
        } else {
            assert_eq!(func.rets.len(), 1);
            Self::abitype_name(&func.rets[0].type_)
        };

        self.w
            .writeln("#[no_mangle]")?
            .writeln(format!("pub fn {}({}) -> {};", func.field_name, args, rets))?;

        Ok(())
    }

    fn guest_idiomatic_def(&mut self, module: &Module, func: &FuncDecl) -> Result<(), IDLError> {
        let mut args = Vec::new();

        let mut before_abi_call: Vec<String> = Vec::new();
        let mut after_abi_call: Vec<String> = Vec::new();

        for input in func.bindings_with(BindDirection::In) {
            match &input.from {
                BindingRef::Ptr(ptr) => {
                    args.push(format!(
                        "{}: &{}",
                        input.name,
                        self.get_defined_typename(&input.type_)
                    ));
                    before_abi_call
                        .push(format!("let {} = {} as *const _ as i32;", ptr, input.name,));
                }
                BindingRef::Slice(ptr, len) => {
                    args.push(format!(
                        "{}: &[{}]",
                        input.name,
                        self.get_defined_typename(&input.type_)
                    ));
                    before_abi_call.push(format!("let {} = {}.as_ptr() as i32;", ptr, input.name,));
                    before_abi_call.push(format!("let {} = {}.len() as i32;", len, input.name,));
                }
                BindingRef::Value(val) => {
                    args.push(format!(
                        "{}: {}",
                        input.name,
                        self.get_defined_typename(&input.type_)
                    ));
                    before_abi_call.push(format!("// TODO: cast {} to abi type", val,));
                }
            }
        }

        for io in func.bindings_with(BindDirection::InOut) {
            match &io.from {
                BindingRef::Ptr(ptr) => {
                    args.push(format!(
                        "{}: &mut {}",
                        io.name,
                        self.get_defined_typename(&io.type_)
                    ));
                    before_abi_call.push(format!(
                        "// TODO: cast the ref to a pointer, and then to u32 named {}: {:?}",
                        ptr, io
                    ));
                }
                BindingRef::Slice(ptr, len) => {
                    args.push(format!(
                        "{}: &mut [{}]",
                        io.name,
                        self.get_defined_typename(&io.type_)
                    ));
                    before_abi_call.push(format!(
                        "// TODO: destructure {} into ptr {} and arg {}",
                        io.name, ptr, len
                    ));
                }
                BindingRef::Value(_val) => {
                    panic!("it should not be possible to have an inout value {:?}", io);
                }
            }

            args.push(format!("/* FIXME inout binding {:?} */", io));
        }

        for input in func.unbound_args() {
            args.push(format!(
                "{}: {}",
                input.name,
                Self::abitype_name(&input.type_)
            ));
        }

        let mut rets = Vec::new();
        for o in func.bindings_with(BindDirection::Out) {
            match &o.from {
                BindingRef::Ptr(ptr) => {
                    rets.push(format!("&'static {}", self.get_defined_typename(&o.type_))); // XXX this should be boxed? need to define allocation protocol like we did in terrarium?
                    after_abi_call.push(format!(
                        "// TODO: cast the u32 named {} ptr, then to a ref, {:?}. FALLIBLE!!",
                        ptr, o
                    ));
                }
                BindingRef::Value(val) => {
                    rets.push(format!("{}", self.get_defined_typename(&o.type_)));
                    after_abi_call.push(format!(
                        "// TODO: cast the ret named {} to a value {:?}",
                        val, o
                    ));
                }
                BindingRef::Slice(_ptr, _len) => {
                    panic!("it should not be possible to have an out slice {:?}", o);
                }
            }
        }
        for o in func.unbound_rets() {
            rets.push(Self::abitype_name(&o.type_).to_owned());
        }

        let name = func.field_name.to_snake_case();
        let arg_syntax = args.join(", ");
        let ret_syntax = if rets.is_empty() {
            "Result<(),()>".to_owned()
        } else {
            assert_eq!(rets.len(), 1);
            format!("Result<{},()>", rets[0])
        };
        self.w.writeln(format!(
            "pub fn {}({}) -> {} {{",
            name, arg_syntax, ret_syntax
        ))?;
        self.w.indent();
        for l in before_abi_call {
            self.w.writeln(l)?;
        }

        {
            // Do the ABI call
            let ret_syntax = if func.rets.is_empty() {
                String::new()
            } else {
                format!("let {} = ", func.rets[0].name)
            };
            let arg_syntax = func
                .args
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<String>>()
                .join(", ");
            self.w
                .writeln(format!("{}abi::{}({});", ret_syntax, name, arg_syntax))?;
        }
        for l in after_abi_call {
            self.w.writeln(l)?;
        }
        if !func.rets.is_empty() {
            self.w
                .writeln(format!("Ok({})", func.rets[0].name.clone()))?;
        }
        self.w.eob()?;
        self.w.writeln("}")?;
        Ok(())
    }

    fn host_abi_definition(&mut self, module: &Module, func: &FuncDecl) -> Result<(), IDLError> {
        let mut args = format!("&mut vmctx,");
        for a in func.args.iter() {
            args += &format!(
                "{}: {},",
                a.name.to_snake_case(),
                Self::abitype_name(&a.type_)
            );
        }

        let rets = if func.rets.len() == 0 {
            "()"
        } else {
            assert_eq!(func.rets.len(), 1);
            Self::abitype_name(&func.rets[0].type_)
        };

        self.w.writeln("#[no_mangle]")?.writeln(format!(
            "// Wasm func {}::{}
pub unsafe extern \"C\" fn {}({}) -> {} {{",
            module.module_name, func.field_name, func.binding_name, args, rets
        ))?;

        self.w.indent();
        self.w.writeln("unimplemented!()")?;
        self.w.eob()?;

        self.w.writeln("}")?;

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
