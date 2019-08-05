#![allow(dead_code)]
#![allow(unused_variables)]

use crate::error::IDLError;
use crate::pretty_writer::PrettyWriter;
use crate::{
    AbiType, AliasDatatype, AtomType, BindingDirection, BindingParam, Datatype, DatatypeVariant,
    EnumDatatype, Function, MemArea, Module, Package, StructDatatype,
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

            for f in module.functions() {
                self.guest_idiomatic_def(&f)?;
            }

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

            self.host_trait_definition(&module)?;
            self.w.eob();

            self.w
                .writeln("use lucet_runtime::{lucet_hostcalls, lucet_hostcall_terminate};");
            self.w.writeln("lucet_hostcalls! {").indent();
            for func in module.functions() {
                self.host_abi_definition(&func)?;
            }
            self.w.eob().writeln("}");

            self.host_ensure_linked(&module);

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
            .rust_tuple_syntax();

        self.w.writeln("#[no_mangle]").writeln(format!(
            "pub fn {}({}) -> {};",
            func.rust_name(),
            arg_syntax.join(", "),
            ret_syntax
        ));

        Ok(())
    }

    fn guest_idiomatic_def(&mut self, func: &Function) -> Result<(), IDLError> {
        use guest_funcs::{AbiCallBuilder, FuncBuilder};

        let name = func.rust_name();
        let mut def = FuncBuilder::new(name, "()".to_owned());
        let mut abi_call = AbiCallBuilder::new(func.clone());

        for input in func
            .bindings()
            .filter(|b| b.direction() == BindingDirection::In)
        {
            match &input.param() {
                BindingParam::Ptr(ptr) => {
                    def.arg(format!(
                        "{}: &{}",
                        input.name(),
                        input.type_().rust_type_name(),
                    ));
                    abi_call.before(format!(
                        "let {} = {} as *const _ as i32;",
                        ptr.name(),
                        input.name(),
                    ));
                }
                BindingParam::Slice(ptr, len) => {
                    def.arg(format!(
                        "{}: &[{}]",
                        input.name(),
                        input.type_().rust_type_name(),
                    ));

                    abi_call.before(format!(
                        "let {} = {}.as_ptr() as i32;",
                        ptr.name(),
                        input.name(),
                    ));
                    abi_call.before(format!(
                        "let {} = {}.len() as i32;",
                        len.name(),
                        input.name()
                    ));
                }
                BindingParam::Value(val) => {
                    def.arg(format!(
                        "{}: {}",
                        input.name(),
                        input.type_().rust_type_name(),
                    ));
                    abi_call.before(format!(
                        "let {} = {} as {};",
                        val.name(),
                        input.name(),
                        val.type_().rust_type_name(),
                    ));
                }
            }
        }

        for io in func
            .bindings()
            .filter(|b| b.direction() == BindingDirection::InOut)
        {
            match &io.param() {
                BindingParam::Ptr(ptr) => {
                    def.arg(format!(
                        "{}: &mut {}",
                        io.name(),
                        io.type_().rust_type_name(),
                    ));
                    abi_call.before(format!(
                        "let {} = {} as *mut _ as i32;",
                        ptr.name(),
                        io.name()
                    ));
                }
                BindingParam::Slice(ptr, len) => {
                    def.arg(format!(
                        "{}: &mut [{}]",
                        io.name(),
                        io.type_().rust_type_name(),
                    ));
                    abi_call.before(format!(
                        "let {} = {}.as_ptr() as i32;",
                        ptr.name(),
                        io.name()
                    ));
                    abi_call.before(format!("let {} = {}.len() as i32;", len.name(), io.name()));
                }
                BindingParam::Value(_val) => {
                    unreachable!("it should not be possible to have an inout value {:?}", io);
                }
            }
        }

        for o in func
            .bindings()
            .filter(|b| b.direction() == BindingDirection::Out)
        {
            match &o.param() {
                BindingParam::Ptr(ptr) => {
                    def.ok_type(o.type_().rust_type_name());
                    abi_call.before(format!(
                        "let mut {}___MEM = ::std::mem::MaybeUninit::<{}>::uninit();",
                        ptr.name(),
                        o.type_().rust_type_name()
                    ));
                    abi_call.before(format!(
                        "let {} = {}___MEM.as_mut_ptr() as i32;",
                        ptr.name(),
                        ptr.name()
                    ));
                    abi_call.after(format!(
                        "let {} = unsafe {{ {}___MEM.assume_init() }};",
                        o.name(),
                        ptr.name()
                    ));
                    def.ok_value(o.name().to_owned());
                }
                BindingParam::Value(val) => {
                    def.ok_type(o.type_().rust_type_name());
                    abi_call.after(format!(
                        "let {} = {}::from({});",
                        o.name(),
                        o.type_().rust_type_name(),
                        val.name(),
                    ));
                    def.ok_value(o.name().to_owned());
                }
                BindingParam::Slice(_ptr, _len) => {
                    unreachable!("it should not be possible to have an out slice {:?}", o);
                }
            }
        }

        def.render(&mut self.w, |mut w| abi_call.render(&mut w))?;

        Ok(())
    }

    fn host_abi_definition(&mut self, func: &Function) -> Result<(), IDLError> {
        let mut args = vec![format!("&mut vmctx")];
        for a in func.args() {
            args.push(format!(
                "{}: {}",
                a.name().to_snake_case(),
                a.type_().rust_type_name(),
            ));
        }

        let abi_rettype = func
            .rets()
            .map(|r| r.type_().rust_type_name())
            .rust_tuple_syntax();

        self.w
            .writeln("#[no_mangle]")
            .writeln(format!(
                "// Wasm func {}::{}",
                func.module().name(),
                func.name()
            ))
            .writeln(format!(
                "pub unsafe extern \"C\" fn {}({},) -> {} {{",
                func.host_func_name(),
                args.join(", "),
                abi_rettype
            ));

        self.w.indent();

        let trait_type_name = func.module().name().to_camel_case();

        self.w.writeln(format!(
            "fn inner(heap: &mut [u8], obj: &mut dyn {}, {}) -> Result<{},()> {{",
            trait_type_name,
            func.args()
                .map(|a| format!("{}: {}", a.name(), a.type_().rust_type_name(),))
                .collect::<Vec<String>>()
                .join(", "),
            abi_rettype,
        ));
        self.w.indent();
        {
            let (pre, post, trait_args, trait_rets, func_rets) = self.trait_dispatch(func);
            self.w.writelns(&pre);
            self.w.writeln(format!(
                "let {} = obj.{}({})?;",
                render_tuple(&trait_rets),
                func.rust_name(),
                trait_args.join(", ")
            ));
            self.w.writelns(&post);
            self.w.writeln(format!("Ok({})", render_tuple(&func_rets)));
        }
        self.w.eob().writeln("}");

        self.w.writeln(format!(
                "let mut ctx: ::std::cell::RefMut<Box<{typename}>> = vmctx.get_embed_ctx_mut::<Box<{typename}>>();",
                typename =trait_type_name
            ));
        self.w.writeln("let mut heap = vmctx.heap_mut();");
        self.w.writeln(format!(
                "match inner(&mut *heap, &mut **ctx, {}) {{ Ok(v) => v, Err(e) => lucet_hostcall_terminate!(\"FIXME\"), }}",
                func.args()
                    .map(|a| a.name().to_owned())
                    .collect::<Vec<String>>()
                    .join(", "),
            ));
        self.w.eob().writeln("}");

        Ok(())
    }

    fn trait_dispatch(
        &self,
        func: &Function,
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

        for input in func
            .bindings()
            .filter(|b| b.direction() == BindingDirection::In)
        {
            match input.param() {
                BindingParam::Ptr(ptr) => {
                    pre.push(format!("let {ptr} = {ptr} as usize;", ptr = ptr.name()));
                    pre.push(format!(
                        "if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                        ptr = ptr.name(),
                        align = input.type_().mem_align(),
                    ));
                    pre.push(format!(
                            "#[allow(non_snake_case)] let {name}___MEM: &[u8] = heap.get({ptr}..({ptr}+{len})).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                            name = input.name(),
                            ptr = ptr.name(),
                            len = input.type_().mem_size(),
                        ));
                    pre.push(format!(
                            "let {name}: &{typename} = unsafe {{ ({name}___MEM.as_ptr() as *const {typename}).as_ref().unwrap()  }}; // convert pointer in linear memory to ref",
                            name = input.name(),
                            typename = input.type_().rust_type_name(),
                        ));
                    trait_args.push(input.name().to_owned());
                }
                BindingParam::Slice(ptr, len) => {
                    pre.push(format!("let {ptr} = {ptr} as usize;", ptr = ptr.name()));
                    pre.push(format!("let {len} = {len} as usize;", len = len.name()));
                    pre.push(format!(
                        "if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                        ptr = ptr.name(),
                        align = input.type_().mem_align(),
                    ));
                    pre.push(format!(
                            "#[allow(non_snake_case)] let {name}___MEM: &[u8] = heap.get({ptr}..({ptr}+({len}*{elem_len}))).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                            name = input.name(),
                            ptr = ptr.name(),
                            len = len.name(),
                            elem_len = input.type_().mem_size(),
                        ));

                    pre.push(format!(
                            "let {}: &[{}] = unsafe {{ ::std::slice::from_raw_parts({name}___MEM.as_ptr() as *const {typename}, {len}) }};",
                            name = input.name(),
                            typename = input.type_().rust_type_name(),
                            len = len.name(),
                        ));
                    trait_args.push(input.name().to_owned());
                }
                BindingParam::Value(value) => {
                    pre.push(format!(
                        "let {name}: {typename} = {value} as {typename};",
                        name = input.name(),
                        typename = input.type_().rust_type_name(),
                        value = value.name(),
                    ));
                    trait_args.push(value.name().to_owned());
                }
            }
        }
        for io in func
            .bindings()
            .filter(|b| b.direction() == BindingDirection::InOut)
        {
            match io.param() {
                BindingParam::Ptr(ptr) => {
                    pre.push(format!("let {ptr} = {ptr} as usize;", ptr = ptr.name()));
                    pre.push(format!(
                        "if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                        ptr = ptr.name(),
                        align = io.type_().mem_align(),
                    ));
                    pre.push(format!(
                            "#[allow(non_snake_case)] let mut {name}___MEM: &mut [u8] = heap.get_mut({ptr}..({ptr}+{len})).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                            name = io.name(),
                            ptr = ptr.name(),
                            len = io.type_().mem_size(),
                        ));
                    pre.push(format!(
                            "let mut {name}: &mut {typename} = unsafe {{ ({name}___MEM.as_mut_ptr() as *mut {typename}).as_mut().unwrap()  }}; // convert pointer in linear memory to ref",
                            name = io.name(),
                            typename = io.type_().rust_type_name(),
                        ));
                    trait_args.push(format!("&mut {}", io.name()));
                }
                BindingParam::Slice(ptr, len) => {
                    pre.push(format!("let {ptr} = {ptr} as usize;", ptr = ptr.name()));
                    pre.push(format!("let {len} = {len} as usize;", len = len.name()));
                    pre.push(format!(
                        "if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                        ptr = ptr.name(),
                        align = io.type_().mem_align(),
                    ));
                    pre.push(format!(
                            "#[allow(non_snake_case)] let mut {name}___MEM: &mut [u8] = heap.get_mut({ptr}..({ptr}+({len}*{elem_len}))).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                            name = io.name(),
                            ptr = ptr.name(),
                            len = len.name(),
                            elem_len = io.type_().mem_size(),
                        ));

                    pre.push(format!(
                            "let mut {}: &mut [{}] = unsafe {{ ::std::slice::from_raw_parts_mut({name}___MEM.as_mut_ptr() as *mut {typename}, {len}) }};",
                            name = io.name(),
                            typename = io.type_().rust_type_name(),
                            len = len.name(),
                        ));

                    trait_args.push(format!("&mut {}", io.name()));
                }
                BindingParam::Value { .. } => unreachable!(),
            }
        }
        for out in func
            .bindings()
            .filter(|b| b.direction() == BindingDirection::Out)
        {
            match out.param() {
                BindingParam::Ptr(ptr) => {
                    pre.push(format!(
                        "if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                        ptr = ptr.name(),
                        align = out.type_().mem_align(),
                    ));
                    pre.push(format!(
                            "#[allow(non_snake_case)] let mut {name}___MEM: &mut [u8] = heap.get_mut({ptr}..({ptr}+{len})).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                            name = out.name(),
                            ptr = ptr.name(),
                            len = out.type_().mem_size(),
                        ));
                    pre.push(format!(
                            "let mut {ptr}: &mut {typename} = unsafe {{ ({name}___MEM.as_mut_ptr() as *mut {typename}).as_mut().unwrap()  }}; // convert pointer in linear memory to ref",
                            name = out.name(),
                            typename = out.type_().rust_type_name(),
                            ptr = ptr.name(),
                        ));

                    trait_rets.push(out.name().to_owned());
                    post.push(format!(
                        "*{} = {}; // Copy into out-pointer reference",
                        ptr.name(),
                        out.name(),
                    ));
                }
                BindingParam::Value(value) => {
                    trait_rets.push(out.name().to_owned());
                    post.push(format!(
                        "let {value}: {typename} = {arg} as {typename};",
                        value = value.name(),
                        typename = value.type_().rust_type_name(),
                        arg = out.name(),
                    ));
                    func_rets.push(value.name().to_owned())
                }
                BindingParam::Slice { .. } => unreachable!(),
            }
        }
        (pre, post, trait_args, trait_rets, func_rets)
    }

    fn host_trait_definition(&mut self, module: &Module) -> Result<(), IDLError> {
        self.w
            .writeln(format!("pub trait {} {{", module.name().to_camel_case()))
            .indent();
        for func in module.functions() {
            let (mut args, rets) = self.trait_idiomatic_params(&func);
            args.insert(0, "&mut self".to_owned());

            self.w.writeln(format!(
                "fn {}({}) -> {};",
                func.rust_name(),
                args.join(", "),
                format!("Result<{},()>", render_tuple(&rets)),
            ));
        }

        self.w.eob().writeln("}");

        Ok(())
    }

    fn trait_idiomatic_params(&self, func: &Function) -> (Vec<String>, Vec<String>) {
        let mut args = Vec::new();
        for input in func
            .bindings()
            .filter(|b| b.direction() == BindingDirection::In)
        {
            match &input.param() {
                BindingParam::Ptr { .. } => args.push(format!(
                    "{}: &{}",
                    input.name(),
                    input.type_().rust_type_name(),
                )),
                BindingParam::Slice { .. } => args.push(format!(
                    "{}: &[{}]",
                    input.name(),
                    input.type_().rust_type_name(),
                )),
                BindingParam::Value { .. } => args.push(format!(
                    "{}: {}",
                    input.name(),
                    input.type_().rust_type_name(),
                )),
            }
        }
        for io in func
            .bindings()
            .filter(|b| b.direction() == BindingDirection::InOut)
        {
            match &io.param() {
                BindingParam::Ptr { .. } => args.push(format!(
                    "{}: &mut {}",
                    io.name(),
                    io.type_().rust_type_name(),
                )),
                BindingParam::Slice { .. } => args.push(format!(
                    "{}: &mut [{}]",
                    io.name(),
                    io.type_().rust_type_name(),
                )),
                BindingParam::Value { .. } => unreachable!(),
            }
        }
        let mut rets = Vec::new();
        for out in func
            .bindings()
            .filter(|b| b.direction() == BindingDirection::Out)
        {
            match &out.param() {
                BindingParam::Ptr { .. } | BindingParam::Value { .. } => {
                    rets.push(format!("{}", out.type_().rust_type_name()))
                }
                BindingParam::Slice { .. } => unreachable!(),
            }
        }
        (args, rets)
    }

    fn host_ensure_linked(&mut self, module: &Module) {
        self.w.writeln("pub fn ensure_linked() {").indent();
        self.w.writeln("unsafe {");
        for func in module.functions() {
            self.w.writeln(format!(
                "::std::ptr::read_volatile({} as *const extern \"C\" fn());",
                func.host_func_name(),
            ));
        }
        self.w.eob().writeln("}");
        self.w.eob().writeln("}");
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
}

impl Module<'_> {
    fn rust_name(&self) -> String {
        self.name().to_snake_case()
    }
}

pub fn render_tuple(members: &[String]) -> String {
    match members.len() {
        0 => "()".to_owned(),
        1 => members[0].clone(),
        _ => format!("({})", members.join(", ")),
    }
}

pub trait RustTupleSyntax {
    fn rust_tuple_syntax(&mut self) -> String;
}

impl<I> RustTupleSyntax for I
where
    I: Iterator<Item = String>,
{
    fn rust_tuple_syntax(&mut self) -> String {
        let rets = self.collect::<Vec<String>>();
        render_tuple(&rets)
    }
}
