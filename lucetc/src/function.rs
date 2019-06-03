use super::runtime::RuntimeFunc;
use crate::decls::ModuleDecls;
use crate::pointer::{NATIVE_POINTER, NATIVE_POINTER_SIZE};
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::entity::EntityRef;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_wasm::{
    FuncEnvironment, FuncIndex, GlobalIndex, GlobalVariable, MemoryIndex, SignatureIndex,
    TableIndex, WasmResult,
};
use std::collections::HashMap;

// VMContext points directly to the heap (offset 0).
// Directly before the heao is a pointer to the globals (offset -NATIVE_POINTER_SIZE).
const GLOBAL_BASE_OFFSET: i32 = -1 * NATIVE_POINTER_SIZE as i32;

pub struct FuncInfo<'a> {
    module_decls: &'a ModuleDecls<'a>,
    vmctx_value: Option<ir::GlobalValue>,
    global_base_value: Option<ir::GlobalValue>,
    runtime_funcs: HashMap<RuntimeFunc, ir::FuncRef>,
}

impl<'a> FuncInfo<'a> {
    pub fn new(module_decls: &'a ModuleDecls<'a>) -> Self {
        Self {
            module_decls,
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
                let signature = func.import_signature(decl.signature().to_owned());
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
        let global_base = self.get_global_base(func);
        let global = self.module_decls.get_global(index).expect("valid global");
        let index = index.as_u32() as i32;
        let offset = (index * NATIVE_POINTER_SIZE as i32).into();
        GlobalVariable::Memory {
            gv: global_base,
            offset,
            ty: global.entity.ty,
        }
    }

    fn make_heap(&mut self, func: &mut ir::Function, index: MemoryIndex) -> ir::Heap {
        assert_eq!(index, MemoryIndex::new(0), "only memory 0 is supported");
        let heap_spec = self.module_decls.get_heap().expect("valid heap");
        let vmctx = self.get_vmctx(func);
        func.create_heap(ir::HeapData {
            base: vmctx,
            min_size: heap_spec.initial_size.into(),
            offset_guard_size: heap_spec.guard_size.into(),
            style: ir::HeapStyle::Static {
                bound: heap_spec.reserved_size.into(),
            },
            index_type: ir::types::I32,
        })
    }

    fn make_table(&mut self, func: &mut ir::Function, index: TableIndex) -> ir::Table {
        let index_type = ir::types::I64;
        let table_decl = self.module_decls.get_table(index).expect("valid table");
        let base_gv = func.create_global_value(ir::GlobalValueData::Symbol {
            name: table_decl.contents_name.into(),
            offset: 0.into(),
            colocated: true,
        });
        let bound_ptr_gv = func.create_global_value(ir::GlobalValueData::Symbol {
            name: table_decl.len_name.into(),
            offset: 0.into(),
            colocated: true,
        });
        let bound_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: bound_ptr_gv,
            global_type: index_type,
            offset: 0.into(),
            readonly: true,
        });
        let element_size = ((NATIVE_POINTER_SIZE * 2) as u64).into();
        let min_size = (table_decl.table.minimum as u64).into();
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
        mut pos: FuncCursor,
        _table_index: TableIndex,
        table: ir::Table,
        sig_index: SignatureIndex,
        sig_ref: ir::SigRef,
        callee: ir::Value,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        let callee_u64 = pos.ins().sextend(ir::types::I64, callee);
        let table_entry_addr = pos.ins().table_addr(ir::types::I64, table, callee_u64, 0);
        // First element at the table entry is the signature index of the function
        let table_entry_sig_offset = 0;
        let table_entry_sig_ix = pos.ins().load(
            ir::types::I64,
            ir::MemFlags::trusted(),
            table_entry_addr,
            table_entry_sig_offset,
        );

        // Translate from the module's non-unique signature space to our internal unique space
        let unique_sig_index = self
            .module_decls
            .get_signature_uid(sig_index)
            .expect("signature index must be valid");

        // Check it against the unique sig_index, trap if wrong
        let valid_type = pos.ins().icmp_imm(
            ir::condcodes::IntCC::Equal,
            table_entry_sig_ix,
            unique_sig_index.as_u32() as i64,
        );
        pos.ins().trapz(valid_type, ir::TrapCode::BadSignature);

        // Second element at the table entry is the function pointer
        let table_entry_fptr_offset = NATIVE_POINTER_SIZE as i32;
        let table_entry_fptr = pos.ins().load(
            NATIVE_POINTER,
            ir::MemFlags::trusted(),
            table_entry_addr,
            table_entry_fptr_offset,
        );

        let mut args: Vec<ir::Value> = Vec::with_capacity(call_args.len() + 1);
        args.extend_from_slice(call_args);
        args.insert(
            0,
            pos.func
                .special_param(ir::ArgumentPurpose::VMContext)
                .expect("vmctx available"),
        );

        Ok(pos.ins().call_indirect(sig_ref, table_entry_fptr, &args))
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

    fn translate_call(
        &mut self,
        mut pos: FuncCursor,
        _callee_index: FuncIndex,
        callee: ir::FuncRef,
        call_args: &[ir::Value],
    ) -> WasmResult<ir::Inst> {
        let mut args: Vec<ir::Value> = Vec::with_capacity(call_args.len() + 1);
        args.extend_from_slice(call_args);
        args.insert(
            0,
            pos.func
                .special_param(ir::ArgumentPurpose::VMContext)
                .expect("vmctx available"),
        );
        Ok(pos.ins().call(callee, &args))
    }

    fn translate_memory_grow(
        &mut self,
        mut pos: FuncCursor<'_>,
        index: MemoryIndex,
        _heap: ir::Heap,
        val: ir::Value,
    ) -> WasmResult<ir::Value> {
        assert!(index == MemoryIndex::new(0));
        // TODO memory grow function doesnt take heap index as argument
        let mem_grow_func = self.get_runtime_func(RuntimeFunc::MemGrow, &mut pos.func);
        let vmctx = pos
            .func
            .special_param(ir::ArgumentPurpose::VMContext)
            .unwrap();
        let inst = pos.ins().call(mem_grow_func, &[vmctx, val]);
        Ok(*pos.func.dfg.inst_results(inst).first().unwrap())
    }

    fn translate_memory_size(
        &mut self,
        mut pos: FuncCursor,
        index: MemoryIndex,
        _heap: ir::Heap,
    ) -> WasmResult<ir::Value> {
        assert!(index == MemoryIndex::new(0));
        // TODO memory size function doesnt take heap index as argument
        let mem_size_func = self.get_runtime_func(RuntimeFunc::MemSize, &mut pos.func);
        let vmctx = pos
            .func
            .special_param(ir::ArgumentPurpose::VMContext)
            .unwrap();
        let inst = pos.ins().call(mem_size_func, &[vmctx]);
        Ok(*pos.func.dfg.inst_results(inst).first().unwrap())
    }
}
