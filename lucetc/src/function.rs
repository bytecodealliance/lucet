use super::runtime::RuntimeFunc;
use crate::decls::ModuleDecls;
use crate::pointer::{NATIVE_POINTER, NATIVE_POINTER_SIZE};
use crate::table::TABLE_REF_SIZE;
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::entity::EntityRef;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_frontend::FunctionBuilder;
use cranelift_wasm::{
    FuncEnvironment, FuncIndex, GlobalIndex, GlobalVariable, MemoryIndex, SignatureIndex,
    TableIndex, WasmError, WasmResult,
};
use lucet_module::InstanceRuntimeData;
use memoffset::offset_of;
use std::collections::HashMap;
use wasmparser::Operator;

pub struct FuncInfo<'a> {
    module_decls: &'a ModuleDecls<'a>,
    count_instructions: bool,
    op_offsets: Vec<u32>,
    vmctx_value: Option<ir::GlobalValue>,
    global_base_value: Option<ir::GlobalValue>,
    runtime_funcs: HashMap<RuntimeFunc, ir::FuncRef>,
}

impl<'a> FuncInfo<'a> {
    pub fn new(module_decls: &'a ModuleDecls<'a>, count_instructions: bool) -> Self {
        Self {
            module_decls,
            count_instructions,
            op_offsets: vec![0],
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
                offset: (-(std::mem::size_of::<InstanceRuntimeData>() as i32)
                    + (offset_of!(InstanceRuntimeData, globals_ptr) as i32))
                    .into(),
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

    fn update_instruction_count_instrumentation(&mut self, op: &Operator, builder: &mut FunctionBuilder, state: &cranelift_wasm::TranslationState) -> WasmResult<()> {
        // So the operation counting works like this:
        // record a stack corresponding with the stack of control flow in the wasm function.
        // for non-control-flow-affecting instructions, increment the top of the stack.
        // for control-flow-affecting operations (If, Else, Unreachable, Call, End, Return, Block,
        // Loop, BrIf, CallIndirect), update the wasm instruction counter and:
        // * if the operation introduces a new scope (If, Block, Loop), push a new 0 on the
        // stack corresponding with that frame.
        // * if the operation does not introduce a new scope (Else, Call, CallIndirect, BrIf),
        // reset the top of stack to 0
        // * if the operation completes a scope (End), pop the top of the stack and reset the new
        // top of stack to 0
        // * this leaves no special behavior for Unreachable and Return. This is acceptable as they
        // are always followed by an End and are either about to trap, or return from a function.
        // * Unreachable is either the end of VM execution and we are off by one instruction, or,
        // is about to dispatch to an exception handler, which we should account for out of band
        // anyway (exception dispatch is much more expensive than a single wasm op)
        // * Return corresponds to exactly one function call, so we can count it by resetting the
        // stack to 1 at return of a function.

        fn flush_counter(environ: &mut FuncInfo, builder: &mut FunctionBuilder) {
            if environ.op_offsets.last() == Some(&0) {
                return;
            }
            let instr_count_offset: ir::immediates::Offset32 =
                (-(std::mem::size_of::<InstanceRuntimeData>() as i32)
                    + offset_of!(InstanceRuntimeData, instruction_count) as i32)
                    .into();
            let vmctx_gv = environ.get_vmctx(builder.func);
            let addr = builder.ins().global_value(environ.pointer_type(), vmctx_gv);
            let flags = ir::MemFlags::trusted();
            let cur_instr_count =
                builder
                    .ins()
                    .load(ir::types::I64, flags, addr, instr_count_offset);
            let update_const = builder.ins().iconst(
                ir::types::I64,
                i64::from(*environ.op_offsets.last().unwrap()),
            );
            let new_instr_count = builder.ins().iadd(cur_instr_count, update_const.into());
            builder
                .ins()
                .store(flags, new_instr_count, addr, instr_count_offset);
            *environ.op_offsets.last_mut().unwrap() = 0;
        };

        // Update the instruction counter, if necessary
        let op_cost = match op {
            // Opening a scope is a syntactic operation, and free.
            Operator::Block { .. } |
            // These do not add counts, see above comment about return/unreachable
            Operator::Unreachable |
            Operator::Return => 0,
            // Call is quick
            Operator::Call { .. } => 1,
            // but indirect calls take some extra work to validate at runtime
            Operator::CallIndirect { .. } => 2,
            // Testing for an if involve some overhead, for now say it's also 1
            Operator::If { .. } => 1,
            // Else is a fallthrough or alternate case for something that's been tested as `if`, so
            // it's already counted
            Operator::Else => 0,
            // Entering a loop is a syntactic operation, and free.
            Operator::Loop { .. } => 0,
            // Closing a scope is a syntactic operation, and free.
            Operator::End => 0,
            // Taking a branch is an operation
            Operator::Br { .. } => 1,
            // brif might be two operations?
            Operator::BrIf { .. } => 1,
            // brtable is kind of cpu intensive compared to other wasm ops
            Operator::BrTable { .. } => 2,
            // nop and drop are free
            Operator::Nop |
            Operator::Drop => 0,
            // everything else, just call it one operation.
            _ => 1,
        };
        self.op_offsets.last_mut().map(|x| *x += op_cost);

        // apply flushing behavior if applicable
        match op {
            Operator::Unreachable
            | Operator::Return
            | Operator::CallIndirect { .. }
            | Operator::Call { .. }
            | Operator::Block { .. }
            | Operator::Loop { .. }
            | Operator::If { .. }
            | Operator::Else
            | Operator::Br { .. }
            | Operator::BrIf { .. }
            | Operator::BrTable { .. } => {
                flush_counter(self, builder);
            }
            Operator::End => {
                // We have to be really careful here to avoid violating a cranelift invariant:
                // if the next operation is `End` as well, this end will have marked the block
                // finished, and attempting to add instruction to update the instruction counter
                // will cause a panic.
                //
                // We can avoid that case by ensuring instruction counts are flushed at the *entry*
                // of any block-opening operation, so that at the exit the `End` will update the
                // count by 0, the update is discarded, and we don't cause a panic.
                flush_counter(self, builder);
            }
            _ => { /* regular operation, do nothing */ }
        }

        // finally, we might have to set up a new counter for a new scope, or fix up counts a bit
        match op {
            Operator::CallIndirect { .. } | Operator::Call { .. } => {
                // add 1 to count the return from the called function
                self.op_offsets.last_mut().map(|x| *x = 1);
            }
            Operator::Block { .. } | Operator::Loop { .. } | Operator::If { .. } => {
                // open a new scope
                self.op_offsets.push(0);
            }
            Operator::End => {
                // close the current scope
                self.op_offsets.pop();
            }
            _ => {}
        }
        Ok(())
    }
}

impl<'a> FuncEnvironment for FuncInfo<'a> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.module_decls.target_config()
    }

