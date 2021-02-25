use super::runtime::RuntimeFunc;
use crate::compiler::CodegenContext;
use crate::decls::{FunctionDecl, ModuleDecls};
use crate::module::UniqueFuncIndex;
use crate::pointer::{NATIVE_POINTER, NATIVE_POINTER_SIZE};
use crate::table::TABLE_REF_SIZE;
use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::entity::EntityRef;
use cranelift_codegen::ir::{self, condcodes::IntCC, InstBuilder};
use cranelift_codegen::isa::TargetFrontendConfig;
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::{Linkage, Module as ClifModule, ModuleError as ClifModuleError};
use cranelift_wasm::{
    wasmparser::Operator, FuncEnvironment, FuncIndex, FuncTranslationState, GlobalIndex,
    GlobalVariable, MemoryIndex, TableIndex, TargetEnvironment, TypeIndex, WasmError, WasmResult,
};
use lucet_module::InstanceRuntimeData;
use memoffset::offset_of;
use std::collections::HashMap;

pub struct FuncInfo<'a> {
    module_decls: &'a ModuleDecls<'a>,
    codegen_context: &'a CodegenContext,
    count_instructions: bool,
    scope_costs: Vec<ScopeInfo>,
    vmctx_value: Option<ir::GlobalValue>,
    global_base_value: Option<ir::GlobalValue>,
    runtime_funcs: HashMap<RuntimeFunc, ir::FuncRef>,
    instr_count_var: Variable,
}

struct ScopeInfo {
    cost: u32,
    is_loop: bool,
}

