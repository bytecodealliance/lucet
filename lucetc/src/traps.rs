use cranelift_codegen::ir;

pub(crate) fn trap_sym_for_func(sym: &str) -> String {
    return format!("lucet_trap_table_{}", sym);
}

// Trapcodes can be thought of as a tuple of (type, subtype). Each are
// represented as a 16-bit unsigned integer. These are packed into a u32
// wherein the type occupies the low 16 bites and the subtype takes the
// high bits.
//
// Not all types have subtypes. Currently, only the user User type has a
// subtype.
pub(crate) fn translate_trapcode(code: ir::TrapCode) -> lucet_module::TrapCode {
    match code {
        ir::TrapCode::StackOverflow => lucet_module::TrapCode::StackOverflow,
        ir::TrapCode::HeapOutOfBounds => lucet_module::TrapCode::HeapOutOfBounds,
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
