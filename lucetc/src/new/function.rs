use super::runtime::RuntimeFunc;
use crate::compiler::entity::POINTER_SIZE;
use crate::new::decls::ModuleDecls;
use crate::program::memory::HeapSpec;
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_wasm::{
    FuncEnvironment, FuncIndex, GlobalIndex, GlobalVariable, MemoryIndex, ModuleEnvironment,
    SignatureIndex, TableIndex, WasmResult,
};
use std::collections::HashMap;

// VMContext points directly to the heap (offset 0).
// Directly before the heao is a pointer to the globals (offset -POINTER_SIZE).
const GLOBAL_BASE_OFFSET: i32 = -1 * POINTER_SIZE as i32;

pub struct FuncInfo<'a> {
    module_decls: &'a ModuleDecls<'a>,
    heap_spec: &'a HeapSpec,
    vmctx_value: Option<ir::GlobalValue>,
    global_base_value: Option<ir::GlobalValue>,
    runtime_funcs: HashMap<RuntimeFunc, ir::FuncRef>,
}

impl<'a> FuncInfo<'a> {
    pub fn new(module_decls: &'a ModuleDecls<'a>, heap_spec: &'a HeapSpec) -> Self {
        Self {
            module_decls,
            heap_spec,
            vmctx_value: None,
            global_base_value: None,
            runtime_funcs: HashMap::new(),
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

    pub fn get_runtime_func(
        &mut self,
        runtime_func: RuntimeFunc,
        func: &mut ir::Function,
    ) -> ir::FuncRef {
        self.runtime_funcs
            .get(&runtime_func)
            .cloned()
            .unwrap_or_else(|| {
                let decl = self
                    .module_decls
                    .get_runtime(runtime_func)
                    .expect("runtime function not available");
                let signature = func.import_signature(decl.signature.clone());
                let fref = func.import_function(ir::ExtFuncData {
                    name: decl.name.into(),
                    signature,
                    colocated: false,
                });
                self.runtime_funcs.insert(runtime_func, fref);
                fref
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
        let vmctx = self.get_vmctx(func);
        func.create_heap(ir::HeapData {
            base: vmctx,
            min_size: self.heap_spec.initial_size.into(),
            offset_guard_size: self.heap_spec.guard_size.into(),
            style: ir::HeapStyle::Static {
                bound: self.heap_spec.reserved_size.into(),
            },
            index_type: ir::types::I32,
        })
    }

    fn make_table(&mut self, func: &mut ir::Function, index: TableIndex) -> ir::Table {
        let table_decl = self.module_decls.get_table(index);
        // TODO: figure out how our statically defined tables
        // fit into this scheme. the legalizer uses these on the
        // `table_addr` instruction, which our version of this
        // code does not even emit.
        let base_gv = unimplemented!();
        let bound_gv = unimplemented!();
        let element_size = unimplemented!();
        let index_type = unimplemented!();
        let min_size = unimplemented!();
        func.create_table(ir::TableData {
            base_gv,
            bound_gv,
            element_size,
            index_type,
            min_size,
        })
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
        unimplemented!()
    }

    fn make_indirect_sig(&mut self, func: &mut ir::Function, index: SignatureIndex) -> ir::SigRef {
        let sig = self.module_decls.get_signature(index).unwrap().clone();
        func.import_signature(sig)
    }

    fn make_direct_func(&mut self, func: &mut ir::Function, index: FuncIndex) -> ir::FuncRef {
        let func_decl = self.module_decls.get_func(index).unwrap();
        let signature = func.import_signature(func_decl.signature.clone());
        let colocated = !func_decl.imported();
        func.import_function(ir::ExtFuncData {
            name: func_decl.name.into(),
            signature,
            colocated,
        })
    }

    fn translate_memory_grow(
        &mut self,
        mut pos: FuncCursor<'_>,
        _index: MemoryIndex,
        _heap: ir::Heap,
        val: ir::Value,
    ) -> WasmResult<ir::Value> {
        // TODO memory grow functions dont take heap index as argument
        let mem_grow_func = self.get_runtime_func(RuntimeFunc::MemGrow, &mut pos.func);
        let vmctx = pos.func.special_param(ir::ArgumentPurpose::VMContext).unwrap();
        let inst = pos.ins().call(mem_grow_func, &[vmctx, val]);
        Ok(*pos.func.dfg.inst_results(inst).first().unwrap())
    }

    fn translate_memory_size(
        &mut self,
        mut pos: FuncCursor,
        index: MemoryIndex,
        _heap: ir::Heap,
    ) -> WasmResult<ir::Value> {
        let mem_size_func = self.get_runtime_func(RuntimeFunc::MemSize, &mut pos.func);
        let vmctx = pos.func.special_param(ir::ArgumentPurpose::VMContext).unwrap();
        let inst = pos.ins().call(mem_size_func, &[vmctx]);
        Ok(*pos.func.dfg.inst_results(inst).first().unwrap())
    }
}
