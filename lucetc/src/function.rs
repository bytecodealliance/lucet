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
    FuncEnvironment, FuncIndex, FuncTranslationState, GlobalIndex, GlobalVariable, MemoryIndex,
    SignatureIndex, TableIndex, WasmError, WasmResult,
};
use lucet_module::InstanceRuntimeData;
use memoffset::offset_of;
use std::collections::HashMap;
use wasmparser::Operator;

pub struct FuncInfo<'a> {
    module_decls: &'a ModuleDecls<'a>,
    count_instructions: bool,
    scope_costs: Vec<u32>,
    vmctx_value: Option<ir::GlobalValue>,
    global_base_value: Option<ir::GlobalValue>,
    runtime_funcs: HashMap<RuntimeFunc, ir::FuncRef>,
}

impl<'a> FuncInfo<'a> {
    pub fn new(module_decls: &'a ModuleDecls<'a>, count_instructions: bool) -> Self {
        Self {
            module_decls,
            count_instructions,
            scope_costs: vec![0],
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

    fn update_instruction_count_instrumentation(
        &mut self,
        op: &Operator,
        builder: &mut FunctionBuilder,
        reachable: bool,
    ) -> WasmResult<()> {
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
            if environ.scope_costs.last() == Some(&0) {
                return;
            }
            let instr_count_offset: ir::immediates::Offset32 =
                (-(std::mem::size_of::<InstanceRuntimeData>() as i32)
                    + offset_of!(InstanceRuntimeData, instruction_count) as i32)
                    .into();
            let vmctx_gv = environ.get_vmctx(builder.func);
            let addr = builder.ins().global_value(environ.pointer_type(), vmctx_gv);
            let trusted_mem = ir::MemFlags::trusted();

            //    Now insert a sequence of clif that is, functionally:
            //
            //    let instruction_count_ptr: &mut u64 = vmctx.instruction_count;
            //    let mut instruction_count: u64 = *instruction_count_ptr;
            //    instruction_count += <counter>;
            //    *instruction_count_ptr = instruction_count;

            let cur_instr_count =
                builder
                    .ins()
                    .load(ir::types::I64, trusted_mem, addr, instr_count_offset);
            let update_const = builder.ins().iconst(
                ir::types::I64,
                i64::from(*environ.scope_costs.last().unwrap()),
            );
            let new_instr_count = builder.ins().iadd(cur_instr_count, update_const.into());
            builder
                .ins()
                .store(trusted_mem, new_instr_count, addr, instr_count_offset);

            *environ.scope_costs.last_mut().unwrap() = 0;
        };

        // Only update or flush the counter when the scope is not sealed.
        //
        // Cranelift dutifully translates the entire wasm body, including dead code, and we can try
        // to insert instrumentation for dead code, but Cranelift seals blocks at operations that
        // involve control flow away from the current block. So we have to track when operations
        // are unreachable and not instrument them, lest we cause a Cranelift panic trying to
        // modify sealed basic blocks.
        if reachable {
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
            self.scope_costs.last_mut().map(|x| *x += op_cost);

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
                    // finished, and attempting to add instructions to update the instruction counter
                    // will cause a panic.
                    //
                    // The only situation where this can occur is if the last structure in a scope is a
                    // subscope (the body of a Loop, If, or Else), so we flush the counter entering
                    // those structures, and guarantee the `End` for their enclosing scope will have a
                    // counter value of 0. In other cases, we're not at risk of closing a scope leading
                    // to closing another scope, and it's safe to flush the counter.
                    //
                    // An example to help:
                    // ```
                    // block
                    //   i32.const 4   ; counter += 1
                    //   i32.const -5  ; counter += 1
                    //   i32.add       ; counter += 1
                    //   block         ; flush counter (counter = 3 -> 0), flush here to avoid having
                    //                 ;                                   accumulated count at the
                    //                 ;                                   final `end`
                    //     i32.const 4 ; counter += 1
                    //     i32.add     ; counter += 1
                    //   end           ; flush counter (counter = 2 -> 0)
                    // end             ; flush counter (counter = 0 -> 0) and is a no-op
                    // ```
                    flush_counter(self, builder);
                }
                _ => { /* regular operation, do nothing */ }
            }
        } else {
            // just a consistency check - the counter must be 0 when exiting a region of
            // unreachable code. If this assertion fails it means we either counted instructions
            // we shouldn't (because they're unreachable), or we didn't flush the counter before
            // starting to also instrument unreachable instructions (and would have tried to
            // overcount)
            assert_eq!(*self.scope_costs.last().unwrap(), 0);
        }

        // finally, we might have to set up a new counter for a new scope, or fix up counts a bit.
        //
        // Note that nothing is required for `Else`, because it will have been preceded by an `End`
        // to close the "then" arm of its enclosing `If`, so the counter will have already been
        // flushed and reset to 0.
        match op {
            Operator::CallIndirect { .. } | Operator::Call { .. } => {
                // only track the expected return if this call was reachable - if the call is not
                // reachable, the "called" function won't return!
                if reachable {
                    // add 1 to count the return from the called function
                    self.scope_costs.last_mut().map(|x| *x = 1);
                }
            }
            Operator::Block { .. } | Operator::Loop { .. } | Operator::If { .. } => {
                // opening a scope, which starts having executed zero wasm ops
                self.scope_costs.push(0);
            }
            Operator::End => {
                // close the top scope
                self.scope_costs.pop();
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
        state: &FuncTranslationState,
    ) -> WasmResult<()> {
        if self.count_instructions {
            self.update_instruction_count_instrumentation(op, builder, state.reachable())?;
        }
        Ok(())
    }
}
