#![allow(dead_code)]
#![allow(unused_variables)]

use crate::error::IDLError;
use crate::pretty_writer::PrettyWriter;
use crate::{
    AbiType, AliasDatatype, AtomType, Datatype, DatatypeVariant, EnumDatatype, Function, MemArea,
    Module, Package, StructDatatype,
};
use heck::{CamelCase, SnakeCase};
use std::io::Write;

mod guest_funcs;

/// Generator for the Rust backend
pub struct RustGenerator {
    pub w: PrettyWriter,
}

impl RustGenerator {
    pub fn new(w: Box<dyn Write>) -> Self {
        Self {
            w: PrettyWriter::new(w),
        }
    }

    pub fn generate_guest(&mut self, package: &Package) -> Result<(), IDLError> {
        for module in package.modules() {
            self.w
                .writeln(format!("mod {} {{", module.rust_name()))
                .indent();
            self.generate_datatypes(&module)?;

            self.w
                .writeln("mod abi {")
                .indent()
                .writeln(format!("#[link(wasm_import_module=\"{}\")]", module.name()))
                .writeln("extern \"C\" {")
                .indent();
            for f in module.functions() {
                self.guest_abi_import(&f)?;
            }
            self.w.eob().writeln("}").eob().writeln("}");

            /*
            for fdecl in module.func_decls() {
                self.guest_idiomatic_def(module, &fdecl.entity)?;
            }

            */

            self.w.eob().writeln("}");
        }
        Ok(())
    }

    pub fn generate_host(&mut self, package: &Package) -> Result<(), IDLError> {
        for module in package.modules() {
            self.w
                .writeln(format!("mod {} {{", module.rust_name()))
                .indent();
            self.generate_datatypes(&module)?;

            /*
                        self.host_trait_definition(module)?;
                        self.w.eob();

                        self.w
                            .writeln("use lucet_runtime::{lucet_hostcalls, lucet_hostcall_terminate};");
                        self.w.writeln("lucet_hostcalls! {").indent();
                        for fdecl in module.func_decls() {
                            self.host_abi_definition(module, &fdecl.entity)?;
                        }
                        self.w.eob().writeln("}");

                        self.host_ensure_linked(module);
            */
            self.w.eob().writeln("}");
        }
        Ok(())
    }

    fn generate_datatypes(&mut self, module: &Module) -> Result<(), IDLError> {
        for dt in module.datatypes() {
            match dt.variant() {
                DatatypeVariant::Struct(s) => self.gen_struct(&s)?,
                DatatypeVariant::Alias(a) => self.gen_alias(&a)?,
                DatatypeVariant::Enum(e) => self.gen_enum(&e)?,
                DatatypeVariant::Atom { .. } => {}
            }
        }
        Ok(())
    }

    fn gen_alias(&mut self, alias: &AliasDatatype) -> Result<(), IDLError> {
        self.w
            .writeln(format!(
                "pub type {} = {};",
                alias.rust_type_name(),
                alias.to().rust_type_name()
            ))
            .eob();

        gen_testcase(&mut self.w, &alias.name().to_snake_case(), move |w| {
            w.writeln(format!(
                "assert_eq!({}, ::std::mem::size_of::<super::{}>());",
                alias.mem_size(),
                alias.rust_type_name()
            ));
            Ok(())
        })?;
        Ok(())
    }

    fn gen_struct(&mut self, struct_: &StructDatatype) -> Result<(), IDLError> {
        self.w
            .writeln("#[repr(C)]")
            .writeln(format!("pub struct {} {{", struct_.rust_type_name()));

        let mut w = self.w.new_block();
        for m in struct_.members() {
            w.writeln(format!(
                "{}: {},",
                m.name().to_snake_case(),
                m.type_().rust_type_name(),
            ));
        }

        self.w.writeln("}").eob();

        gen_testcase(&mut self.w, &struct_.name().to_snake_case(), |w| {
            w.writeln(format!(
                "assert_eq!({}, ::std::mem::size_of::<super::{}>());",
                struct_.mem_size(),
                struct_.rust_type_name(),
            ));

            for m in struct_.members() {
                w.writeln(format!(
                    "assert_eq!({}, {{ let base = ::std::ptr::null::<super::{}>(); unsafe {{ (&(*base).{}) as *const _ as usize }} }});",
                    m.offset(), struct_.rust_type_name(), m.name(),
                ));
            }
            Ok(())
        })?;
        Ok(())
    }

