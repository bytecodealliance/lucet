use cranelift_module::{DataContext, Linkage};
use failure::Error;
use lucet_module_data::writer::ModuleData;

use crate::compiler::Compiler;
use crate::program::data::sparse::make_sparse;

pub fn compile_module_data(compiler: &mut Compiler) -> Result<(), Error> {
    let mut module_data_serialized: Vec<u8> = Vec::new();
    {
        let heap_spec = compiler.prog.heap_spec();
        let sparse_data = make_sparse(
            &compiler.prog.data_initializers()?,
            compiler.prog.heap_spec(),
        );
        let module_data = ModuleData::new(heap_spec, sparse_data);
        module_data.serialize(&mut module_data_serialized)?;
    }

    let mut module_data_ctx = DataContext::new();
    module_data_ctx.define(module_data_serialized.into_boxed_slice());

    let module_data_decl =
        compiler
            .module
            .declare_data("lucet_module_data", Linkage::Export, false)?;
    compiler
        .module
        .define_data(module_data_decl, &module_data_ctx)?;
    Ok(())
}
