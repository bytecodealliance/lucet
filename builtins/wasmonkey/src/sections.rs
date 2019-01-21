use parity_wasm::elements::{Module, Section};

pub fn find_type_section_idx(module: &Module) -> Option<usize> {
    module.sections().iter().position(|section| match section {
        Section::Type(_) => true,
        _ => false,
    })
}
