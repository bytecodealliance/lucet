use crate::{
    AbiType, AtomType, BindingDirection, BindingParam, Datatype, DatatypeVariant, EnumVariant,
    FuncBinding, FuncParam, Function, MemArea, Module, StructMember,
};
use heck::{CamelCase, SnakeCase};

pub trait RustTupleSyntax {
    fn rust_tuple_syntax(&mut self, base_case: &str) -> String;
}

impl<I> RustTupleSyntax for I
where
    I: Iterator<Item = String>,
{
    fn rust_tuple_syntax(&mut self, base_case: &str) -> String {
        let rets = self.collect::<Vec<String>>();
        render_tuple(&rets, base_case)
    }
}

pub trait RustTypeName {
    fn rust_type_name(&self) -> String;
}

impl RustTypeName for Module<'_> {
    fn rust_type_name(&self) -> String {
        self.name().to_camel_case()
    }
}

impl RustTypeName for AtomType {
    fn rust_type_name(&self) -> String {
        format!("{}", self)
    }
}

impl RustTypeName for AbiType {
    fn rust_type_name(&self) -> String {
        format!("{}", self)
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

pub trait RustName {
    fn rust_name(&self) -> String;
}

impl RustName for Function<'_> {
    fn rust_name(&self) -> String {
        self.name().to_snake_case()
    }
}

impl RustName for Module<'_> {
    fn rust_name(&self) -> String {
        self.name().to_snake_case()
    }
}

impl RustName for EnumVariant<'_> {
    fn rust_name(&self) -> String {
        self.name().to_camel_case()
    }
}

impl RustName for StructMember<'_> {
    fn rust_name(&self) -> String {
        self.name().to_snake_case()
    }
}

impl RustName for FuncParam<'_> {
    fn rust_name(&self) -> String {
        self.name().to_snake_case()
    }
}

impl RustName for FuncBinding<'_> {
    fn rust_name(&self) -> String {
        self.name().to_snake_case()
    }
}

pub fn render_tuple(members: &[String], base_case: &str) -> String {
    match members.len() {
        0 => base_case.to_owned(),
        1 => members[0].clone(),
        _ => format!("({})", members.join(", ")),
    }
}

pub trait RustFunc<'a> {
    fn rust_idiom_args(&self) -> Vec<RustIdiomArg<'a>>;
    fn rust_idiom_rets(&self) -> Vec<RustIdiomRet<'a>>;
    fn host_func_name(&self) -> String;
}

impl<'a> RustFunc<'a> for Function<'a> {
    fn rust_idiom_args(&self) -> Vec<RustIdiomArg<'a>> {
        self.bindings()
            .filter(|b| b.direction() == BindingDirection::In)
            .chain(
                self.bindings()
                    .filter(|b| b.direction() == BindingDirection::InOut),
            )
            .map(|b| RustIdiomArg { binding: b })
            .collect()
    }

    fn rust_idiom_rets(&self) -> Vec<RustIdiomRet<'a>> {
        self.bindings()
            .filter(|b| b.direction() == BindingDirection::Out)
            .map(|binding| RustIdiomRet { binding })
            .collect()
    }

    fn host_func_name(&self) -> String {
        format!(
            "__{}_{}",
            self.module().name().to_snake_case(),
            self.name().to_snake_case()
        )
    }
}

pub struct RustIdiomArg<'a> {
    binding: FuncBinding<'a>,
}

