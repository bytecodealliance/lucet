use errors::*;
use parity_wasm::elements::{FunctionNameSection, IndexMap};

pub fn prepend_function_name(
    function_names_section: &mut FunctionNameSection,
    name: String,
) -> Result<(), WError> {
    let mut map_new = IndexMap::with_capacity(function_names_section.names().len() + 1 as usize);
    for (idx, name) in function_names_section.names() {
        map_new.insert(idx + 1, name.clone());
    }
    map_new.insert(0, name);
    *function_names_section.names_mut() = map_new;
    Ok(())
}
