use super::Compiler;
use crate::program::memory::HeapSpec;
use byteorder::{LittleEndian, WriteBytesExt};
use cranelift_module::{DataContext, Linkage};
use failure::Error;

pub fn compile_memory_specs(compiler: &mut Compiler) -> Result<(), Error> {
    let heap = compiler.prog.heap_spec();

    let mut heap_spec_ctx = DataContext::new();
    heap_spec_ctx.define(serialize_spec(&heap).into_boxed_slice());
    let heap_spec_decl = compiler
        .module
        .declare_data("lucet_heap_spec", Linkage::Export, false)?;
    compiler
        .module
        .define_data(heap_spec_decl, &heap_spec_ctx)?;
    Ok(())
}

fn serialize_spec(spec: &HeapSpec) -> Vec<u8> {
    let mut serialized: Vec<u8> = Vec::with_capacity(5 * 8);

    serialized
        .write_u64::<LittleEndian>(spec.reserved_size)
        .unwrap();
    serialized
        .write_u64::<LittleEndian>(spec.guard_size)
        .unwrap();
    serialized
        .write_u64::<LittleEndian>(spec.initial_size)
        .unwrap();
    serialized
        .write_u64::<LittleEndian>(spec.max_size.unwrap_or(0))
        .unwrap();
    serialized
        .write_u64::<LittleEndian>(if spec.max_size.is_none() { 0 } else { 1 })
        .unwrap();
    serialized
}