    // Enums generate both a specific typedef, and a traditional C-style enum
    // The typedef is required to use a native type which is consistent across all architectures
    fn gen_enum(&mut self, enum_: &EnumDatatype) -> Result<(), IDLError> {
        self.w
            .writeln("#[repr(C)]")
            .writeln("#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]")
            .writeln(format!("pub enum {} {{", enum_.rust_type_name()));

        let mut w = self.w.new_block();
        for v in enum_.variants() {
            w.writeln(format!("{},", v.name().to_camel_case()));
        }

        self.w.writeln("}").eob();

        gen_testcase(&mut self.w, &enum_.name().to_snake_case(), |w| {
            w.writeln(format!(
                "assert_eq!({}, ::std::mem::size_of::<super::{}>());",
                enum_.mem_size(),
                enum_.rust_type_name(),
            ));
            Ok(())
        })?;
        Ok(())
    }
    fn guest_abi_import(&mut self, func: &Function) -> Result<(), IDLError> {
        let mut arg_syntax = Vec::new();
        for a in func.args() {
            arg_syntax.push(format!("{}: {}", a.name(), a.type_().rust_type_name()));
        }

        let ret_syntax = func
            .rets()
            .map(|r| r.type_().rust_type_name())
            .rust_return_signature();

        self.w.writeln("#[no_mangle]").writeln(format!(
            "pub fn {}({}) -> {};",
            func.rust_name(),
            arg_syntax.join(", "),
            ret_syntax
        ));

        Ok(())
    }

