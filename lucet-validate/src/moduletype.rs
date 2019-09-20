use crate::Error;
use cranelift_entity::{entity_impl, PrimaryMap};
use std::collections::HashMap;
pub use wasmparser::Type;
use wasmparser::{ExternalKind, FuncType, ImportSectionEntryType, ModuleReader, SectionContent};

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
struct TypeIndex(u32);
entity_impl!(TypeIndex);

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
struct FuncIndex(u32);
entity_impl!(FuncIndex);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuncSignature {
    pub params: Vec<Type>,
    pub returns: Vec<Type>,
}

#[derive(Clone)]
struct Func {
    pub ty: TypeIndex,
    pub import: Option<(String, String)>,
}

#[derive(Clone)]
pub struct ModuleType {
    types: PrimaryMap<TypeIndex, FuncSignature>,
    funcs: PrimaryMap<FuncIndex, Func>,
    exports: HashMap<String, FuncIndex>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportFunc {
    pub module: String,
    pub field: String,
    pub ty: FuncSignature,
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

    pub fn export(&self, name: &str) -> Option<&FuncSignature> {
        self.exports.get(name).map(|funcix| {
            let func = self.funcs.get(*funcix).expect("valid funcix");
            self.types.get(func.ty).expect("valid typeix")
        })
    }

    pub fn parse(module_contents: &[u8]) -> Result<Self, Error> {
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
                            FuncType {
                                form: wasmparser::Type::Func,
                                params,
                                returns,
                            } => {
                                module.types.push(FuncSignature {
                                    params: params.to_vec(),
                                    returns: returns.to_vec(),
                                });
                            }
                            _ => Err(Error::Unsupported("type section entry".to_string()))?,
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
                                        import.field.to_owned(),
                                    )),
                                });
                            }
                            ImportSectionEntryType::Memory(_) => {
                                Err(Error::Unsupported(format!(
                                    "memory import {}:{}",
                                    import.module, import.field
                                )))?;
                            }
                            ImportSectionEntryType::Table(_) => {
                                Err(Error::Unsupported(format!(
                                    "table import {}:{}",
                                    import.module, import.field
                                )))?;
                            }
                            ImportSectionEntryType::Global(_) => {
                                Err(Error::Unsupported(format!(
                                    "global import {}:{}",
                                    import.module, import.field
                                )))?;
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