impl<'a> FuncInfo<'a> {
    pub fn new(
        module_decls: &'a ModuleDecls<'a>,
        codegen_context: &'a CodegenContext,
        count_instructions: bool,
        arg_count: u32,
        local_count: u32,
    ) -> Self {
        Self {
            module_decls,
            codegen_context,
            count_instructions,
            scope_costs: vec![ScopeInfo {
                cost: 0,
                is_loop: false,
            }],
            vmctx_value: None,
            global_base_value: None,
            runtime_funcs: HashMap::new(),
            // variable indices correspond to Wasm bytecode's index space,
            // so we designate a new one after all the Wasm locals to hold
            // the instruction count.
            instr_count_var: Variable::with_u32(arg_count + local_count),
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
        let decls = &self.module_decls;
        *self.runtime_funcs.entry(runtime_func).or_insert_with(|| {
            let decl = decls
                .get_runtime(runtime_func)
                .expect("runtime function not available");
            let signature = func.import_signature(decl.signature().to_owned());
            let fref = func.import_function(ir::ExtFuncData {
                name: decl.name.into(),
                signature,
                colocated: false,
            });
            fref
        })
    }

    fn get_instr_count_addr_offset(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
    ) -> (ir::Value, ir::immediates::Offset32) {
        let instr_count_offset: ir::immediates::Offset32 =
            (-(std::mem::size_of::<InstanceRuntimeData>() as i32)
                + offset_of!(InstanceRuntimeData, instruction_count_adj) as i32)
                .into();
        let vmctx_gv = self.get_vmctx(builder.func);
        let addr = builder.ins().global_value(self.pointer_type(), vmctx_gv);
        (addr, instr_count_offset)
    }

    fn load_instr_count(&mut self, builder: &mut FunctionBuilder<'_>) {
        let (addr, instr_count_offset) = self.get_instr_count_addr_offset(builder);
        let trusted_mem = ir::MemFlags::trusted();

        // Do the equivalent of:
        //
        //    let instruction_count_ptr: &mut i64 = vmctx.instruction_count;
        //    let instruction_count: i64 = *instruction_count_ptr;
        //    vars[instr_count] = instruction_count;
        let cur_instr_count =
            builder
                .ins()
                .load(ir::types::I64, trusted_mem, addr, instr_count_offset);
        builder.def_var(self.instr_count_var, cur_instr_count);
    }

    fn save_instr_count(&mut self, builder: &mut FunctionBuilder<'_>) {
        let (addr, instr_count_offset) = self.get_instr_count_addr_offset(builder);
        let trusted_mem = ir::MemFlags::trusted();

        // Do the equivalent of:
        //
        //    let instruction_count_ptr: &mut i64 = vmctx.instruction_count;
        //    let instruction_count = vars[instr_count];
        //    *instruction_count_ptr = instruction_count;
        let new_instr_count = builder.use_var(self.instr_count_var);
        builder
            .ins()
            .store(trusted_mem, new_instr_count, addr, instr_count_offset);
    }

    fn update_instruction_count_instrumentation_pre(
        &mut self,
        op: &Operator<'_>,
        builder: &mut FunctionBuilder<'_>,
        reachable: bool,
    ) -> WasmResult<()> {
        // We count Wasm instructions, and bound runtime, using two counters in the
        // `InstanceRuntimeData`: `instruction_count_bound` and `instruction_count_adj`. The sum of
        // these two gives the instruction count, but we only increment the `adj` counter in
        // generated code. When the sign of this counter flips from negative to positive, we have
        // exceeded the bound and we must yield, which we do by invoking a particular hostcall.
        //
        // The operation counting works like this:
        // - Record a stack corresponding with the stack of control flow in the wasm function.
        //   for non-control-flow-affecting instructions, increment the top of the stack.
        //   The sum of all stack elements represents instructions we have counted but not
        //   yet flushed to the counter.
        // - For control-flow-affecting operations (If, Else, Unreachable, Call, End, Return, Block,
        //   Loop, BrIf, CallIndirect), update the wasm instruction counter and:
        //   * if the operation introduces a new scope (If, Block, Loop), push a new 0 on the
        //     stack corresponding with that frame.
        //   * if the operation does not introduce a new scope (Else, Call, CallIndirect, BrIf),
        //     reset the top of stack to 0 (because we will have flushed to the counter).
        //   * if the operation completes a scope (End), pop the top of the stack and reset the new
        //     top of stack to 0
        //   * this leaves no special behavior for Unreachable and Return. This is acceptable as they
        //     are always followed by an End and are either about to trap, or return from a function.
        //   * Unreachable is either the end of VM execution and we are off by one instruction, or,
        //     is about to dispatch to an exception handler, which we should account for out of band
        //     anyway (exception dispatch is much more expensive than a single wasm op)
        //   * Return corresponds to exactly one function call, so we can count it by resetting the
        //     stack to 1 at return of a function.
        //
        // We keep a cache of the counter in the pinned register. We load it in the prologue, save
        // it in the epilogue, and save and reload it around calls. (We could alter our ABI to
        // preserve the pinned reg across calls within Wasm, and save and reload it only around
        // hostcalls, as long as we could load and save it in a trampoline wrapping the initial
        // Wasm entry. We haven't yet done this.)

        /// Flush the currently-accumulated instruction count to the counter in the instance data,
        /// invoking the yield hostcall if we hit the bound.
        fn flush_counter(environ: &mut FuncInfo<'_>, builder: &mut FunctionBuilder<'_>) {
            match environ.scope_costs.last() {
                Some(info) if info.cost == 0 => return,
                _ => {}
            }

            //    Now insert a sequence of clif that is, functionally:
            //
            //    let mut instruction_count = vars[instr_count];
            //    instruction_count += <counter>;
            //    vars[instr_count] = instruction_count;

            let cur_instr_count = builder.use_var(environ.instr_count_var);
            let update_const = builder.ins().iconst(
                ir::types::I64,
                i64::from(environ.scope_costs.last().unwrap().cost),
            );
            let new_instr_count = builder.ins().iadd(cur_instr_count, update_const);
            builder.def_var(environ.instr_count_var, new_instr_count);
            environ.scope_costs.last_mut().unwrap().cost = 0;
        };

        fn do_check(environ: &mut FuncInfo<'_>, builder: &mut FunctionBuilder<'_>) {
            let yield_block = builder.create_block();
            let continuation_block = builder.create_block();
            // If `adj` is positive, branch to yield block.
            let zero = builder.ins().iconst(ir::types::I64, 0);
            let new_instr_count = builder.use_var(environ.instr_count_var);
            let cmp = builder.ins().ifcmp(new_instr_count, zero);
            builder
                .ins()
                .brif(IntCC::SignedGreaterThanOrEqual, cmp, yield_block, &[]);
            builder.ins().jump(continuation_block, &[]);
            builder.seal_block(yield_block);

            builder.switch_to_block(yield_block);
            environ.save_instr_count(builder);
            let yield_hostcall =
                environ.get_runtime_func(RuntimeFunc::YieldAtBoundExpiration, &mut builder.func);
            let vmctx_gv = environ.get_vmctx(builder.func);
            let addr = builder.ins().global_value(environ.pointer_type(), vmctx_gv);
            builder.ins().call(yield_hostcall, &[addr]);
            environ.load_instr_count(builder);
            builder.ins().jump(continuation_block, &[]);
            builder.seal_block(continuation_block);

            builder.switch_to_block(continuation_block);
        }

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
            self.scope_costs
                .last_mut()
                .map(|ref mut info| info.cost += op_cost);

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
                    let do_check_and_save = match op {
                        Operator::Call { .. }
                        | Operator::CallIndirect { .. }
                        | Operator::Return
                        | Operator::BrTable { .. } => true,
                        Operator::Br { relative_depth } | Operator::BrIf { relative_depth } => {
                            // only if loop backedge
                            self.scope_costs[self.scope_costs.len() - 1 - *relative_depth as usize]
                                .is_loop
                        }
                        _ => false,
                    };
                    flush_counter(self, builder);
                    if do_check_and_save {
                        self.save_instr_count(builder);
                        do_check(self, builder);
                    }
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
            assert_eq!(self.scope_costs.last().unwrap().cost, 0);
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
                    self.scope_costs
                        .last_mut()
                        .map(|ref mut info| info.cost += 1);
                }
            }
            Operator::Block { .. } | Operator::Loop { .. } | Operator::If { .. } => {
                // opening a scope, which starts having executed zero wasm ops
                let is_loop = match op {
                    Operator::Loop { .. } => true,
                    _ => false,
                };
                self.scope_costs.push(ScopeInfo { cost: 0, is_loop });
            }
            Operator::End => {
                // close the top scope
                self.scope_costs.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn update_instruction_count_instrumentation_post(
        &mut self,
        op: &Operator<'_>,
        builder: &mut FunctionBuilder<'_>,
        reachable: bool,
    ) -> WasmResult<()> {
        // Handle reloads after calls.
        let is_call = match op {
            Operator::Call { .. } | Operator::CallIndirect { .. } => true,
            _ => false,
        };
        if reachable && is_call {
            self.load_instr_count(builder);
        }
        Ok(())
    }

    fn update_instruction_count_instrumentation_before_func(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
    ) -> WasmResult<()> {
        if self.count_instructions {
            builder.declare_var(self.instr_count_var, ir::types::I64);
            self.load_instr_count(builder);
        }
        Ok(())
    }

    fn update_instruction_count_instrumentation_after_func(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        reachable: bool,
    ) -> WasmResult<()> {
        if reachable {
            self.save_instr_count(builder);
        }
        Ok(())
    }
}

/// Get the local trampoline function to do safety checks before calling an imported hostcall.
fn get_trampoline_func(
    codegen_context: &CodegenContext,
    hostcall_index: UniqueFuncIndex,
    func_decl: &FunctionDecl,
    signature: &ir::Signature,
) -> Result<ir::ExternalName, ClifModuleError> {
    use std::collections::hash_map::Entry;
    let funcid = match codegen_context
        .trampolines()
        .entry(func_decl.name.symbol().to_string())
    {
        Entry::Occupied(o) => o.get().0,
        Entry::Vacant(v) => {
            let trampoline_name = format!("trampoline_{}", func_decl.name.symbol());

            let funcid = codegen_context.module().declare_function(
                &trampoline_name,
                Linkage::Local,
                signature,
            )?;
            v.insert((funcid, hostcall_index)).0
        }
    };

    Ok(ir::ExternalName::from(funcid))
}

impl<'a> TargetEnvironment for FuncInfo<'a> {
    fn target_config(&self) -> TargetFrontendConfig {
        self.module_decls.target_config()
    }
}

impl<'a> FuncEnvironment for FuncInfo<'a> {
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
            tls: false,
        });
        let tables_list_gv = func.create_global_value(ir::GlobalValueData::Symbol {
            name: self.module_decls.get_tables_list_name().as_externalname(),
            offset: 0.into(),
            colocated: true,
            tls: false,
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
        sig_index: TypeIndex,
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
        index: TypeIndex,
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

        // if we're setting up a function ref for a call to an imported function, we'll need a
        // trampoline to check the stack first. in that case, return the trampoline function
        // instead.
        let colocated = !func_decl.imported();
        let name = if colocated {
            func_decl.name.into()
        } else {
            get_trampoline_func(
                &self.codegen_context,
                unique_index,
                &func_decl,
                &func.dfg.signatures[signature],
            )
            .map_err(|err| WasmError::User(format!("{}", err)))?
        };
        Ok(func.import_function(ir::ExtFuncData {
            name,
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

    fn translate_memory_copy(
        &mut self,
        _pos: FuncCursor,
        _index: MemoryIndex,
        _heap: ir::Heap,
        _index2: MemoryIndex,
        _heap2: ir::Heap,
        _dst: ir::Value,
        _src: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Err(WasmError::Unsupported(
            "bulk memory operations not supported yet".into(),
        ))
    }

    fn translate_memory_fill(
        &mut self,
        _pos: FuncCursor,
        _index: MemoryIndex,
        _heap: ir::Heap,
        _dst: ir::Value,
        _val: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Err(WasmError::Unsupported(
            "bulk memory operations not supported yet".into(),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn translate_memory_init(
        &mut self,
        _pos: FuncCursor,
        _index: MemoryIndex,
        _heap: ir::Heap,
        _seg_index: u32,
        _dst: ir::Value,
        _src: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Err(WasmError::Unsupported(
            "bulk memory operations not supported yet".into(),
        ))
    }

    fn translate_data_drop(&mut self, _pos: FuncCursor, _seg_index: u32) -> WasmResult<()> {
        Err(WasmError::Unsupported(
            "bulk memory operations not supported yet".into(),
        ))
    }

    fn translate_table_size(
        &mut self,
        _pos: FuncCursor,
        _index: TableIndex,
        _table: ir::Table,
    ) -> WasmResult<ir::Value> {
        Err(WasmError::Unsupported(
            "reference type operations not supported yet".into(),
        ))
    }

    fn translate_table_grow(
        &mut self,
        _pos: FuncCursor,
        _table_index: TableIndex,
        _table: ir::Table,
        _delta: ir::Value,
        _init_value: ir::Value,
    ) -> WasmResult<ir::Value> {
        Err(WasmError::Unsupported(
            "reference type operations not supported yet".into(),
        ))
    }

    fn translate_table_get(
        &mut self,
        _func: &mut FunctionBuilder,
        _table_index: TableIndex,
        _table: ir::Table,
        _index: ir::Value,
    ) -> WasmResult<ir::Value> {
        Err(WasmError::Unsupported(
            "reference type operations not supported yet".into(),
        ))
    }

    fn translate_table_set(
        &mut self,
        _func: &mut FunctionBuilder,
        _table_index: TableIndex,
        _table: ir::Table,
        _value: ir::Value,
        _index: ir::Value,
    ) -> WasmResult<()> {
        Err(WasmError::Unsupported(
            "reference type operations not supported yet".into(),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn translate_table_copy(
        &mut self,
        _pos: FuncCursor,
        _dst_table_index: TableIndex,
        _dst_table: ir::Table,
        _src_table_index: TableIndex,
        _src_table: ir::Table,
        _dst: ir::Value,
        _src: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Err(WasmError::Unsupported(
            "reference type operations not supported yet".into(),
        ))
    }

    fn translate_table_fill(
        &mut self,
        _pos: FuncCursor,
        _table_index: TableIndex,
        _dst: ir::Value,
        _val: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Err(WasmError::Unsupported(
            "reference type operations not supported yet".into(),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn translate_table_init(
        &mut self,
        _pos: FuncCursor,
        _seg_index: u32,
        _table_index: TableIndex,
        _table: ir::Table,
        _dst: ir::Value,
        _src: ir::Value,
        _len: ir::Value,
    ) -> WasmResult<()> {
        Err(WasmError::Unsupported(
            "reference type operations not supported yet".into(),
        ))
    }

    fn translate_elem_drop(&mut self, _pos: FuncCursor, _seg_index: u32) -> WasmResult<()> {
        Err(WasmError::Unsupported(
            "bulk memory operations not supported yet".into(),
        ))
    }

    fn translate_ref_func(
        &mut self,
        _pos: FuncCursor,
        _func_index: FuncIndex,
    ) -> WasmResult<ir::Value> {
        Err(WasmError::Unsupported(
            "reference type operations not supported yet".into(),
        ))
    }

    fn translate_custom_global_get(
        &mut self,
        _pos: FuncCursor,
        _global_index: GlobalIndex,
    ) -> WasmResult<ir::Value> {
        Err(WasmError::Unsupported(
            "custom global operations not supported yet".into(),
        ))
    }

    fn translate_custom_global_set(
        &mut self,
        _pos: FuncCursor,
        _global_index: GlobalIndex,
        _val: ir::Value,
    ) -> WasmResult<()> {
        Err(WasmError::Unsupported(
            "custom global operations not supported yet".into(),
        ))
    }

    fn before_translate_operator(
        &mut self,
        op: &Operator<'_>,
        builder: &mut FunctionBuilder<'_>,
        state: &FuncTranslationState,
    ) -> WasmResult<()> {
        if self.count_instructions {
            self.update_instruction_count_instrumentation_pre(op, builder, state.reachable())?;
        }
        Ok(())
    }

    fn after_translate_operator(
        &mut self,
        op: &Operator<'_>,
        builder: &mut FunctionBuilder<'_>,
        state: &FuncTranslationState,
    ) -> WasmResult<()> {
        if self.count_instructions {
            self.update_instruction_count_instrumentation_post(op, builder, state.reachable())?;
        }
        Ok(())
    }

    fn before_translate_function(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        _state: &FuncTranslationState,
    ) -> WasmResult<()> {
        if self.count_instructions {
            self.update_instruction_count_instrumentation_before_func(builder)?;
        }
        Ok(())
    }

    fn after_translate_function(
        &mut self,
        builder: &mut FunctionBuilder<'_>,
        state: &FuncTranslationState,
    ) -> WasmResult<()> {
        if self.count_instructions {
            self.update_instruction_count_instrumentation_after_func(builder, state.reachable())?;
        }
        Ok(())
    }

    fn translate_atomic_wait(
        &mut self,
        _: FuncCursor,
        _: MemoryIndex,
        _: ir::Heap,
        _: ir::Value,
        _: ir::Value,
        _: ir::Value,
    ) -> WasmResult<ir::Value> {
        unimplemented!("translate atomic wait");
    }
    fn translate_atomic_notify(
        &mut self,
        _: FuncCursor,
        _: MemoryIndex,
        _: ir::Heap,
        _: ir::Value,
        _: ir::Value,
    ) -> WasmResult<ir::Value> {
        unimplemented!("translate atomic verify");
    }
}
