use crate::compiler::Compiler;
use crate::program::data::sparse::CompiledSparseData;
use byteorder::{LittleEndian, WriteBytesExt};
use cranelift_module::{DataContext, Linkage};
use failure::Error;
use lucet_module_data::ModuleData;

pub fn compile_module_data(compiler: &mut Compiler) -> Result<(), Error> {
    let module_data_serialized: Vec<u8> = {
        let heap_spec = compiler.prog.heap_spec();
        let compiled_data = CompiledSparseData::new(
            &compiler.prog.data_initializers()?,
            compiler.prog.heap_spec(),
        );
        let sparse_data = compiled_data.sparse_data();
        let module_data = ModuleData::new(heap_spec, sparse_data, Vec::new());
        module_data.serialize_bincode()?
    };

    {
        let mut serialized_len: Vec<u8> = Vec::new();
        serialized_len
            .write_u32::<LittleEndian>(module_data_serialized.len() as u32)
            .unwrap();
        let mut data_len_ctx = DataContext::new();
        data_len_ctx.define(serialized_len.into_boxed_slice());

        let data_len_decl =
            compiler
                .module
                .declare_data("lucet_module_data_len", Linkage::Export, false)?;
        compiler.module.define_data(data_len_decl, &data_len_ctx)?;
    }

    {
        let mut module_data_ctx = DataContext::new();
        module_data_ctx.define(module_data_serialized.into_boxed_slice());

        let module_data_decl =
            compiler
                .module
                .declare_data("lucet_module_data", Linkage::Export, true)?;
        compiler
            .module
            .define_data(module_data_decl, &module_data_ctx)?;
    }
    Ok(())
}
