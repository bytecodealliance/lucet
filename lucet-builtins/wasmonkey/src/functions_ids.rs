use crate::errors::*;
use parity_wasm::elements::{
    CodeSection, ElementSection, ExportSection, FuncBody, Instruction, Instructions, Internal,
    Module,
};

fn shift_function_ids_in_code_section(
    code_section: &mut CodeSection,
    shift: u32,
) -> Result<(), WError> {
    let code_bodies = code_section.bodies_mut();
    for code_body in code_bodies.iter_mut() {
        let opcodes = code_body.code_mut().elements_mut();
        for opcode in opcodes.iter_mut() {
            if let Instruction::Call(function_id) = *opcode {
                *opcode = Instruction::Call(function_id + shift)
            }
        }
    }
    Ok(())
}

fn shift_function_ids_in_exports_section(export_section: &mut ExportSection, shift: u32) {
    for entry in export_section.entries_mut() {
        let internal = entry.internal_mut();
        if let Internal::Function(function_id) = *internal {
            *internal = Internal::Function(function_id + shift)
        }
    }
}

fn shift_function_ids_in_elements_section(elements_section: &mut ElementSection, shift: u32) {
    for elements_segment in elements_section.entries_mut() {
        for function_id in elements_segment.members_mut() {
            *function_id += shift;
        }
    }
}

pub fn shift_function_ids(module: &mut Module, shift: u32) -> Result<(), WError> {
    shift_function_ids_in_code_section(module.code_section_mut().expect("No code section"), shift)?;
    if let Some(export_section) = module.export_section_mut() {
        shift_function_ids_in_exports_section(export_section, shift)
    }
    if let Some(elements_section) = module.elements_section_mut() {
        shift_function_ids_in_elements_section(elements_section, shift)
    }
    Ok(())
}

fn replace_function_id_in_code_section(code_section: &mut CodeSection, before: u32, after: u32) {
    let code_bodies = code_section.bodies_mut();
    for code_body in code_bodies.iter_mut() {
        let opcodes = code_body.code_mut().elements_mut();
        for opcode in opcodes.iter_mut() {
            match *opcode {
                Instruction::Call(function_id) if function_id == before => {
                    *opcode = Instruction::Call(after)
                }
                _ => {}
            }
        }
    }
}

fn replace_function_id_in_elements_section(
    elements_section: &mut ElementSection,
    before: u32,
    after: u32,
) {
    for elements_segment in elements_section.entries_mut() {
        for function_id in elements_segment.members_mut() {
            if *function_id == before {
                *function_id = after;
            }
        }
    }
}

pub fn replace_function_id(module: &mut Module, before: u32, after: u32) -> Result<(), WError> {
    if let Some(code_section) = module.code_section_mut() {
        replace_function_id_in_code_section(code_section, before, after)
    }

    if let Some(elements_section) = module.elements_section_mut() {
        replace_function_id_in_elements_section(elements_section, before, after)
    };

    Ok(())
}

#[allow(dead_code)]
pub fn disable_function_id(module: &mut Module, function_id: u32) -> Result<(), WError> {
    let base_id = match module.import_section() {
        None => 0,
        Some(import_section) => import_section.entries().len() as u32,
    };
    let code_section = module.code_section_mut().expect("No code section");
    let code_bodies = code_section.bodies_mut();
    let opcodes = Instructions::new(vec![Instruction::Unreachable, Instruction::End]);
    let func_body = FuncBody::new(vec![], opcodes);
    code_bodies[(function_id - base_id) as usize] = func_body;
    Ok(())
}
