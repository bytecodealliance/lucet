use crate::error::LucetcError;
use bimap::BiMap;
use parity_wasm::elements::{Internal, Module, NameSection, Section};
use std::collections::HashMap;

pub struct ModuleNames {
    func_exports: Vec<u32>,
    func_names: BiMap<u32, String>,
    glob_exports: HashMap<usize, String>,
}

impl ModuleNames {
    pub fn function_symbol(&self, ix: u32) -> String {
        let n = self.func_names.get_by_left(&ix);
        if self.function_exported(ix) {
            format!("guest_func_{}", n.expect("exported name must be defined"))
        } else {
            if let Some(n) = n {
                format!("guest_internalfunc_{}", n)
            } else {
                format!("guest_internalfunc_{}", ix)
            }
        }
    }
    pub fn function_exported(&self, ix: u32) -> bool {
        self.func_exports.contains(&ix)
    }
    pub fn global_symbol(&self, ix: u32) -> Option<String> {
        self.glob_exports.get(&(ix as usize)).cloned()
    }
}

fn define_unique_name(func_names: &mut BiMap<u32, String>, ix: u32, n: String) {
    assert!(!func_names.contains_left(&ix));
    if func_names.contains_right(&n) {
        // Name is not unique, search for one:
        let mut suffix: usize = 1;
        loop {
            let n_uniq = format!("{}_{}", n, suffix);
            if !func_names.contains_right(&n_uniq) {
                func_names.insert(ix, n_uniq);
                break;
            }
            suffix += 1;
        }
    } else {
        func_names.insert(ix, n);
    }
}

pub fn module_names(module: &Module) -> Result<ModuleNames, LucetcError> {
    let mut func_exports = Vec::new();
    let mut func_names = BiMap::new();
    let mut glob_exports = HashMap::new();

    if let Some(export_entries) = module.export_section().map(|s| s.entries()) {
        for entry in export_entries.iter() {
            match *entry.internal() {
                Internal::Function(idx) => {
                    func_exports.push(idx);
                    func_names.insert(idx, entry.field().to_owned());
                }
                Internal::Global(idx) => {
                    glob_exports.insert(idx as usize, String::from(entry.field()));
                }
                Internal::Table(_) => {} // We do not do anything with exported tables
                Internal::Memory(_) => {} // We do not do anything with exported memories
            }
        }
    }

    for section in module.sections() {
        match *section {
            Section::Name(ref name_section) => match *name_section {
                NameSection::Function(ref func_sect) => {
                    for (idx, name) in func_sect.names() {
                        if !func_names.contains_left(&idx) {
                            define_unique_name(&mut func_names, idx, name.clone());
                            func_names.insert(idx, name.to_owned());
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    Ok(ModuleNames {
        func_exports,
        glob_exports,
        func_names,
    })
}

#[cfg(test)]
mod tests {
    use super::define_unique_name;
    use bimap::BiMap;
    #[test]
    fn trivial() {
        let mut func_names = BiMap::new();
        func_names.insert(0, "zero".to_owned());
        define_unique_name(&mut func_names, 1, "one".to_owned());
        assert_eq!(func_names.get_by_left(&1), Some(&"one".to_owned()));
    }

    #[test]
    fn one_dup() {
        let mut func_names = BiMap::new();
        func_names.insert(0, "foo".to_owned());
        define_unique_name(&mut func_names, 1, "foo".to_owned());
        assert_eq!(func_names.get_by_left(&1), Some(&"foo_1".to_owned()));
    }

    #[test]
    fn two_dup() {
        let mut func_names = BiMap::new();
        func_names.insert(0, "foo".to_owned());
        define_unique_name(&mut func_names, 1, "foo".to_owned());
        define_unique_name(&mut func_names, 2, "foo".to_owned());
        assert_eq!(func_names.get_by_left(&1), Some(&"foo_1".to_owned()));
        assert_eq!(func_names.get_by_left(&2), Some(&"foo_2".to_owned()));
    }

    #[test]
    fn dup_of_base() {
        let mut func_names = BiMap::new();
        func_names.insert(0, "foo_1".to_owned());
        define_unique_name(&mut func_names, 1, "foo".to_owned());
        define_unique_name(&mut func_names, 2, "foo".to_owned());
        assert_eq!(func_names.get_by_left(&1), Some(&"foo".to_owned()));
        assert_eq!(func_names.get_by_left(&2), Some(&"foo_2".to_owned()));
    }
}
