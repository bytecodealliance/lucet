use crate::compiler::entity::POINTER_SIZE;
use crate::new::decls::ModuleDecls;
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir;
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_wasm::{
    FuncEnvironment, FuncIndex, GlobalIndex, GlobalVariable, MemoryIndex, ModuleEnvironment,
    SignatureIndex, TableIndex, WasmResult,
};

// VMContext points directly to the heap (offset 0).
// Directly before the heao is a pointer to the globals (offset -POINTER_SIZE).
const GLOBAL_BASE_OFFSET: i32 = -1 * POINTER_SIZE as i32;

pub struct FuncInfo<'a> {
    module_decls: &'a ModuleDecls<'a>,
    vmctx_value: Option<ir::GlobalValue>,
    global_base_value: Option<ir::GlobalValue>,
}

impl<'a> FuncInfo<'a> {
    pub fn new(module_decls: &'a ModuleDecls<'a>) -> Self {
        Self {
            module_decls,
            vmctx_value: None,
            global_base_value: None,
        }
    }

    pub fn get_vmctx(&mut self, func: &mut ir::Function) -> ir::GlobalValue {
        self.vmctx_value.unwrap_or_else(|| {
            let vmctx_value = func.create_global_value(ir::GlobalValueData::VMContext);
            self.vmctx_value = Some(vmctx_value);
            vmctx_value
        })
    }

    pub fn get_global_base(&mut self, func: &mut ir::Function) -> ir::GlobalValue {
        self.global_base_value.unwrap_or_else(|| {
            let vmctx = self.get_vmctx(func);
            let global_base_value = func.create_global_value(ir::GlobalValueData::Load {
                base: vmctx,
                offset: GLOBAL_BASE_OFFSET.into(),
                global_type: ir::types::I64,
                readonly: false,
            });
            self.global_base_value = Some(global_base_value);
            global_base_value
        })
    }
}

impl<'a> FuncEnvironment for FuncInfo<'a> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.module_decls.target_config()
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
