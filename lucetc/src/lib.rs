pub mod bindings;
pub mod compiler;
pub mod error;
pub mod load;
pub mod patch;
pub mod program;

use crate::compiler::data::{compile_data_initializers, compile_sparse_page_data};
use crate::compiler::function::compile_function;
use crate::compiler::globals::compile_global_specs;
use crate::compiler::memory::compile_memory_specs;
use crate::compiler::table::compile_table;
use crate::compiler::module_data::compile_module_data;
use crate::compiler::Compiler;
use crate::error::{LucetcError, LucetcErrorKind};
use crate::program::Program;
use failure::ResultExt;

pub fn compile<'p>(program: &'p Program, name: &str) -> Result<Compiler<'p>, LucetcError> {
    let mut compiler = Compiler::new(name.to_owned(), &program)?;

    compile_data_initializers(&mut compiler).context(LucetcErrorKind::DataInitializers)?;
    compile_sparse_page_data(&mut compiler).context(LucetcErrorKind::DataInitializers)?;
    compile_memory_specs(&mut compiler).context(LucetcErrorKind::MemorySpecs)?;
    compile_global_specs(&mut compiler).context(LucetcErrorKind::GlobalSpecs)?;
    compile_module_data(&mut compiler).context(LucetcErrorKind::ModuleData)?;

    for function in program.defined_functions() {
        let body = program.function_body(&function);
        compile_function(&mut compiler, &function, body)
            .context(LucetcErrorKind::Function(function.symbol().to_owned()))?;
    }
    for table in program.tables() {
        compile_table(&mut compiler, &table)
            .context(LucetcErrorKind::Table(table.symbol().to_owned()))?;
    }

    Ok(compiler)
}
