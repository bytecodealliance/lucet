use crate::{AtomType, Error, FuncSignature, ImportFunc};
use cranelift_entity::{entity_impl, PrimaryMap};
use std::collections::HashMap;
use wasmparser::{
    ExternalKind, FuncType, ImportSectionEntryType, ModuleReader, SectionContent, Type as WType,
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

#[derive(Clone)]
pub struct ModuleType {
    types: PrimaryMap<TypeIndex, FuncSignature>,
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

    pub fn export(&self, name: &str) -> Option<&FuncSignature> {
        self.exports.get(name).map(|funcix| {
            let func = self.funcs.get(*funcix).expect("valid funcix");
            self.types.get(func.ty).expect("valid typeix")
        })
    }

    pub fn parse_wasm(module_contents: &[u8]) -> Result<Self, Error> {
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
                                form: WType::Func,
                                params,
                                returns,
                            } => {
                                let ret = match returns.len() {
                                    0 => None,
                                    1 => Some(wasmparser_to_atomtype(&returns[0])?),
                                    _ => Err(Error::Unsupported(format!(
                                        "more than 1 return value: {:?}",
                                        returns,
                                    )))?,
                                };
                                let args = params
                                    .iter()
                                    .map(|a| wasmparser_to_atomtype(a))
                                    .collect::<Result<Vec<_>, _>>()?;
                                module.types.push(FuncSignature { args, ret });
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

fn wasmparser_to_atomtype(a: &WType) -> Result<AtomType, Error> {
    match a {
        WType::I32 => Ok(AtomType::I32),
        WType::I64 => Ok(AtomType::I64),
        WType::F32 => Ok(AtomType::F32),
        WType::F64 => Ok(AtomType::F64),
        _ => Err(Error::Unsupported(format!("wasmparser type {:?}", a))),
    }
}