impl<'a> RustIdiomArg<'a> {
    pub fn name(&self) -> String {
        self.binding.rust_name()
    }
    pub fn direction(&self) -> BindingDirection {
        self.binding.direction()
    }
    pub fn type_(&self) -> Datatype<'a> {
        self.binding.type_()
    }
    pub fn type_name(&self) -> String {
        self.type_().rust_type_name()
    }
    pub fn param(&self) -> BindingParam<'a> {
        self.binding.param()
    }

    fn mutable(&self) -> &'static str {
        if self.binding.direction() == BindingDirection::InOut {
            "mut "
        } else {
            ""
        }
    }

    pub fn arg_declaration(&self) -> String {
        match self.binding.param() {
            BindingParam::Ptr { .. } => {
                format!("{}: &{}{}", self.name(), self.mutable(), self.type_name())
            }
            BindingParam::Slice { .. } => {
                format!("{}: &{}[{}]", self.name(), self.mutable(), self.type_name())
            }
            BindingParam::Value { .. } => {
                assert_eq!(self.binding.direction(), BindingDirection::In);
                format!("{}: {}", self.name(), self.type_name())
            }
        }
    }

    pub fn arg_value(&self) -> String {
        match self.binding.param() {
            BindingParam::Ptr { .. } | BindingParam::Slice { .. } => {
                format!("&{}{}", self.mutable(), self.name(),)
            }
            BindingParam::Value { .. } => {
                assert_eq!(self.binding.direction(), BindingDirection::In);
                self.name()
            }
        }
    }

    pub fn guest_abi_args(&self) -> Vec<String> {
        match self.binding.param() {
            BindingParam::Ptr(ptr) => vec![format!(
                "let {} = {} as *const _ as i32;",
                ptr.rust_name(),
                self.name(),
            )],
            BindingParam::Slice(ptr, len) => vec![
                format!("let {} = {}.as_ptr() as i32;", ptr.rust_name(), self.name()),
                format!("let {} = {}.len() as i32;", len.rust_name(), self.name()),
            ],
            BindingParam::Value(val) => match val.type_().variant() {
                DatatypeVariant::Atom(AtomType::Bool) => {
                    vec![format!("let {} = {} != 0;", val.rust_name(), self.name(),)]
                }
                _ => vec![format!(
                    "let {} = {} as {};",
                    val.rust_name(),
                    self.name(),
                    val.type_().rust_type_name()
                )],
            },
        }
    }

    pub fn host_unpack_to_abi(&self) -> Vec<String> {
        if self.binding.direction() == BindingDirection::In {
            match self.binding.param() {
                BindingParam::Ptr(ptr) => vec![
                    format!("let {ptr} = {ptr} as usize;",
                        ptr = ptr.rust_name()
                    ),
                    format!("if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                        ptr = ptr.rust_name(),
                        align = self.binding.type_().mem_align(),
                    ),
                    format!("#[allow(non_snake_case)] let {name}___MEM: &[u8] = heap.get({ptr}..({ptr}+{len})).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                        name = self.name(),
                        ptr = ptr.rust_name(),
                        len = self.binding.type_().mem_size(),
                    ),
                    format!("let {name}: &{typename} = unsafe {{ ({name}___MEM.as_ptr() as *const {typename}).as_ref().expect(\"determined to be valid ref\")  }}; // convert pointer in linear memory to ref",
                        name = self.name(),
                        typename = self.type_name(),
                    ),
                ],
                BindingParam::Slice(ptr, len) => vec![
                    format!("let {ptr} = {ptr} as usize;",
                        ptr = ptr.rust_name()
                    ),
                    format!("let {len} = {len} as usize;",
                        len = len.rust_name()
                    ),
                    format!("if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                        ptr = ptr.rust_name(),
                        align = self.binding.type_().mem_align(),
                    ),
                    format!("#[allow(non_snake_case)] let {name}___MEM: &[u8] = heap.get({ptr}..({ptr}+({len}*{elem_len}))).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                        name = self.name(),
                        ptr = ptr.rust_name(),
                        len = len.rust_name(),
                        elem_len =self.binding.type_().mem_size(),
                    ),
                    format!("let {name}: &[{typename}] = unsafe {{ ::std::slice::from_raw_parts({name}___MEM.as_ptr() as *const {typename}, {len}) }};",
                        name = self.name(),
                        typename = self.type_name(),
                        len = len.rust_name(),
                    )
                ],
                BindingParam::Value(_val) => vec![cast_value_to_binding(&self.binding)],
            }
        } else {
            assert_eq!(self.binding.direction(), BindingDirection::InOut);
            match self.binding.param() {
                BindingParam::Ptr(ptr) => vec![
                    format!("let {ptr} = {ptr} as usize;",
                        ptr = ptr.rust_name()
                    ),
                    format!("if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                        ptr = ptr.rust_name(),
                        align = self.binding.type_().mem_align(),
                    ),
                    format!("#[allow(non_snake_case)] let mut {name}___MEM: &mut [u8] = heap.get_mut({ptr}..({ptr}+{len})).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                        name = self.name(),
                        ptr = ptr.rust_name(),
                        len = self.binding.type_().mem_size(),
                    ),
                    format!("let mut {name}: &mut {typename} = unsafe {{ ({name}___MEM.as_mut_ptr() as *mut {typename}).as_mut().expect(\"determined to be valid ref\")  }}; // convert pointer in linear memory to ref",
                        name = self.name(),
                        typename = self.type_name(),
                    ),
                ],
                BindingParam::Slice(ptr, len) => vec![
                    format!("let {ptr} = {ptr} as usize;",
                        ptr = ptr.rust_name()
                    ),
                    format!("let {len} = {len} as usize;",
                        len = len.rust_name()
                    ),
                    format!("if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                        ptr = ptr.rust_name(),
                        align = self.binding.type_().mem_align(),
                    ),
                    format!("#[allow(non_snake_case)] let mut {name}___MEM: &mut [u8] = heap.get_mut({ptr}..({ptr}+({len}*{elem_len}))).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                        name = self.name(),
                        ptr = ptr.rust_name(),
                        len = len.rust_name(),
                        elem_len = self.binding.type_().mem_size(),
                    ),

                    format!("let mut {name}: &mut [{typename}] = unsafe {{ ::std::slice::from_raw_parts_mut({name}___MEM.as_mut_ptr() as *mut {typename}, {len}) }};",
                        name = self.name(),
                        typename = self.type_name(),
                        len = len.rust_name(),
                    ),
                ],
                BindingParam::Value {..} => unreachable!(),
            }
        }
    }
}

