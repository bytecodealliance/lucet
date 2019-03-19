use crate::new::module::ModuleInfo;

use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir;
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_wasm::{
    FuncEnvironment, FuncIndex, GlobalIndex, GlobalVariable, MemoryIndex, ModuleEnvironment,
    SignatureIndex, TableIndex, WasmResult,
};

pub struct FuncInfo<'a> {
    module: &'a ModuleInfo<'a>,
}

impl<'a> FuncInfo<'a> {
    pub fn new(module: &'a ModuleInfo<'a>) -> Self {
        Self { module }
    }
}

impl<'a> FuncEnvironment for FuncInfo<'a> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.module.target_config()
    }

    fn make_global(&mut self, func: &mut ir::Function, index: GlobalIndex) -> GlobalVariable {
        unimplemented!();
    }

    fn make_heap(&mut self, func: &mut ir::Function, index: MemoryIndex) -> ir::Heap {
        unimplemented!();
    }

    fn make_table(&mut self, func: &mut ir::Function, index: TableIndex) -> ir::Table {
        unimplemented!();
    }

    fn make_indirect_sig(&mut self, func: &mut ir::Function, index: SignatureIndex) -> ir::SigRef {
        unimplemented!();
    }

    fn make_direct_func(&mut self, func: &mut ir::Function, index: FuncIndex) -> ir::FuncRef {
        unimplemented!();
    }

    fn translate_call_indirect(
        &mut self,
        pos: FuncCursor,
        table_index: TableIndex,
        table: ir::Table,
        sig_index: SignatureIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        unimplemented!();
    }

    fn translate_memory_grow(
        &mut self,
        pos: FuncCursor,
        index: MemoryIndex,
        heap: ir::Heap,
        val: ir::Value,
    ) -> WasmResult<ir::Value> {
        unimplemented!();
    }

    fn translate_memory_size(
        &mut self,
        pos: FuncCursor,
        index: MemoryIndex,
        heap: ir::Heap,
    ) -> WasmResult<ir::Value> {
        unimplemented!();
    }
}