    /*
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
                        unreachable!("it should not be possible to have an inout value {:?}", io);
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
                        unreachable!("it should not be possible to have an out slice {:?}", o);
                    }
                }
            }

            def.render(&mut self.w, |mut w| abi_call.render(&mut w))?;

            Ok(())
        }

        fn host_abi_definition(&mut self, module: &Module, func: &FuncDecl) -> Result<(), IDLError> {
            let mut args = vec![format!("&mut vmctx")];
            for a in func.args.iter() {
                args.push(format!(
                    "{}: {}",
                    a.name.to_snake_case(),
                    Self::abitype_name(&a.type_)
                ));
            }

            let abi_rettype = if func.rets.len() == 0 {
                "()"
            } else {
                assert_eq!(func.rets.len(), 1);
                Self::abitype_name(&func.rets[0].type_)
            };

            self.w
                .writeln("#[no_mangle]")
                .writeln(format!(
                    "// Wasm func {}::{}",
                    module.module_name, func.field_name
                ))
                .writeln(format!(
                    "pub unsafe extern \"C\" fn {}({},) -> {} {{",
                    func.binding_name,
                    args.join(", "),
                    abi_rettype
                ));

            self.w.indent();

            let typename = module.module_name.to_camel_case();

            self.w.writeln(format!(
                "fn inner(heap: &mut [u8], obj: &mut dyn {}, {}) -> Result<{},()> {{",
                typename,
                func.args
                    .iter()
                    .map(|a| format!(
                        "{}: {}",
                        a.name.to_snake_case(),
                        Self::abitype_name(&a.type_)
                    ))
                    .collect::<Vec<String>>()
                    .join(", "),
                abi_rettype,
            ));
            self.w.indent();
            {
                let (pre, post, trait_args, trait_rets, func_rets) = self.trait_dispatch(module, func);
                self.w.writelns(&pre);
                self.w.writeln(format!(
                    "let {} = obj.{}({})?;",
                    render_tuple(&trait_rets),
                    func.field_name.to_snake_case(),
                    trait_args.join(", ")
                ));
                self.w.writelns(&post);
                self.w.writeln(format!("Ok({})", render_tuple(&func_rets)));
            }
            self.w.eob().writeln("}");

            self.w.writeln(format!(
                "let mut ctx: ::std::cell::RefMut<Box<{typename}>> = vmctx.get_embed_ctx_mut::<Box<{typename}>>();",
                typename = typename
            ));
            self.w.writeln("let mut heap = vmctx.heap_mut();");
            self.w.writeln(format!(
                "match inner(&mut *heap, &mut **ctx, {}) {{ Ok(v) => v, Err(e) => lucet_hostcall_terminate!(\"FIXME\"), }}",
                func.args
                    .iter()
                    .map(|a| a.name.to_snake_case())
                    .collect::<Vec<String>>()
                    .join(", "),
            ));
            self.w.eob().writeln("}");

            Ok(())
        }

        fn trait_dispatch(
            &self,
            module: &Module,
            func: &FuncDecl,
        ) -> (
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
            Vec<String>,
        ) {
            let mut pre = Vec::new();
            let mut post = Vec::new();
            let mut trait_args = Vec::new();
            let mut trait_rets = Vec::new();
            let mut func_rets = Vec::new();

            for input in func.in_bindings.iter() {
                let input_mem = module.get_mem_area(&input.type_);
                match &input.from {
                    BindingRef::Ptr(ptr_pos) => {
                        let ptr = func.get_param(ptr_pos).expect("valid param");
                        pre.push(format!("let {ptr} = {ptr} as usize;", ptr = ptr.name));
                        pre.push(format!(
                            "if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                            ptr = ptr.name,
                            align = input_mem.mem_align(),
                        ));
                        pre.push(format!(
                            "#[allow(non_snake_case)] let {name}___MEM: &[u8] = heap.get({ptr}..({ptr}+{len})).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                            name = input.name,
                            ptr = ptr.name,
                            len = input_mem.mem_size(),
                        ));
                        pre.push(format!(
                            "let {name}: &{typename} = unsafe {{ ({name}___MEM.as_ptr() as *const {typename}).as_ref().unwrap()  }}; // convert pointer in linear memory to ref",
                            name = input.name,
                            typename = self.get_defined_typename(&input.type_),
                        ));
                        trait_args.push(input.name.clone());
                    }
                    BindingRef::Slice(ptr_pos, len_pos) => {
                        let ptr = func.get_param(ptr_pos).expect("valid param");
                        let len = func.get_param(len_pos).expect("valid param");

                        pre.push(format!("let {ptr} = {ptr} as usize;", ptr = ptr.name));
                        pre.push(format!("let {len} = {len} as usize;", len = len.name));
                        pre.push(format!(
                            "if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                            ptr = ptr.name,
                            align = input_mem.mem_align(),
                        ));
                        pre.push(format!(
                            "#[allow(non_snake_case)] let {name}___MEM: &[u8] = heap.get({ptr}..({ptr}+({len}*{elem_len}))).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                            name = input.name,
                            ptr = ptr.name,
                            len = len.name,
                            elem_len = input_mem.mem_size(),
                        ));

                        pre.push(format!(
                            "let {}: &[{}] = unsafe {{ ::std::slice::from_raw_parts({name}___MEM.as_ptr() as *const {typename}, {len}) }};",
                            name = input.name,
                            typename = self.get_defined_typename(&input.type_),
                            len = len.name,
                        ));
                        trait_args.push(input.name.clone());
                    }
                    BindingRef::Value(value_pos) => {
                        let value = func.get_param(value_pos).expect("valid param");
                        pre.push(format!(
                            "let {name}: {typename} = {value} as {typename};",
                            name = input.name,
                            typename = self.get_defined_typename(&input.type_),
                            value = value.name
                        ));
                        trait_args.push(value.name.clone());
                    }
                }
            }
            for io in func.inout_bindings.iter() {
                let io_mem = module.get_mem_area(&io.type_);
                match &io.from {
                    BindingRef::Ptr(ptr_pos) => {
                        let ptr = func.get_param(ptr_pos).expect("valid param");
                        pre.push(format!("let {ptr} = {ptr} as usize;", ptr = ptr.name));
                        pre.push(format!(
                            "if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                            ptr = ptr.name,
                            align = io_mem.mem_align(),
                        ));
                        pre.push(format!(
                            "#[allow(non_snake_case)] let mut {name}___MEM: &mut [u8] = heap.get_mut({ptr}..({ptr}+{len})).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                            name = io.name,
                            ptr = ptr.name,
                            len = io_mem.mem_size(),
                        ));
                        pre.push(format!(
                            "let mut {name}: &mut {typename} = unsafe {{ ({name}___MEM.as_mut_ptr() as *mut {typename}).as_mut().unwrap()  }}; // convert pointer in linear memory to ref",
                            name = io.name,
                            typename = self.get_defined_typename(&io.type_),
                        ));
                        trait_args.push(format!("&mut {}", io.name));
                    }
                    BindingRef::Slice(ptr_pos, len_pos) => {
                        let ptr = func.get_param(ptr_pos).expect("valid param");
                        let len = func.get_param(len_pos).expect("valid param");

                        pre.push(format!("let {ptr} = {ptr} as usize;", ptr = ptr.name));
                        pre.push(format!("let {len} = {len} as usize;", len = len.name));
                        pre.push(format!(
                            "if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                            ptr = ptr.name,
                            align = io_mem.mem_align(),
                        ));
                        pre.push(format!(
                            "#[allow(non_snake_case)] let mut {name}___MEM: &mut [u8] = heap.get_mut({ptr}..({ptr}+({len}*{elem_len}))).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                            name = io.name,
                            ptr = ptr.name,
                            len = len.name,
                            elem_len = io_mem.mem_size(),
                        ));

                        pre.push(format!(
                            "let mut {}: &mut [{}] = unsafe {{ ::std::slice::from_raw_parts_mut({name}___MEM.as_mut_ptr() as *mut {typename}, {len}) }};",
                            name = io.name,
                            typename = self.get_defined_typename(&io.type_),
                            len = len.name,
                        ));

                        trait_args.push(format!("&mut {}", io.name.clone()));
                    }
                    BindingRef::Value { .. } => unreachable!(),
                }
            }
            for out in func.out_bindings.iter() {
                let out_mem = module.get_mem_area(&out.type_);
                match &out.from {
                    BindingRef::Ptr(ptr_pos) => {
                        let ptr = func.get_param(ptr_pos).expect("valid param");

                        pre.push(format!("let {ptr} = {ptr} as usize;", ptr = ptr.name));
                        pre.push(format!(
                            "if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                            ptr = ptr.name,
                            align = out_mem.mem_align(),
                        ));
                        pre.push(format!(
                            "#[allow(non_snake_case)] let mut {name}___MEM: &mut [u8] = heap.get_mut({ptr}..({ptr}+{len})).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                            name = out.name,
                            ptr = ptr.name,
                            len = out_mem.mem_size(),
                        ));
                        pre.push(format!(
                            "let mut {ptr}: &mut {typename} = unsafe {{ ({name}___MEM.as_mut_ptr() as *mut {typename}).as_mut().unwrap()  }}; // convert pointer in linear memory to ref",
                            name = out.name,
                            typename = self.get_defined_typename(&out.type_),
                            ptr = ptr.name,
                        ));

                        trait_rets.push(out.name.clone());
                        post.push(format!(
                            "*{} = {}; // Copy into out-pointer reference",
                            ptr.name, out.name,
                        ));
                    }
                    BindingRef::Value(value_pos) => {
                        let value = func.get_param(value_pos).expect("valid param");
                        trait_rets.push(out.name.clone());
                        post.push(format!(
                            "let {value}: {typename} = {arg} as {typename};",
                            value = value.name,
                            typename = Self::abitype_name(&value.type_),
                            arg = out.name,
                        ));
                        func_rets.push(value.name.clone())
                    }
                    BindingRef::Slice { .. } => unreachable!(),
                }
            }
            (pre, post, trait_args, trait_rets, func_rets)
        }

        fn host_trait_definition(&mut self, module: &Module) -> Result<(), IDLError> {
            self.w
                .writeln(format!(
                    "pub trait {} {{",
                    module.module_name.to_camel_case()
                ))
                .indent();
            for fdecl in module.func_decls() {
                let func_name = fdecl.entity.field_name.to_snake_case();

                let (mut args, rets) = self.trait_idiomatic_params(&fdecl.entity);
                args.insert(0, "&mut self".to_owned());

                self.w.writeln(format!(
                    "fn {}({}) -> {};",
                    func_name,
                    args.join(", "),
                    format!("Result<{},()>", render_tuple(&rets)),
                ));
            }

            self.w.eob().writeln("}");

            Ok(())
        }

        fn trait_idiomatic_params(&self, func: &FuncDecl) -> (Vec<String>, Vec<String>) {
            let mut args = Vec::new();
            for input in func.in_bindings.iter() {
                match &input.from {
                    BindingRef::Ptr { .. } => args.push(format!(
                        "{}: &{}",
                        input.name,
                        self.get_defined_typename(&input.type_)
                    )),
                    BindingRef::Slice { .. } => args.push(format!(
                        "{}: &[{}]",
                        input.name,
                        self.get_defined_typename(&input.type_)
                    )),
                    BindingRef::Value { .. } => args.push(format!(
                        "{}: {}",
                        input.name,
                        self.get_defined_typename(&input.type_)
                    )),
                }
            }
            for io in func.inout_bindings.iter() {
                match &io.from {
                    BindingRef::Ptr { .. } => args.push(format!(
                        "{}: &mut {}",
                        io.name,
                        self.get_defined_typename(&io.type_)
                    )),
                    BindingRef::Slice { .. } => args.push(format!(
                        "{}: &mut [{}]",
                        io.name,
                        self.get_defined_typename(&io.type_)
                    )),
                    BindingRef::Value { .. } => unreachable!(),
                }
            }
            let mut rets = Vec::new();
            for out in func.out_bindings.iter() {
                match &out.from {
                    BindingRef::Ptr { .. } | BindingRef::Value { .. } => {
                        rets.push(format!("{}", self.get_defined_typename(&out.type_)))
                    }
                    BindingRef::Slice { .. } => unreachable!(),
                }
            }
            (args, rets)
        }

        fn host_ensure_linked(&mut self, module: &Module) {
            self.w.writeln("pub fn ensure_linked() {").indent();
            self.w.writeln("unsafe {");
            for fdecl in module.func_decls() {
                self.w.writeln(format!(
                    "::std::ptr::read_volatile({} as *const extern \"C\" fn());",
                    fdecl.entity.binding_name,
                ));
            }
            self.w.eob().writeln("}");
            self.w.eob().writeln("}");
        }
    */
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

pub fn render_tuple(members: &[String]) -> String {
    match members.len() {
        0 => "()".to_owned(),
        1 => members[0].clone(),
        _ => format!("({})", members.join(", ")),
    }
}

trait RustTypeName {
    fn rust_type_name(&self) -> String;
}

impl RustTypeName for AtomType {
    fn rust_type_name(&self) -> String {
        match self {
            AtomType::Bool => "bool",
            AtomType::U8 => "u8",
            AtomType::U16 => "u16",
            AtomType::U32 => "u32",
            AtomType::U64 => "u64",
            AtomType::I8 => "i8",
            AtomType::I16 => "i16",
            AtomType::I32 => "i32",
            AtomType::I64 => "i64",
            AtomType::F32 => "f32",
            AtomType::F64 => "f64",
        }
        .to_owned()
    }
}

impl RustTypeName for AbiType {
    fn rust_type_name(&self) -> String {
        AtomType::from(self.clone()).rust_type_name()
    }
}

impl RustTypeName for Datatype<'_> {
    fn rust_type_name(&self) -> String {
        match self.variant() {
            DatatypeVariant::Struct(_) | DatatypeVariant::Enum(_) | DatatypeVariant::Alias(_) => {
                self.name().to_camel_case()
            }
            DatatypeVariant::Atom(a) => a.rust_type_name(),
        }
    }
}