    fn make_global(
        &mut self,
        func: &mut ir::Function,
        index: GlobalIndex,
    ) -> Result<GlobalVariable, WasmError> {
        let global_base = self.get_global_base(func);
        let global = self.module_decls.get_global(index).expect("valid global");
        let index = index.as_u32() as i32;
        let offset = (index * NATIVE_POINTER_SIZE as i32).into();
        Ok(GlobalVariable::Memory {
            gv: global_base,
            offset,
            ty: global.entity.ty,
        })
    }

    fn make_heap(
        &mut self,
        func: &mut ir::Function,
        index: MemoryIndex,
    ) -> Result<ir::Heap, WasmError> {
        assert_eq!(index, MemoryIndex::new(0), "only memory 0 is supported");
        let heap_spec = self.module_decls.get_heap().expect("valid heap");
        let vmctx = self.get_vmctx(func);
        Ok(func.create_heap(ir::HeapData {
            base: vmctx,
            min_size: heap_spec.initial_size.into(),
            offset_guard_size: heap_spec.guard_size.into(),
            style: ir::HeapStyle::Static {
                bound: heap_spec.reserved_size.into(),
            },
            index_type: ir::types::I32,
        }))
    }

    fn make_table(
        &mut self,
        func: &mut ir::Function,
        index: TableIndex,
    ) -> Result<ir::Table, WasmError> {
        let index_type = ir::types::I64;
        let table_decl = self.module_decls.get_table(index).expect("valid table");
        let base_gv = func.create_global_value(ir::GlobalValueData::Symbol {
            name: table_decl.contents_name.into(),
            offset: 0.into(),
            colocated: true,
        });
        let tables_list_gv = func.create_global_value(ir::GlobalValueData::Symbol {
            name: self.module_decls.get_tables_list_name().as_externalname(),
            offset: 0.into(),
            colocated: true,
        });

        let table_bound_offset = (TABLE_REF_SIZE as u32)
            .checked_mul(index.as_u32())
            .and_then(|entry| entry.checked_add(NATIVE_POINTER_SIZE as u32))
            .ok_or(WasmError::ImplLimitExceeded)?;

        if table_bound_offset > std::i32::MAX as u32 {
            return Err(WasmError::ImplLimitExceeded);
        }

        let bound_gv = func.create_global_value(ir::GlobalValueData::Load {
            base: tables_list_gv,
            global_type: index_type,
            offset: (table_bound_offset as i32).into(),
            readonly: true,
        });
        let element_size = ((NATIVE_POINTER_SIZE * 2) as u64).into();
        let min_size = (table_decl.table.minimum as u64).into();
        Ok(func.create_table(ir::TableData {
            base_gv,
            bound_gv,
            element_size,
            index_type,
            min_size,
        }))
    }

    fn translate_call_indirect(
        &mut self,
        mut pos: FuncCursor<'_>,
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

    fn make_indirect_sig(
        &mut self,
        func: &mut ir::Function,
        index: SignatureIndex,
    ) -> Result<ir::SigRef, WasmError> {
        let sig = self.module_decls.get_signature(index).unwrap().clone();
        Ok(func.import_signature(sig))
    }

    fn make_direct_func(
        &mut self,
        func: &mut ir::Function,
        index: FuncIndex,
    ) -> Result<ir::FuncRef, WasmError> {
        let unique_index = *self
            .module_decls
            .info
            .function_mapping
            .get(index)
            .expect("function indices are valid");
        let func_decl = self.module_decls.get_func(unique_index).unwrap();
        let signature = func.import_signature(func_decl.signature.clone());
        let colocated = !func_decl.imported();
        Ok(func.import_function(ir::ExtFuncData {
            name: func_decl.name.into(),
            signature,
            colocated,
        }))
    }

    fn translate_call(
        &mut self,
        mut pos: FuncCursor<'_>,
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
        mut pos: FuncCursor<'_>,
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

    fn before_translate_operator(
        &mut self,
        op: &Operator,
        builder: &mut FunctionBuilder,
        state: &cranelift_wasm::TranslationState,
    ) -> WasmResult<()> {
        if self.count_instructions {
            self.update_instruction_count_instrumentation(op, builder, state)?;
        }
        Ok(())
    }
}
