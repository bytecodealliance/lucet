//! Manual definition of the stack probe.
//!
//! Rust currently fails to reexport symbols in dynamic libraries. This means that the old way of
//! including an assembly stack probe in the runtime does not work when embedding in C.
//!
//! There is an [issue](https://github.com/rust-lang/rust/issues/36342) tracking this, but until
//! it's closed we are taking the approach of including the stack probe in every Lucet module, and
//! adding custom entries for it into the trap table, so that stack overflows in the probe will be
//! treated like any other guest trap.

use crate::compiler::CodegenContext;
use crate::decls::ModuleDecls;
use crate::error::Error;
use crate::module::UniqueFuncIndex;
use cranelift_codegen::{
    ir::{self, types, AbiParam, Signature},
    isa::CallConv,
};
use cranelift_module::{Linkage, TrapSite};
use cranelift_wasm::{WasmFuncType, WasmType};

/// Stack probe symbol name
pub const STACK_PROBE_SYM: &str = "lucet_probestack";

/// The binary of the stack probe.
pub(crate) const STACK_PROBE_BINARY: &[u8] = &[
    // 49 89 c3                     mov    %rax,%r11
    // 48 81 ec 00 10 00 00         sub    $0x1000,%rsp
    // 48 85 64 24 08               test   %rsp,0x8(%rsp)
    // 49 81 eb 00 10 00 00         sub    $0x1000,%r11
    // 49 81 fb 00 10 00 00         cmp    $0x1000,%r11
    // 77 e4                        ja     4dfd3 <lucet_probestack+0x3>
    // 4c 29 dc                     sub    %r11,%rsp
    // 48 85 64 24 08               test   %rsp,0x8(%rsp)
    // 48 01 c4                     add    %rax,%rsp
    // c3                           retq
    0x49, 0x89, 0xc3, 0x48, 0x81, 0xec, 0x00, 0x10, 0x00, 0x00, 0x48, 0x85, 0x64, 0x24, 0x08, 0x49,
    0x81, 0xeb, 0x00, 0x10, 0x00, 0x00, 0x49, 0x81, 0xfb, 0x00, 0x10, 0x00, 0x00, 0x77, 0xe4, 0x4c,
    0x29, 0xdc, 0x48, 0x85, 0x64, 0x24, 0x08, 0x48, 0x01, 0xc4, 0xc3,
];

pub fn trap_sites() -> Vec<TrapSite> {
    vec![
        TrapSite {
            offset: 10, /* test %rsp,0x8(%rsp) */
            srcloc: ir::SourceLoc::default(),
            code: ir::TrapCode::StackOverflow,
        },
        TrapSite {
            offset: 34, /* test %rsp,0x8(%rsp) */
            srcloc: ir::SourceLoc::default(),
            code: ir::TrapCode::StackOverflow,
        },
    ]
}

pub fn declare<'a>(
    decls: &mut ModuleDecls<'a>,
    codegen_context: &CodegenContext,
) -> Result<UniqueFuncIndex, Error> {
    Ok(decls
        .declare_new_function(
            codegen_context,
            STACK_PROBE_SYM.to_string(),
            Linkage::Local,
            WasmFuncType {
                params: vec![].into_boxed_slice(),
                returns: vec![WasmType::I32].into_boxed_slice(),
            },
            Signature {
                params: vec![],
                returns: vec![AbiParam::new(types::I32)],
                call_conv: CallConv::SystemV, // the stack probe function is very specific to x86_64, and possibly to SystemV ABI platforms?
            },
        )
        .unwrap())
}