impl Function<'_> {
    fn rust_name(&self) -> String {
        self.name().to_snake_case()
    }

    pub fn host_func_name(&self) -> String {
        format!(
            "__{}_{}",
            self.module().name().to_snake_case(),
            self.name().to_snake_case()
        )
    }
    /*
        fn rust_idiomatic_args<'a>(&'a self) -> impl Iterator<Item = FuncBinding<'a>> {
            unimplemented!()
            //self.bindings().filter(|b| !binding_is_ret(b))
        }
        fn rust_idiomatic_rets<'a>(&'a self) -> impl Iterator<Item = FuncBinding<'a>> {
            unimplemented!()
            //self.bindings().filter(|b| binding_is_ret(b))
        }
    */
}

impl Module<'_> {
    fn rust_name(&self) -> String {
        self.name().to_snake_case()
    }
}

trait RustReturnSignature {
    fn rust_return_signature(&mut self) -> String;
}

impl<I> RustReturnSignature for I
where
    I: Iterator<Item = String>,
{
    fn rust_return_signature(&mut self) -> String {
        let mut rets = self.collect::<Vec<String>>();
        if rets.len() == 0 {
            "()".to_owned()
        } else if rets.len() == 1 {
            rets.pop().unwrap()
        } else {
            format!("({})", rets.join(", "))
        }
    }
}