pub struct RustIdiomRet<'a> {
    binding: FuncBinding<'a>,
}

impl<'a> RustIdiomRet<'a> {
    pub fn name(&self) -> String {
        self.binding.rust_name()
    }
    pub fn direction(&self) -> BindingDirection {
        self.binding.direction()
    }
    pub fn type_(&self) -> Datatype<'a> {
        self.binding.type_()
    }
    pub fn type_name(&self) -> String {
        self.type_().rust_type_name()
    }
    pub fn param(&self) -> BindingParam<'a> {
        self.binding.param()
    }

    pub fn ret_declaration(&self) -> String {
        match self.binding.param() {
            BindingParam::Ptr { .. } | BindingParam::Value { .. } => self.type_name(),
            BindingParam::Slice { .. } => unreachable!(),
        }
    }

    pub fn guest_abi_args(&self) -> Vec<String> {
        match self.binding.param() {
            BindingParam::Ptr(ptr) => vec![
                format!(
                    "#[allow(non_snake_case)] let mut {}___MEM = ::std::mem::MaybeUninit::<{}>::uninit();",
                    ptr.rust_name(),
                    self.type_name(),
                ),
                format!(
                    "let {} = {}___MEM.as_mut_ptr() as i32;",
                    ptr.rust_name(),
                    ptr.rust_name()
                ),
            ],
            BindingParam::Value { .. } => vec![],
            BindingParam::Slice { .. } => unreachable!(),
        }
    }

    pub fn guest_from_abi_call(&self) -> String {
        match self.binding.param() {
            BindingParam::Ptr(ptr) => format!(
                "let {} = unsafe {{ {}___MEM.assume_init() }};",
                self.name(),
                ptr.rust_name()
            ),
            BindingParam::Value(_val) => cast_value_to_binding(&self.binding),
            BindingParam::Slice { .. } => unreachable!(),
        }
    }

    pub fn host_unpack_to_abi(&self) -> Vec<String> {
        match self.binding.param() {
            BindingParam::Ptr(ptr) => {
                let mut lines = vec![
                format!(
                    "let {ptr} = {ptr} as usize;",
                     ptr = ptr.rust_name()
                ),
                format!(
                    "if {ptr} % {align} != 0 {{ Err(())?; /* FIXME: align failed */ }}",
                    ptr = ptr.name(),
                    align = self.binding.type_().mem_align(),
                ),
                format!(
                    "#[allow(non_snake_case)] let mut {name}___MEM: &mut [u8] = heap.get_mut({ptr}..({ptr}+{len})).ok_or_else(|| () /* FIXME: bounds check failed */)?;",
                    name = self.name(),
                    ptr = ptr.rust_name(),
                    len =self.binding.type_().mem_size(),
                )];
                match self.binding.type_().canonicalize().variant() {
                    DatatypeVariant::Enum(_) | DatatypeVariant::Struct(_) => {
                        lines.push(format!("if !{}::validate_bytes(&{}___MEM) {{ Err(())?; /* FIXME invalid representation */ }}",
                        self.type_name(), self.name()))
                    }
                    DatatypeVariant::Atom(_) => {} // No representation validation required
                    DatatypeVariant::Alias(_) => unreachable!("anti-aliased"),
                }
                lines.push(format!(
                    "let mut {ptr}: &mut {typename} = unsafe {{ ({name}___MEM.as_mut_ptr() as *mut {typename}).as_mut().expect(\"determined to be valid ref\")  }}; // convert pointer in linear memory to ref",
                    name = self.name(),
                    typename = self.type_name(),
                    ptr = ptr.rust_name(),
                ));
                lines
            }
            BindingParam::Value(_val) => vec![],
            BindingParam::Slice { .. } => unreachable!(),
        }
    }

    pub fn host_unpack_from_abi(&self) -> String {
        match self.binding.param() {
            BindingParam::Ptr(ptr) => format!(
                "*{} = {}; // Copy into out-pointer reference",
                ptr.rust_name(),
                self.name(),
            ),
            BindingParam::Value(val) => match val.type_().variant() {
                DatatypeVariant::Atom(AtomType::Bool) => format!(
                    "let {value}: {typename} = {arg} != 0;",
                    value = val.rust_name(),
                    typename = val.type_().rust_type_name(),
                    arg = self.name(),
                ),

                _ => format!(
                    "let {value}: {typename} = {arg} as {typename};",
                    value = val.rust_name(),
                    typename = val.type_().rust_type_name(),
                    arg = self.name(),
                ),
            },
            BindingParam::Slice { .. } => unreachable!(),
        }
    }
}

fn cast_value_to_binding(b: &FuncBinding) -> String {
    match b.param() {
        BindingParam::Value(val) => match b.type_().canonicalize().variant() {
            DatatypeVariant::Enum(_) => format!(
                "let {} = {}::from_u32({} as u32).ok_or(())?; // FIXME throw the right error",
                b.rust_name(),
                b.type_().rust_type_name(),
                val.rust_name(),
            ),
            DatatypeVariant::Atom(AtomType::Bool) => {
                format!("let {} = {} != 0;", b.rust_name(), val.rust_name(),)
            }
            DatatypeVariant::Atom(_) => format!(
                "let {} = {} as {};",
                b.rust_name(),
                val.rust_name(),
                b.type_().rust_type_name(),
            ),
            DatatypeVariant::Alias(_) => unreachable!("anti-aliased"),
            DatatypeVariant::Struct(_) => unreachable!("can't represent struct as binding value"),
        },
        _ => panic!("can only cast BindingParam::Value to binding type"),
    }
}
