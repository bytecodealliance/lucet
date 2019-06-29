use cranelift_codegen::ir;
use cranelift_object::ObjectTrapSite;

use failure::Error;
use lucet_module::TrapSite;
use object::write::{Object, StandardSection, Symbol, SymbolId};
use object::{SymbolKind, SymbolScope};

pub fn write_trap_table(
    func_sym: SymbolId,
    traps: &[ObjectTrapSite],
    obj: &mut Object,
) -> Result<SymbolId, Error> {
    let func_name = &obj.symbol(func_sym).name;
    let trap_sym = trap_sym_for_func(func_name);

    // write the actual function-level trap table
    let traps: Vec<TrapSite> = traps
        .iter()
        .map(|site| TrapSite {
            offset: site.offset,
            code: translate_trapcode(site.code),
        })
        .collect();

    let trap_site_bytes = unsafe {
        std::slice::from_raw_parts(
            traps.as_ptr() as *const u8,
            traps.len() * std::mem::size_of::<TrapSite>(),
        )
    };

    // and write the function trap table into the object
    let section_id = obj.section_id(StandardSection::ReadOnlyData);
    let value = obj.append_section_data(section_id, trap_site_bytes, 4);
    let size = trap_site_bytes.len() as u64;
    let symbol_id = obj.add_symbol(Symbol {
        name: trap_sym,
        value,
        size,
        kind: SymbolKind::Data,
        scope: SymbolScope::Dynamic,
        weak: false,
        section: Some(section_id),
    });

    Ok(symbol_id)
}

fn trap_sym_for_func(sym: &[u8]) -> Vec<u8> {
    let mut trap_sym = b"lucet_trap_table_".to_vec();
    trap_sym.extend_from_slice(sym);
    trap_sym
}

// Trapcodes can be thought of as a tuple of (type, subtype). Each are
// represented as a 16-bit unsigned integer. These are packed into a u32
// wherein the type occupies the low 16 bites and the subtype takes the
// high bits.
//
// Not all types have subtypes. Currently, only the user User type has a
// subtype.
fn translate_trapcode(code: ir::TrapCode) -> lucet_module::TrapCode {
    match code {
        ir::TrapCode::StackOverflow => lucet_module::TrapCode::StackOverflow,
        ir::TrapCode::HeapOutOfBounds => lucet_module::TrapCode::HeapOutOfBounds,
        ir::TrapCode::OutOfBounds => lucet_module::TrapCode::OutOfBounds,
        ir::TrapCode::IndirectCallToNull => lucet_module::TrapCode::IndirectCallToNull,
        ir::TrapCode::BadSignature => lucet_module::TrapCode::BadSignature,
        ir::TrapCode::IntegerOverflow => lucet_module::TrapCode::IntegerOverflow,
        ir::TrapCode::IntegerDivisionByZero => lucet_module::TrapCode::IntegerDivByZero,
        ir::TrapCode::BadConversionToInteger => lucet_module::TrapCode::BadConversionToInteger,
        ir::TrapCode::Interrupt => lucet_module::TrapCode::Interrupt,
        ir::TrapCode::TableOutOfBounds => lucet_module::TrapCode::TableOutOfBounds,
        ir::TrapCode::UnreachableCodeReached => lucet_module::TrapCode::Unreachable,
        ir::TrapCode::User(_) => panic!("we should never emit a user trapcode"),
    }
}
