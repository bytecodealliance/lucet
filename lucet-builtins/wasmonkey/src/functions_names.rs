use crate::errors::*;
use parity_wasm::elements::{FunctionNameSubsection, IndexMap};

pub fn prepend_function_name(
    function_names_subsection: &mut FunctionNameSubsection,
    name: String,
) -> Result<(), WError> {
    let mut map_new = IndexMap::with_capacity(function_names_subsection.names().len() + 1 as usize);
    for (idx, name) in function_names_subsection.names() {
        map_new.insert(idx + 1, name.clone());
    }
    map_new.insert(0, name);
    *function_names_subsection.names_mut() = map_new;
    Ok(())
}
