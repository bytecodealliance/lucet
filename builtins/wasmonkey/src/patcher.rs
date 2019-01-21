use errors::*;
use functions_ids::*;
use functions_names::*;
use map::*;
use parity_wasm;
use parity_wasm::elements::{
    self, External, ImportEntry, ImportSection, Internal, Module, NameSection, Section,
};
use sections::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use symbols::{self, ExtractedSymbols};

pub const BUILTIN_PREFIX: &str = "builtin_";

#[derive(Default, Clone, Debug)]
pub struct PatcherConfig {
    pub builtins_path: Option<PathBuf>,
    pub builtins_map_path: Option<PathBuf>,
    pub builtins_map_original_names: bool,
    pub builtins_additional: Vec<String>,
}

pub struct Patcher {
    pub config: PatcherConfig,
    patched_module: Module,
    patched_builtins_map: PatchedBuiltinsMap,
}

impl Patcher {
    pub fn new(config: PatcherConfig, module: Module) -> Result<Self, WError> {
        let symbols = match &config.builtins_path {
            None => ExtractedSymbols::from(vec![]),
            Some(builtins_path) => symbols::extract_symbols(&builtins_path)?,
        }.merge_additional(&config.builtins_additional);
        let builtins_names = symbols.builtins_names();
        let (patched_module, patched_builtins_map) = patch_module(module, &builtins_names)?;
        let patcher = Patcher {
            config,
            patched_module,
            patched_builtins_map,
        };
        Ok(patcher)
    }

    pub fn from_bytes(config: PatcherConfig, bytes: &[u8]) -> Result<Self, WError> {
        let module = parity_wasm::deserialize_buffer(bytes)?;
        Self::new(config, module)
    }

    pub fn from_file<P: AsRef<Path>>(config: PatcherConfig, path_in: P) -> Result<Self, WError> {
        let module = parity_wasm::deserialize_file(path_in)?;
        Self::new(config, module)
    }

    pub fn into_bytes(self) -> Result<Vec<u8>, WError> {
        let bytes = elements::serialize(self.patched_module)?;
        Ok(bytes)
    }

    pub fn store_to_file<P: AsRef<Path>>(self, path_out: P) -> Result<(), WError> {
        elements::serialize_to_file(path_out, self.patched_module)?;
        if let Some(builtins_map_path) = self.config.builtins_map_path {
            self.patched_builtins_map
                .write_to_file(builtins_map_path, self.config.builtins_map_original_names)?;
        }
        Ok(())
    }

    pub fn patched_builtins_map(&self, module: &str) -> Result<HashMap<String, String>, WError> {
        self.patched_builtins_map
            .builtins_map(module, self.config.builtins_map_original_names)
    }

    pub fn patched_module(self) -> Module {
        self.patched_module
    }
}

#[derive(Debug)]
pub struct Builtin {
    pub name: String,
    pub original_function_id: Option<u32>,
    pub function_type_id: Option<u32>,
}

impl Builtin {
    pub fn new(name: String) -> Self {
        Builtin {
            name,
            original_function_id: None,
            function_type_id: None,
        }
    }

    pub fn import_name(&self) -> String {
        format!("{}{}", BUILTIN_PREFIX, self.name)
    }
}

fn function_type_id_for_function_id(module: &Module, function_id: u32) -> Option<u32> {
    let offset = module
        .import_section()
        .map(|import_section| import_section.entries().len() as u32)
        .unwrap_or(0);
    if function_id < offset {
        return None;
    }
    let functions_section_type_ids = module.function_section().unwrap().entries();
    Some(functions_section_type_ids[(function_id - offset) as usize].type_ref())
}

fn add_function_type_id_to_builtins(
    module: &Module,
    builtins: &mut Vec<Builtin>,
) -> Result<(), WError> {
    for builtin in builtins.iter_mut() {
        let function_type_id =
            function_type_id_for_function_id(module, builtin.original_function_id.unwrap())
                .expect("Function ID not found");
        builtin.function_type_id = Some(function_type_id);
    }
    Ok(())
}

fn retain_only_used_builtins(module: &Module, builtins: &mut Vec<Builtin>) -> Result<(), WError> {
    let export_section = module.export_section().expect("No export section");

    for entry in export_section.entries() {
        let internal = entry.internal();
        let function_id = match internal {
            Internal::Function(function_id) => *function_id,
            _ => continue,
        };
        let field = entry.field();
        for builtin in builtins.iter_mut() {
            if field == builtin.name {
                assert!(builtin.original_function_id.is_none());
                builtin.original_function_id = Some(function_id);
                break;
            }
        }
    }

    builtins.retain(|builtin| builtin.original_function_id.is_some());
    Ok(())
}

fn add_import_section_if_missing(module: &mut Module) -> Result<(), WError> {
    if module.import_section().is_some() {
        return Ok(());
    }
    let import_section = ImportSection::with_entries(vec![]);
    let import_section_idx = find_type_section_idx(&module).unwrap() + 1;
    module
        .sections_mut()
        .insert(import_section_idx, Section::Import(import_section));
    Ok(())
}

fn prepend_builtin_to_import_section(module: &mut Module, builtin: &Builtin) -> Result<(), WError> {
    let import_name = builtin.import_name();
    let external = External::Function(builtin.function_type_id.unwrap());
    let import_entry = ImportEntry::new("env".to_string(), import_name, external);
    module
        .import_section_mut()
        .unwrap()
        .entries_mut()
        .insert(0, import_entry);
    Ok(())
}

fn prepend_builtin_to_names_section(module: &mut Module, builtin: &Builtin) -> Result<(), WError> {
    let import_name = builtin.import_name();
    let names_section = module
        .names_section_mut()
        .expect("Names section not present");
    let function_names_section = match names_section {
        NameSection::Function(function_names_section) => function_names_section,
        _ => xbail!(WError::InternalError("Unexpected names section")),
    };
    prepend_function_name(function_names_section, import_name)?;
    Ok(())
}

fn patch_module(
    module: Module,
    builtins_names: &[&str],
) -> Result<(Module, PatchedBuiltinsMap), WError> {
    let mut module = module
        .parse_names()
        .map_err(|_| WError::InternalError("Unable to parse names"))?;

    let mut builtins: Vec<_> = builtins_names
        .iter()
        .map(|x| Builtin::new(x.to_string()))
        .collect();

    retain_only_used_builtins(&module, &mut builtins)?;
    add_function_type_id_to_builtins(&module, &mut builtins)?;

    add_import_section_if_missing(&mut module)?;
    for (builtin_idx, builtin) in builtins.iter_mut().enumerate() {
        prepend_builtin_to_import_section(&mut module, &builtin)?;
        prepend_builtin_to_names_section(&mut module, &builtin)?;
        shift_function_ids(&mut module, 1)?;
        let original_function_id = builtin.original_function_id.unwrap() + builtin_idx as u32 + 1;
        let new_function_id = 0;
        replace_function_id(&mut module, original_function_id, new_function_id)?;
    }

    let mut patched_builtins_map = PatchedBuiltinsMap::with_capacity(builtins.len());
    for builtin in builtins {
        patched_builtins_map.insert(builtin.name.clone(), builtin.import_name());
    }
    Ok((module, patched_builtins_map))
}
