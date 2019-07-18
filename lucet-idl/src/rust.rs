#![allow(dead_code)]
#![allow(unused_variables)]

use crate::error::IDLError;
use crate::function::{BindingRef, FuncDecl};
use crate::module::Module;
use crate::package::Package;
use crate::pretty_writer::PrettyWriter;
use crate::types::{
    AbiType, AliasDataType, AtomType, DataType, DataTypeRef, DataTypeVariant, EnumDataType, Ident,
    Named, StructDataType,
};
use heck::{CamelCase, SnakeCase};
use std::collections::HashMap;
use std::io::Write;

mod guest_funcs;

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

            self.w
                .writeln("mod abi {")
                .indent()
                .writeln(format!(
                    "#[link(wasm_import_module=\"{}\")]",
                    module.module_name
                ))
                .writeln("extern \"C\" {")
                .indent();
            for fdecl in module.func_decls() {
                self.guest_abi_import(module, &fdecl.entity)?;
            }
            self.w.eob().writeln("}").eob().writeln("}");
        }
        Ok(())
    }

    pub fn generate_host(&mut self, package: &Package) -> Result<(), IDLError> {
        for (_ident, module) in package.modules.iter() {
            self.w.writeln("use lucet_runtime::lucet_hostcalls;");
            self.generate_datatypes(module)?;
            self.w.writeln("lucet_hostcalls! {");
            self.w.indent();
            for fdecl in module.func_decls() {
                self.host_abi_definition(module, &fdecl.entity)?;
            }
            self.w.eob();
            self.w.writeln("}");
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
            .writeln(format!("pub type {} = {};", typename, pointee_name))
            .eob();

        gen_testcase(&mut self.w, &dt.name.name.to_snake_case(), move |w| {
            w.writeln(format!(
                "assert_eq!({}, ::std::mem::size_of::<super::{}>());",
                dt.entity.repr_size, typename
            ));
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
            .writeln("#[repr(C)]")
            .writeln(format!("pub struct {} {{", typename));

        let mut w = self.w.new_block();
        for m in struct_.members.iter() {
            w.writeln(format!(
                "{}: {},",
                m.name.to_snake_case(),
                self.get_defined_typename(&m.type_)
            ));
        }

        self.w.writeln("}").eob();

        gen_testcase(&mut self.w, &dt.name.name.to_snake_case(), |w| {
            w.writeln(format!(
                "assert_eq!({}, ::std::mem::size_of::<super::{}>());",
                dt.entity.repr_size, typename
            ));

            for m in struct_.members.iter() {
                w.writeln(format!(
                    "assert_eq!({}, {{ let base = ::std::ptr::null::<super::{}>(); unsafe {{ (&(*base).{}) as *const _ as usize }} }});",
                    m.offset, typename, m.name,
                ));
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
            .writeln("#[repr(C)]")
            .writeln("#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]")
            .writeln(format!("pub enum {} {{", typename));

        let mut w = self.w.new_block();
        for m in enum_.members.iter() {
            w.writeln(format!("{},", m.name.to_camel_case()));
        }

        self.w.writeln("}").eob();

        gen_testcase(&mut self.w, &dt.name.name.to_snake_case(), |w| {
            w.writeln(format!(
                "assert_eq!({}, ::std::mem::size_of::<super::{}>());",
                dt.entity.repr_size, typename
            ));
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
            .writeln("#[no_mangle]")
            .writeln(format!("pub fn {}({}) -> {};", func.field_name, args, rets));

        Ok(())
    }

    fn guest_idiomatic_def(&mut self, module: &Module, func: &FuncDecl) -> Result<(), IDLError> {
        use guest_funcs::{AbiCallBuilder, FuncBuilder};

        let name = func.field_name.to_snake_case();
        let mut def = FuncBuilder::new(name, "()".to_owned());
        let mut abi_call = AbiCallBuilder::new(func);

        for input in func.in_bindings.iter() {
            match &input.from {
                BindingRef::Ptr(ptr_pos) => {
                    let ptr = func.get_param(ptr_pos).expect("valid param");
                    def.arg(format!(
                        "{}: &{}",
                        input.name,
                        self.get_defined_typename(&input.type_)
                    ));
                    abi_call.before(format!(
                        "let {} = {} as *const _ as i32;",
                        ptr.name, input.name,
                    ));
                    abi_call.param(ptr_pos, ptr.name.clone());
                }
                BindingRef::Slice(ptr_pos, len_pos) => {
                    let ptr = func.get_param(ptr_pos).expect("valid param");
                    let len = func.get_param(len_pos).expect("lenid param");
                    def.arg(format!(
                        "{}: &[{}]",
                        input.name,
                        self.get_defined_typename(&input.type_)
                    ));

                    abi_call.before(format!(
                        "let {} = {}.as_ptr() as i32;",
                        ptr.name, input.name,
                    ));
                    abi_call.param(ptr_pos, ptr.name.clone());

                    abi_call.before(format!("let {} = {}.len() as i32;", len.name, input.name,));
                    abi_call.param(len_pos, len.name.clone());
                }
                BindingRef::Value(val_pos) => {
                    let val = func.get_param(val_pos).expect("valid param");
                    def.arg(format!(
                        "{}: {}",
                        input.name,
                        self.get_defined_typename(&input.type_)
                    ));
                    abi_call.before(format!(
                        "let {} = {} as {};",
                        val.name,
                        input.name,
                        Self::abitype_name(&val.type_)
                    ));
                    abi_call.param(val_pos, val.name.clone());
                }
            }
        }

        for io in func.inout_bindings.iter() {
            match &io.from {
                BindingRef::Ptr(ptr_pos) => {
                    let ptr = func.get_param(ptr_pos).expect("valid param");
                    def.arg(format!(
                        "{}: &mut {}",
                        io.name,
                        self.get_defined_typename(&io.type_)
                    ));
                    abi_call.before(format!("let {} = {} as *mut _ as i32;", ptr.name, io.name));
                    abi_call.param(ptr_pos, ptr.name.clone());
                }
                BindingRef::Slice(ptr_pos, len_pos) => {
                    let ptr = func.get_param(ptr_pos).expect("ptrid param");
                    let len = func.get_param(len_pos).expect("lenid param");
                    def.arg(format!(
                        "{}: &mut [{}]",
                        io.name,
                        self.get_defined_typename(&io.type_)
                    ));
                    abi_call.before(format!("let {} = {}.as_ptr() as i32;", ptr.name, io.name));
                    abi_call.before(format!("let {} = {}.len() as i32;", len.name, io.name));
                    abi_call.param(ptr_pos, ptr.name.clone());
                    abi_call.param(len_pos, len.name.clone());
                }
                BindingRef::Value(_val) => {
                    panic!("it should not be possible to have an inout value {:?}", io);
                }
            }
        }

        for o in func.out_bindings.iter() {
            match &o.from {
                BindingRef::Ptr(ptr_pos) => {
                    let ptr = func.get_param(ptr_pos).expect("valid param");
                    let otypename = self.get_defined_typename(&o.type_);
                    def.ok_type(otypename.clone());
                    abi_call.before(format!(
                        "let mut {} = ::std::mem::MaybeUninit::<{}>::uninit();",
                        ptr.name, otypename
                    ));
                    abi_call.param(ptr_pos, format!("{}.as_mut_ptr() as i32", ptr.name));
                    abi_call.after(format!(
                        "let {} = unsafe {{ {}.assume_init() }};",
                        ptr.name, ptr.name
                    ));
                    def.ok_value(ptr.name.clone());
                }
                BindingRef::Value(val_pos) => {
                    let val = func.get_param(val_pos).expect("valid param");
                    def.ok_type(self.get_defined_typename(&o.type_));
                    abi_call.param(val_pos, val.name.clone());
                    abi_call.after(format!(
                        "let {} = {} as {};",
                        val.name,
                        val.name,
                        self.get_defined_typename(&o.type_),
                    ));
                    def.ok_value(val.name.clone());
                }
                BindingRef::Slice(_ptr, _len) => {
                    panic!("it should not be possible to have an out slice {:?}", o);
                }
            }
        }

        def.render(&mut self.w, |mut w| abi_call.render(&mut w))?;

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

        self.w.writeln("#[no_mangle]").writeln(format!(
            "// Wasm func {}::{}
pub unsafe extern \"C\" fn {}({}) -> {} {{",
            module.module_name, func.field_name, func.binding_name, args, rets
        ));

        self.w.indent();
        self.w.writeln("unimplemented!()");

        self.w.eob().writeln("}");

        Ok(())
    }
}

fn gen_testcase<F>(w: &mut PrettyWriter, name: &str, f: F) -> Result<(), IDLError>
where
    F: FnOnce(&mut PrettyWriter) -> Result<(), IDLError>,
{
    w.writeln("#[cfg(test)]")
        .writeln(format!("mod {} {{", name));
    let mut ww = w.new_block();
    ww.writeln("#[test]").writeln("fn test() {");
    let mut www = ww.new_block();
    f(&mut www)?;
    ww.writeln("}").eob();
    w.writeln("}").eob();
    Ok(())
}
