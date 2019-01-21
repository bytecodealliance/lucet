use crate::error::{LucetcError, LucetcErrorKind};
use failure::{format_err, ResultExt};
use parity_wasm::elements::Instruction;

pub fn const_init_expr(opcodes: &[Instruction]) -> Result<i64, LucetcError> {
    let len = opcodes.len();
    if !(len >= 1 && opcodes[len - 1] == Instruction::End) {
        Err(format_err!(
            "invalid init expr: must be terminated with End opcode, got {:?}",
            opcodes
        ))?;
    }

    if len > 2 {
        Err(format_err!(
            "init expr is too long to be a single const, got {:?}",
            opcodes
        ))
        .context(LucetcErrorKind::Unsupported(
            "non-const init expr".to_owned(),
        ))?;
    }

    match opcodes[0] {
        Instruction::I32Const(i32_const) => Ok(i32_const as i64),
        Instruction::I64Const(i64_const) => Ok(i64_const),
        _ => Err(format_err!(
            "init expr is not a const integer expr, got {:?}",
            opcodes
        ))
        .context(LucetcErrorKind::Unsupported("non-int init expr".to_owned()))?,
    }
}
