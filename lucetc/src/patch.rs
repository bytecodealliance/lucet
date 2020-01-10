use crate::error::Error;
use parity_wasm::elements::Module;
use std::collections::HashMap;
use std::path::Path;
use wasmonkey::{Patcher, PatcherConfig};

pub fn patch_module<P: AsRef<Path>>(
    module: Module,
    builtins_path: P,
) -> Result<(Module, HashMap<String, String>), Error> {
    let mut patcher_config = PatcherConfig::default();
    patcher_config.builtins_map_original_names = false;
    patcher_config.builtins_path = Some(builtins_path.as_ref().into());
    let patcher = Patcher::new(patcher_config, module).map_err(Error::Patcher)?;
    let patched_builtins_map = patcher
        .patched_builtins_map("env")
        .map_err(Error::Patcher)?;
    let patched_module = patcher.patched_module();
    Ok((patched_module, patched_builtins_map))
}
