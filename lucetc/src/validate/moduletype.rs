use cranelift_entity::{entity_impl, PrimaryMap};
use std::collections::HashMap;
use thiserror::Error;
use wasmparser::{
    ExternalKind, FuncType, ImportSectionEntryType, ModuleReader, SectionContent, TypeDef,
};

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
struct TypeIndex(u32);
entity_impl!(TypeIndex);

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
struct FuncIndex(u32);
entity_impl!(FuncIndex);

#[derive(Clone)]
struct Func {
    pub ty: TypeIndex,
    pub import: Option<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportFunc {
    pub module: String,
    pub field: String,
    pub ty: FuncType,
}

#[derive(Debug, Error)]
pub enum ModuleTypeError {
    #[error("WebAssembly validation error at offset {1}: {0}")]
    WasmValidation(String, usize),
    #[error("Unsupported: {0}")]
    Unsupported(String),
}

impl From<wasmparser::BinaryReaderError> for ModuleTypeError {
    fn from(e: wasmparser::BinaryReaderError) -> ModuleTypeError {
        ModuleTypeError::WasmValidation(e.message().to_owned(), e.offset())
    }
}

#[derive(Clone)]
pub struct ModuleType {
    types: PrimaryMap<TypeIndex, FuncType>,
    funcs: PrimaryMap<FuncIndex, Func>,
    exports: HashMap<String, FuncIndex>,
}

impl ModuleType {
    pub fn imports(&self) -> Vec<ImportFunc> {
        self.funcs
            .iter()
            .filter_map(|(_, f)| {
                f.import.clone().map(|(module, field)| ImportFunc {
                    module,
                    field,
                    ty: self.types.get(f.ty).expect("get type").clone(),
                })
            })
            .collect()
    }

    pub fn export(&self, name: &str) -> Option<&FuncType> {
        self.exports.get(name).map(|funcix| {
            let func = self.funcs.get(*funcix).expect("valid funcix");
            self.types.get(func.ty).expect("valid typeix")
        })
    }

    pub fn parse_wasm(module_contents: &[u8]) -> Result<Self, ModuleTypeError> {
        let mut module = ModuleType {
            types: PrimaryMap::new(),
            funcs: PrimaryMap::new(),
            exports: HashMap::new(),
        };

        let mut module_reader = ModuleReader::new(module_contents)?;
        while !module_reader.eof() {
            let section = module_reader.read()?;
            match section.content()? {
                SectionContent::Type(types) => {
                    for entry in types {
                        match entry? {
                            TypeDef::Func(functype) => {
                                module.types.push(functype);
                            }
                            _ => {
                                return Err(ModuleTypeError::Unsupported(
                                    "type section entry".to_string(),
                                ))
                            }
                        }
                    }
                }
                SectionContent::Import(imports) => {
                    for import in imports {
                        let import = import?;
                        match import.ty {
                            ImportSectionEntryType::Function(ftype) => {
                                module.funcs.push(Func {
                                    ty: TypeIndex::from_u32(ftype),
                                    import: Some((
                                        import.module.to_owned(),
                                        import
                                            .field
                                            .expect("func import has field name")
                                            .to_owned(),
                                    )),
                                });
                            }
                            ImportSectionEntryType::Memory(_) => {
                                return Err(ModuleTypeError::Unsupported(format!(
                                    "memory import {}:{:?}",
                                    import.module, import.field
                                )));
                            }
                            ImportSectionEntryType::Table(_) => {
                                return Err(ModuleTypeError::Unsupported(format!(
                                    "table import {}:{:?}",
                                    import.module, import.field
                                )));
                            }
                            ImportSectionEntryType::Global(_) => {
                                return Err(ModuleTypeError::Unsupported(format!(
                                    "global import {}:{:?}",
                                    import.module, import.field
                                )));
                            }
                            ImportSectionEntryType::Module(_) => {
                                return Err(ModuleTypeError::Unsupported(format!(
                                    "module import {}:{:?}",
                                    import.module, import.field
                                )));
                            }
                            ImportSectionEntryType::Instance(_) => {
                                return Err(ModuleTypeError::Unsupported(format!(
                                    "instance import {}:{:?}",
                                    import.module, import.field
                                )));
                            }
                        }
                    }
                }
                SectionContent::Export(exports) => {
                    for export in exports {
                        let export = export?;
                        match export.kind {
                            ExternalKind::Function => {
                                module.exports.insert(
                                    export.field.to_string(),
                                    FuncIndex::from_u32(export.index),
                                );
                            }
                            _ => {} // Dont care about other exports
                        }
                    }
                }
                SectionContent::Function(functions) => {
                    for function_ty in functions {
                        let ty = TypeIndex::from_u32(function_ty?);
                        module.funcs.push(Func { ty, import: None });
                    }
                }
                _ => {} // Dont care about other sections
            }
        }

        Ok(module)
    }
}
