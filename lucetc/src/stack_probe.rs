//! Manual definition of the stack probe.
//!
//! Rust currently fails to reexport symbols in dynamic libraries. This means that the old way of
//! including an assembly stack probe in the runtime does not work when embedding in C.
//!
//! There is an [issue](https://github.com/rust-lang/rust/issues/36342) tracking this, but until
//! it's closed we are taking the approach of including the stack probe in every Lucet module, and
//! adding custom entries for it into the trap table, so that stack overflows in the probe will be
//! treated like any other guest trap.

use crate::decls::ModuleDecls;
use crate::error::Error;
use crate::module::UniqueFuncIndex;
use cranelift_codegen::binemit::TrapSink;
use cranelift_codegen::ir;
use cranelift_codegen::ir::{types, AbiParam, Signature};
use cranelift_codegen::isa::CallConv;
use cranelift_faerie::traps::{FaerieTrapManifest, FaerieTrapSink};
use cranelift_faerie::FaerieProduct;
use cranelift_module::{Backend as ClifBackend, Linkage, Module as ClifModule};
use faerie::Decl;

/// Stack probe symbol name
pub const STACK_PROBE_SYM: &'static str = "lucet_probestack";

/// The binary of the stack probe.
pub(crate) const STACK_PROBE_BINARY: &'static [u8] = &[
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

pub fn declare_metadata<'a, B: ClifBackend>(
    decls: &mut ModuleDecls<'a>,
    clif_module: &mut ClifModule<B>,
) -> Result<UniqueFuncIndex, Error> {
    Ok(decls
        .declare_new_function(
            clif_module,
            STACK_PROBE_SYM.to_string(),
            Linkage::Local,
            Signature {
                params: vec![],
                returns: vec![AbiParam::new(types::I32)],
                call_conv: CallConv::SystemV, // the stack probe function is very specific to x86_64, and possibly to SystemV ABI platforms?
            },
        )
        .unwrap())
}

pub fn declare_and_define(product: &mut FaerieProduct) -> Result<(), Error> {
    product
        .artifact
        .declare_with(
            STACK_PROBE_SYM,
            Decl::function(),
            STACK_PROBE_BINARY.to_vec(),
        )
        .map_err(|source| Error::Failure(source, "Stack probe error".to_owned()))?;
    add_sink(
        product
            .trap_manifest
            .as_mut()
            .expect("trap manifest is present"),
    );
    Ok(())
}

fn add_sink(manifest: &mut FaerieTrapManifest) {
    let mut stack_probe_trap_sink =
        FaerieTrapSink::new(STACK_PROBE_SYM, STACK_PROBE_BINARY.len() as u32);
    stack_probe_trap_sink.trap(
        10, /* test %rsp,0x8(%rsp) */
        ir::SourceLoc::default(),
        ir::TrapCode::StackOverflow,
    );
    stack_probe_trap_sink.trap(
        34, /* test %rsp,0x8(%rsp) */
        ir::SourceLoc::default(),
        ir::TrapCode::StackOverflow,
    );
    manifest.add_sink(stack_probe_trap_sink);
}
