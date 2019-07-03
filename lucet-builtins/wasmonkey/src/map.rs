use crate::errors::*;
use serde_json;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

#[derive(Clone, Debug, Default, Serialize)]
pub struct PatchedBuiltinsMap {
    pub env: HashMap<String, String>,
}

impl PatchedBuiltinsMap {
    pub fn with_capacity(capacity: usize) -> Self {
        PatchedBuiltinsMap {
            env: HashMap::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, name: String, imported_name: String) -> Option<String> {
        self.env.insert(name, imported_name)
    }

    pub fn write_to_file<P: AsRef<Path>>(
        &self,
        builtins_map_path: P,
        original_names: bool,
    ) -> Result<(), WError> {
        let mut map_with_original_names;
        let map = if original_names {
            self
        } else {
            map_with_original_names = PatchedBuiltinsMap::default();
            for imported_name in self.env.values() {
                map_with_original_names
                    .env
                    .insert(imported_name.clone(), imported_name.clone());
            }
            &map_with_original_names
        };
        let json = serde_json::to_string_pretty(map).map_err(|_| WError::ParseError)?;
        File::create(builtins_map_path)?.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn builtins_map(
        &self,
        module: &str,
        original_names: bool,
    ) -> Result<HashMap<String, String>, WError> {
        if module != "env" {
            xbail!(WError::UsageError("Empty module"))
        }
        if original_names {
            return Ok(self.env.clone());
        }
        let mut env_map_with_original_names = HashMap::new();
        for imported_name in self.env.values() {
            env_map_with_original_names.insert(imported_name.clone(), imported_name.clone());
        }
        Ok(env_map_with_original_names)
    }
}
