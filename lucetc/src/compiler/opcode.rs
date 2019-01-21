// This file is derived from cranelift/lib/wasm/src/code_translator.rs
// at commit 5a952497b9144f8a9437a4fb3af73b03b5be8dde

use crate::compiler::entity::{EntityCreator, NATIVE_POINTER, POINTER_SIZE};
use crate::compiler::state::{ControlVariant, TranslationState};
use crate::compiler::Compiler;
use crate::program::types::cton_valuetype;
use crate::program::CtonSignature;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::types::{F32, F64, I32, I64};
use cranelift_codegen::ir::{self, InstBuilder, JumpTableData, MemFlags};
use cranelift_codegen::packed_option::ReservedValue;
use cranelift_frontend::{FunctionBuilder, Variable};
use failure::{format_err, Error};
use parity_wasm::elements::{BlockType, Instruction};
use std::{i32, u32};

pub fn translate_opcode(
    op: &Instruction,
    builder: &mut FunctionBuilder,
    state: &mut TranslationState,
    entity_creator: &mut EntityCreator,
    compiler: &Compiler,
) -> Result<(), Error> {
    if !state.reachable {
        return translate_unreachable_opcode(op, builder, state);
    }

    match *op {
        /********************************** Locals ****************************************
         *  `get_local` and `set_local` are treated as non-SSA variables and will completely
         *  diseappear in the Cretonne Code
         ***********************************************************************************/
        Instruction::GetLocal(local_index) => {
            state.push1(builder.use_var(Variable::with_u32(local_index)))
        }
        Instruction::SetLocal(local_index) => {
            let val = state.pop1();
            builder.def_var(Variable::with_u32(local_index), val);
        }
        Instruction::TeeLocal(local_index) => {
            let val = state.peek1();
            builder.def_var(Variable::with_u32(local_index), val);
        }

        /********************************** Globals ****************************************
         *  `get_global` and `set_global` are handled by the environment.
         ***********************************************************************************/
        Instruction::GetGlobal(global_index) => {
            let global = entity_creator.get_global(builder.func, global_index, compiler)?;
            let addr = builder.ins().global_value(NATIVE_POINTER, global.var);
            let flags = ir::MemFlags::new();
            let val = builder.ins().load(global.ty, flags, addr, 0);
            state.push1(val);
        }

        Instruction::SetGlobal(global_index) => {
            let global = entity_creator.get_global(builder.func, global_index, compiler)?;
            let addr = builder.ins().global_value(NATIVE_POINTER, global.var);
            let flags = ir::MemFlags::new();
            let val = state.pop1();
            builder.ins().store(flags, val, addr, 0);
        }

        /********************************* Stack misc ***************************************
         *  `drop`, `nop`, `select`.
         ***********************************************************************************/
        // Stack Misc
        Instruction::Nop => {}
        Instruction::Drop => {
            state.pop1();
        }
        Instruction::Select => {
            let (whentrue, whenfalse, cond) = state.pop3();
            state.push1(builder.ins().select(cond, whentrue, whenfalse));
        }

        /***************************** Control flow blocks **********************************
         *  When starting a control flow block, we create a new `Ebb` that will hold the code
         *  after the block, and we push a frame on the control stack. Depending on the type
         *  of block, we create a new `Ebb` for the body of the block with an associated
         *  jump instruction.
         *
         *  The `End` instruction pops the last control frame from the control stack, seals
         *  the destination block (since `br` instructions targeting it only appear inside the
         *  block and have already been translated) and modify the value stack to use the
         *  possible `Ebb`'s arguments values.
         ***********************************************************************************/
        Instruction::Unreachable => {
            // Generates a trap instr and sets state so translate_unreachable_opcode is
            // used until the control flow frame is complete.
            builder.ins().trap(ir::TrapCode::User(0));
            state.reachable = false;
        }
        // Create a new Ebb for the continuation, push frame onto control type.
        Instruction::Block(ty) => {
            let next = builder.create_ebb();
            if let Some(ty) = cton_blocktype(&ty) {
                builder.append_ebb_param(next, ty);
            }
            state.push_control_frame(ControlVariant::_block(), next, num_return_values(&ty));
        }
        Instruction::Loop(ty) => {
            let loop_body = builder.create_ebb();
            let next = builder.create_ebb();
            if let Some(ty) = cton_blocktype(&ty) {
                builder.append_ebb_param(next, ty);
            }
            builder.ins().jump(loop_body, &[]);
            state.push_control_frame(
                ControlVariant::_loop(loop_body),
                next,
                num_return_values(&ty),
            );
            builder.switch_to_block(loop_body);
        }
        Instruction::If(ty) => {
            let val = state.pop1();
            let if_not = builder.create_ebb();
            // No arguments to jump:
            // When If has no Else cuause, ty is EmptyBlock.
            // When there is an Else clause, this jump destination will be
            // overwritten, and the ultimate continuation will get the correct
            // arguments from the Else ebb.
            let branch_inst = builder.ins().brz(val, if_not, &[]);
            if let Some(ty) = cton_blocktype(&ty) {
                builder.append_ebb_param(if_not, ty);
            }
            let reachable = state.reachable;
            state.push_control_frame(
                ControlVariant::_if(branch_inst, reachable),
                if_not,
                num_return_values(&ty),
            );
        }

        Instruction::Else => {
            let last = state.control_stack.len() - 1;
            let (branch_inst, ref mut reachable_from_top) = match state.control_stack[last].variant
            {
                ControlVariant::If {
                    branch_inst,
                    reachable_from_top,
                    ..
                } => (branch_inst, reachable_from_top),
                _ => panic!("impossible: else instruction when not in `if`"),
            };
            // The if h as an else, so there's no branch to end from the top.
            *reachable_from_top = false;
            let retcnt = state.control_stack[last].num_return_values;
            let dest = state.control_stack[last].destination;
            builder.ins().jump(dest, state.peekn(retcnt));
            state.dropn(retcnt);
            // Retarget the If branch to the else block
            let else_ebb = builder.create_ebb();
            builder.change_jump_destination(branch_inst, else_ebb);
            builder.seal_block(else_ebb);
            builder.switch_to_block(else_ebb);
            // When the End of this else block is reached, it will terminate the
            // If just as above, to the destination.
        }

        Instruction::End => {
            let frame = state
                .control_stack
                .pop()
                .expect("end instruction has populated ctrl stack");
            let return_count = frame.num_return_values;
            if !builder.is_unreachable() || !builder.is_pristine() {
                builder
                    .ins()
                    .jump(frame.following_code(), state.peekn(return_count));
            }
            builder.switch_to_block(frame.following_code());
            builder.seal_block(frame.following_code());
            // Loop body needs to be sealed as well
            if let ControlVariant::Loop { body } = frame.variant {
                builder.seal_block(body);
            }
            state.stack.truncate(frame.original_stack_size);
            let following_params = builder.ebb_params(frame.following_code());
            state.stack.extend_from_slice(following_params);
        }
        /**************************** Branch instructions *********************************
         * The branch instructions all have as arguments a target nesting level, which
         * corresponds to how many control stack frames do we have to pop to get the
         * destination `Ebb`.
         *
         * Once the destination `Ebb` is found, we sometimes have to declare a certain depth
         * of the stack unreachable, because some branch instructions are terminator.
         *
         * The `br_table` case is much more complicated because Cretonne's `br_table` instruction
         * does not support jump arguments like all the other branch instructions. That is why, in
         * the case where we would use jump arguments for every other branch instructions, we
         * need to split the critical edges leaving the `br_tables` by creating one `Ebb` per
         * table destination; the `br_table` will point to these newly created `Ebbs` and these
         * `Ebb`s contain only a jump instruction pointing to the final destination, this time with
         * jump arguments.
         *
         * This system is also implemented in Cretonne's SSA construction algorithm, because
         * `use_var` located in a destination `Ebb` of a `br_table` might trigger the addition
         * of jump arguments in each predecessor branch instruction, one of which might be a
         * `br_table`.
         ***********************************************************************************/
        Instruction::Br(relative_depth) => {
            let i = state.control_stack.len() - 1 - (relative_depth as usize);
            let (return_count, br_destination) = {
                let frame = &mut state.control_stack[i];
                // We signal that all the code that follows until the next End is unreachable
                frame.set_branched_to_exit();
                let return_count = if frame.is_loop() {
                    0
                } else {
                    frame.num_return_values
                };
                (return_count, frame.br_destination())
            };
            builder
                .ins()
                .jump(br_destination, state.peekn(return_count));
            state.dropn(return_count);
            state.reachable = false;
        }
        Instruction::BrIf(relative_depth) => {
            let val = state.pop1();
            let i = state.control_stack.len() - 1 - (relative_depth as usize);
            let (return_count, br_destination) = {
                let frame = &mut state.control_stack[i];
                // The values returned by the branch are still available for the reachable
                // code that comes after it
                frame.set_branched_to_exit();
                let return_count = if frame.is_loop() {
                    0
                } else {
                    frame.num_return_values
                };
                (return_count, frame.br_destination())
            };
            builder
                .ins()
                .brnz(val, br_destination, state.peekn(return_count));
        }
        Instruction::BrTable(ref data_table) => {
            let depths = &data_table.table;
            let default = data_table.default;
            use std::collections::hash_map::{self, HashMap};
            let mut min_depth: u32 = default;
            for depth in depths.iter() {
                if *depth < min_depth {
                    min_depth = *depth;
                }
            }
            let jump_args_count = {
                let i = state.control_stack.len() - 1 - (min_depth as usize);
                let min_depth_frame = &state.control_stack[i];
                if min_depth_frame.is_loop() {
                    0
                } else {
                    min_depth_frame.num_return_values
                }
            };
            if jump_args_count == 0 {
                // No jump arguments
                let val = state.pop1();
                let mut data = JumpTableData::with_capacity(depths.len());
                for depth in depths.iter() {
                    let ebb = {
                        let i = state.control_stack.len() - 1 - (*depth as usize);
                        let frame = &mut state.control_stack[i];
                        frame.set_branched_to_exit();
                        frame.br_destination()
                    };
                    data.push_entry(ebb);
                }
                let jt = builder.create_jump_table(data);
                let ebb = {
                    let i = state.control_stack.len() - 1 - (default as usize);
                    let frame = &mut state.control_stack[i];
                    frame.set_branched_to_exit();
                    frame.br_destination()
                };
                builder.ins().br_table(val, ebb, jt);
            } else {
                // Here we have jump arguments, but Cretonne's br_table doesn't support them
                // We then proceed to split the edges going out of the br_table
                let val = state.pop1();
                let return_count = jump_args_count;
                let mut data = JumpTableData::with_capacity(depths.len());
                let mut dest_ebb_sequence = Vec::new();
                let mut dest_ebb_map = HashMap::new();
                for depth in depths.iter() {
                    let branch_ebb = match dest_ebb_map.entry(*depth as usize) {
                        hash_map::Entry::Occupied(entry) => *entry.get(),
                        hash_map::Entry::Vacant(entry) => {
                            let ebb = builder.create_ebb();
                            dest_ebb_sequence.push((*depth as usize, ebb));
                            *entry.insert(ebb)
                        }
                    };
                    data.push_entry(branch_ebb);
                }
                let jt = builder.create_jump_table(data);
                let default_ebb = {
                    let i = state.control_stack.len() - 1 - (default as usize);
                    let frame = &mut state.control_stack[i];
                    frame.set_branched_to_exit();
                    frame.br_destination()
                };
                dest_ebb_sequence.push((default as usize, default_ebb));
                builder.ins().br_table(val, default_ebb, jt);
                for (depth, dest_ebb) in dest_ebb_sequence {
                    builder.switch_to_block(dest_ebb);
                    builder.seal_block(dest_ebb);
                    let i = state.control_stack.len() - 1 - depth;
                    let real_dest_ebb = {
                        let frame = &mut state.control_stack[i];
                        frame.set_branched_to_exit();
                        frame.br_destination()
                    };
                    builder.ins().jump(real_dest_ebb, state.peekn(return_count));
                }
                state.dropn(return_count);
            }
            state.reachable = false;
        }
        Instruction::Return => {
            let (return_count, br_destination) = {
                let frame = &mut state.control_stack[0];
                frame.set_branched_to_exit();
                let return_count = frame.num_return_values;
                (return_count, frame.br_destination())
            };
            {
                let args = state.peekn(return_count);
                builder.ins().jump(br_destination, args);
            }
            state.dropn(return_count);
            state.reachable = false;
        }

        /************************************ Calls ****************************************
         * The call instructions pop off their arguments from the stack and append their
         * return values to it.
         ************************************ Calls ****************************************/
        Instruction::Call(callee_index) => {
            let &(ref callee_ref, ref callee_func) =
                entity_creator.get_direct_func(builder.func, callee_index, compiler)?;

            let sig = callee_func.signature();
            let num_args = normal_args(&sig);
            let call_args = with_vmctx(builder.func, state.peekn(num_args))?;

            let call = builder.cursor().ins().call(*callee_ref, &call_args);

            state.dropn(num_args);
            state.pushn(builder.inst_results(call));
        }

        Instruction::CallIndirect(type_index, _reserved) => {
            let table_index = 0;
            let (table, table_base) =
                entity_creator.get_table(table_index, builder.func, compiler)?;
            let &(ref sig_ref, ref fnsig) =
                entity_creator.get_indirect_sig(builder.func, type_index)?;
            let num_args = normal_args(&fnsig.cton_signature());

            let callee = state.pop1();

            // Indirect calls are performed by looking up the callee function and type in a table that
            // is present in the same object file.
            // The table is an array of pairs of (type index, function pointer). Both elements in the
            // pair are the size of a pointer.
            // The array is indexed by the callee, as an integer. The callee passed in above is a
            // symbolic value because it is only known at run-time.
            // We bounds-check the callee, look up the type index, check that it is equal to the type
            // index for the call, and then call the function at the pointer.

            let call: Result<ir::Inst, Error> = {
                let mut pos = builder.cursor();

                // `callee` is an integer value that may represent a valid offset into the
                // icall table.
                let calleebound = table.elements().len();
                // First see if the callee is even a valid index into the table.
                let inbounds = pos.ins().icmp_imm(
                    ir::condcodes::IntCC::UnsignedLessThan,
                    callee,
                    calleebound as i64,
                );
                pos.ins().trapz(inbounds, ir::TrapCode::IndirectCallToNull);

                let table_addr = pos.ins().global_value(NATIVE_POINTER, table_base);
                let callee_64 = pos.ins().uextend(ir::Type::int(64).unwrap(), callee);
                // Get the type index from memory:
                let table_type_offs = pos.ins().imul_imm(callee_64, 2 * POINTER_SIZE as i64);
                let table_type = pos.ins().iadd(table_addr, table_type_offs);
                let typ = pos.ins().load(
                    ir::Type::int(64).unwrap(),
                    ir::MemFlags::new(),
                    table_type,
                    0,
                );
                let valid_type =
                    pos.ins()
                        .icmp_imm(ir::condcodes::IntCC::Equal, typ, type_index as i64);
                pos.ins().trapz(valid_type, ir::TrapCode::BadSignature);
                // Get the function ptr from memory:
                let func_addr = pos.ins().load(
                    NATIVE_POINTER,
                    ir::MemFlags::new(),
                    table_type,
                    8, // Size of i64 above
                );

                let call_args = with_vmctx(pos.func, state.peekn(num_args))?;

                Ok(pos.ins().call_indirect(*sig_ref, func_addr, &call_args))
            };
            let call = call?;
            state.dropn(num_args);
            state.pushn(builder.inst_results(call));
        }

        /******************************* Memory management ***********************************
         * Memory management calls out to functions that come from the runtime.
         ************************************************************************************/
        Instruction::GrowMemory(_reserved) => {
            let &(ref callee_ref, ref _callee_func) = entity_creator.get_runtime_func(
                builder.func,
                "lucet_vmctx_grow_memory".into(),
                compiler,
            )?;

            let new_pages = state.pop1();

            let call_args = with_vmctx(builder.func, &[new_pages])?;

            let call = builder.cursor().ins().call(*callee_ref, &call_args);

            state.pushn(builder.inst_results(call));
        }

        Instruction::CurrentMemory(_reserved) => {
            let &(ref callee_ref, ref _callee_func) = entity_creator.get_runtime_func(
                builder.func,
                "lucet_vmctx_current_memory".into(),
                compiler,
            )?;

            let call_args = with_vmctx(builder.func, &[])?;

            let call = builder.cursor().ins().call(*callee_ref, &call_args);

            state.pushn(builder.inst_results(call));
        }

        /******************************* Load instructions ***********************************
         * Wasm specifies an integer alignment flag but we drop it in Cretonne.
         * The memory base address is provided by the environment.
         * TODO: differentiate between 32 bit and 64 bit architecture, to put the uextend or not
         ************************************************************************************/
        Instruction::I32Load8U(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Uload8,
                I32,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I32Load16U(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Uload16,
                I32,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I32Load8S(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Sload8,
                I32,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I32Load16S(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Sload16,
                I32,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I64Load8U(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Uload8,
                I64,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I64Load16U(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Uload16,
                I64,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I64Load8S(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Sload8,
                I64,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I64Load16S(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Sload16,
                I64,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I64Load32S(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Sload32,
                I64,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I64Load32U(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Uload32,
                I64,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I32Load(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Load,
                I32,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::F32Load(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Load,
                F32,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I64Load(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Load,
                I64,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::F64Load(_flags, offset) => {
            translate_load(
                offset,
                ir::Opcode::Load,
                F64,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        /****************************** Store instructions ***********************************
         * Wasm specifies an integer alignment flag but we drop it in Cretonne.
         * The memory base address is provided by the environment.
         * TODO: differentiate between 32 bit and 64 bit architecture, to put the uextend or not
         ************************************************************************************/
        Instruction::I32Store(_flags, offset)
        | Instruction::I64Store(_flags, offset)
        | Instruction::F32Store(_flags, offset)
        | Instruction::F64Store(_flags, offset) => {
            translate_store(
                offset,
                ir::Opcode::Store,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I32Store8(_flags, offset) | Instruction::I64Store8(_flags, offset) => {
            translate_store(
                offset,
                ir::Opcode::Istore8,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I32Store16(_flags, offset) | Instruction::I64Store16(_flags, offset) => {
            translate_store(
                offset,
                ir::Opcode::Istore16,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        Instruction::I64Store32(_flags, offset) => {
            translate_store(
                offset,
                ir::Opcode::Istore32,
                builder,
                state,
                entity_creator,
                compiler,
            )?;
        }
        /****************************** Nullary Opcodes ************************************/
        Instruction::I32Const(value) => state.push1(builder.ins().iconst(I32, i64::from(value))),
        Instruction::I64Const(value) => state.push1(builder.ins().iconst(I64, value)),
        Instruction::F32Const(value) => {
            state.push1(builder.ins().f32const(f32_translation(value)));
        }
        Instruction::F64Const(value) => {
            state.push1(builder.ins().f64const(f64_translation(value)));
        }
        /******************************* Unary Opcodes *************************************/
        Instruction::I32Clz | Instruction::I64Clz => {
            let arg = state.pop1();
            state.push1(builder.ins().clz(arg));
        }
        Instruction::I32Ctz | Instruction::I64Ctz => {
            let arg = state.pop1();
            state.push1(builder.ins().ctz(arg));
        }
        Instruction::I32Popcnt | Instruction::I64Popcnt => {
            let arg = state.pop1();
            state.push1(builder.ins().popcnt(arg));
        }
        Instruction::I64ExtendSI32 => {
            let val = state.pop1();
            state.push1(builder.ins().sextend(I64, val));
        }
        Instruction::I64ExtendUI32 => {
            let val = state.pop1();
            state.push1(builder.ins().uextend(I64, val));
        }
        Instruction::I32WrapI64 => {
            let val = state.pop1();
            state.push1(builder.ins().ireduce(I32, val));
        }
        Instruction::F32Sqrt | Instruction::F64Sqrt => {
            let arg = state.pop1();
            state.push1(builder.ins().sqrt(arg));
        }
        Instruction::F32Ceil | Instruction::F64Ceil => {
            let arg = state.pop1();
            state.push1(builder.ins().ceil(arg));
        }
        Instruction::F32Floor | Instruction::F64Floor => {
            let arg = state.pop1();
            state.push1(builder.ins().floor(arg));
        }
        Instruction::F32Trunc | Instruction::F64Trunc => {
            let arg = state.pop1();
            state.push1(builder.ins().trunc(arg));
        }
        Instruction::F32Nearest | Instruction::F64Nearest => {
            let arg = state.pop1();
            state.push1(builder.ins().nearest(arg));
        }
        Instruction::F32Abs | Instruction::F64Abs => {
            let val = state.pop1();
            state.push1(builder.ins().fabs(val));
        }
        Instruction::F32Neg | Instruction::F64Neg => {
            let arg = state.pop1();
            state.push1(builder.ins().fneg(arg));
        }
        Instruction::F64ConvertUI64 | Instruction::F64ConvertUI32 => {
            let val = state.pop1();
            state.push1(builder.ins().fcvt_from_uint(F64, val));
        }
        Instruction::F64ConvertSI64 | Instruction::F64ConvertSI32 => {
            let val = state.pop1();
            state.push1(builder.ins().fcvt_from_sint(F64, val));
        }
        Instruction::F32ConvertSI64 | Instruction::F32ConvertSI32 => {
            let val = state.pop1();
            state.push1(builder.ins().fcvt_from_sint(F32, val));
        }
        Instruction::F32ConvertUI64 | Instruction::F32ConvertUI32 => {
            let val = state.pop1();
            state.push1(builder.ins().fcvt_from_uint(F32, val));
        }
        Instruction::F64PromoteF32 => {
            let val = state.pop1();
            state.push1(builder.ins().fpromote(F64, val));
        }
        Instruction::F32DemoteF64 => {
            let val = state.pop1();
            state.push1(builder.ins().fdemote(F32, val));
        }
        Instruction::I64TruncSF64 | Instruction::I64TruncSF32 => {
            let val = state.pop1();
            state.push1(builder.ins().fcvt_to_sint(I64, val));
        }
        Instruction::I32TruncSF64 | Instruction::I32TruncSF32 => {
            let val = state.pop1();
            state.push1(builder.ins().fcvt_to_sint(I32, val));
        }
        Instruction::I64TruncUF64 | Instruction::I64TruncUF32 => {
            let val = state.pop1();
            state.push1(builder.ins().fcvt_to_uint(I64, val));
        }
        Instruction::I32TruncUF64 | Instruction::I32TruncUF32 => {
            let val = state.pop1();
            state.push1(builder.ins().fcvt_to_uint(I32, val));
        }
        Instruction::F32ReinterpretI32 => {
            let val = state.pop1();
            state.push1(builder.ins().bitcast(F32, val));
        }
        Instruction::F64ReinterpretI64 => {
            let val = state.pop1();
            state.push1(builder.ins().bitcast(F64, val));
        }
        Instruction::I32ReinterpretF32 => {
            let val = state.pop1();
            state.push1(builder.ins().bitcast(I32, val));
        }
        Instruction::I64ReinterpretF64 => {
            let val = state.pop1();
            state.push1(builder.ins().bitcast(I64, val));
        }
        /****************************** Binary Opcodes ************************************/
        Instruction::I32Add | Instruction::I64Add => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().iadd(arg1, arg2));
        }
        Instruction::I32And | Instruction::I64And => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().band(arg1, arg2));
        }
        Instruction::I32Or | Instruction::I64Or => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().bor(arg1, arg2));
        }
        Instruction::I32Xor | Instruction::I64Xor => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().bxor(arg1, arg2));
        }
        Instruction::I32Shl | Instruction::I64Shl => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().ishl(arg1, arg2));
        }
        Instruction::I32ShrS | Instruction::I64ShrS => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().sshr(arg1, arg2));
        }
        Instruction::I32ShrU | Instruction::I64ShrU => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().ushr(arg1, arg2));
        }
        Instruction::I32Rotl | Instruction::I64Rotl => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().rotl(arg1, arg2));
        }
        Instruction::I32Rotr | Instruction::I64Rotr => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().rotr(arg1, arg2));
        }
        Instruction::F32Add | Instruction::F64Add => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().fadd(arg1, arg2));
        }
        Instruction::I32Sub | Instruction::I64Sub => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().isub(arg1, arg2));
        }
        Instruction::F32Sub | Instruction::F64Sub => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().fsub(arg1, arg2));
        }
        Instruction::I32Mul | Instruction::I64Mul => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().imul(arg1, arg2));
        }
        Instruction::F32Mul | Instruction::F64Mul => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().fmul(arg1, arg2));
        }
        Instruction::F32Div | Instruction::F64Div => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().fdiv(arg1, arg2));
        }
        Instruction::I32DivS | Instruction::I64DivS => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().sdiv(arg1, arg2));
        }
        Instruction::I32DivU | Instruction::I64DivU => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().udiv(arg1, arg2));
        }
        Instruction::I32RemS | Instruction::I64RemS => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().srem(arg1, arg2));
        }
        Instruction::I32RemU | Instruction::I64RemU => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().urem(arg1, arg2));
        }
        Instruction::F32Min | Instruction::F64Min => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().fmin(arg1, arg2));
        }
        Instruction::F32Max | Instruction::F64Max => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().fmax(arg1, arg2));
        }
        Instruction::F32Copysign | Instruction::F64Copysign => {
            let (arg1, arg2) = state.pop2();
            state.push1(builder.ins().fcopysign(arg1, arg2));
        }
        /**************************** Comparison Opcodes **********************************/
        Instruction::I32LtS | Instruction::I64LtS => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().icmp(IntCC::SignedLessThan, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::I32LtU | Instruction::I64LtU => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().icmp(IntCC::UnsignedLessThan, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::I32LeS | Instruction::I64LeS => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().icmp(IntCC::SignedLessThanOrEqual, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::I32LeU | Instruction::I64LeU => {
            let (arg1, arg2) = state.pop2();
            let val = builder
                .ins()
                .icmp(IntCC::UnsignedLessThanOrEqual, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::I32GtS | Instruction::I64GtS => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().icmp(IntCC::SignedGreaterThan, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::I32GtU | Instruction::I64GtU => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().icmp(IntCC::UnsignedGreaterThan, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::I32GeS | Instruction::I64GeS => {
            let (arg1, arg2) = state.pop2();
            let val = builder
                .ins()
                .icmp(IntCC::SignedGreaterThanOrEqual, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::I32GeU | Instruction::I64GeU => {
            let (arg1, arg2) = state.pop2();
            let val = builder
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::I32Eqz | Instruction::I64Eqz => {
            let arg = state.pop1();
            let val = builder.ins().icmp_imm(IntCC::Equal, arg, 0);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::I32Eq | Instruction::I64Eq => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().icmp(IntCC::Equal, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::F32Eq | Instruction::F64Eq => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().fcmp(FloatCC::Equal, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::I32Ne | Instruction::I64Ne => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().icmp(IntCC::NotEqual, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::F32Ne | Instruction::F64Ne => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().fcmp(FloatCC::NotEqual, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::F32Gt | Instruction::F64Gt => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().fcmp(FloatCC::GreaterThan, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::F32Ge | Instruction::F64Ge => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().fcmp(FloatCC::GreaterThanOrEqual, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::F32Lt | Instruction::F64Lt => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().fcmp(FloatCC::LessThan, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        Instruction::F32Le | Instruction::F64Le => {
            let (arg1, arg2) = state.pop2();
            let val = builder.ins().fcmp(FloatCC::LessThanOrEqual, arg1, arg2);
            state.push1(builder.ins().bint(I32, val));
        }
        _ => panic!("Unimplemented opcode: {:?}", *op),
    }

    Ok(())
}

fn translate_unreachable_opcode(
    op: &Instruction,
    builder: &mut FunctionBuilder,
    state: &mut TranslationState,
) -> Result<(), Error> {
    // Don't translate any ops for this code, because it is unreachable.
    // Just record the phantom stack for this code so we know when unreachable code ends.
    match *op {
        Instruction::If(_) => {
            state.push_control_frame(
                ControlVariant::_if(ir::Inst::reserved_value(), false),
                ir::Ebb::reserved_value(),
                0,
            );
        }
        Instruction::Loop(_) | Instruction::Block(_) => {
            state.push_control_frame(ControlVariant::_block(), ir::Ebb::reserved_value(), 0);
        }
        Instruction::Else => {
            let i = state.control_stack.len() - 1;
            match state.control_stack[i].variant {
                ControlVariant::If {
                    branch_inst,
                    ref mut reachable_from_top,
                    ..
                } => {
                    if *reachable_from_top {
                        // We have reached a branch from the top of the if to the else
                        state.reachable = true;
                        // And because there's an else, there can no longer be a branch from the
                        // top directly to the end.
                        *reachable_from_top = false;
                        // Retarget the If branch to the else block
                        let else_ebb = builder.create_ebb();
                        builder.change_jump_destination(branch_inst, else_ebb);
                        builder.seal_block(else_ebb);
                        builder.switch_to_block(else_ebb);
                    }
                }
                _ => panic!("impossible: else instruction when not in `if`"),
            }
        }
        Instruction::End => {
            let stack = &mut state.stack;
            let control_stack = &mut state.control_stack;
            let frame = control_stack.pop().unwrap();

            // Now we have split off the stack the values not used by unreachable code that hasn't
            // been translated
            stack.truncate(frame.original_stack_size);
            let reachable_anyway = match frame.variant {
                ControlVariant::Loop { body, .. } => {
                    // Have to seal the body loop block
                    builder.seal_block(body);
                    // Loops cant have branches to the end
                    false
                }
                ControlVariant::If {
                    reachable_from_top, ..
                } => {
                    // A reachable if without an else has a branch from the top directly to the
                    // bottom
                    reachable_from_top
                }
                // Blocks are already handled
                ControlVariant::Block { .. } => false,
            };

            if frame.exit_is_branched_to() || reachable_anyway {
                builder.switch_to_block(frame.following_code());
                builder.seal_block(frame.following_code());
                // Add the return values of the block only if the next block is reachable
                stack.extend_from_slice(builder.ebb_params(frame.following_code()));
                state.reachable = true;
            }
        }
        _ => {} // All other opcodes are not translated
    }
    Ok(())
}

fn cton_blocktype(bt: &BlockType) -> Option<ir::Type> {
    match bt {
        &BlockType::Value(vt) => Some(cton_valuetype(&vt)),
        &BlockType::NoResult => None,
    }
}

fn num_return_values(bt: &BlockType) -> usize {
    match cton_blocktype(bt) {
        Some(_) => 1,
        None => 0,
    }
}

// Translate a load instruction.
// XXX we could take the flags argument from the wasm load instruction,
// and use that to generate a cton memflags.
fn translate_load<'m>(
    offset: u32,
    opcode: ir::Opcode,
    result_ty: ir::Type,
    builder: &mut FunctionBuilder,
    state: &mut TranslationState,
    entity_creator: &mut EntityCreator<'m>,
    compiler: &Compiler,
) -> Result<(), Error> {
    let addr32 = state.pop1();
    // We don't yet support multiple linear memories.
    let heap = entity_creator.get_heap(builder.func, 0, compiler)?;
    let (base, offset) = get_heap_addr(heap, addr32, offset, NATIVE_POINTER, builder);
    let flags = MemFlags::new();
    let (load, dfg) = builder
        .ins()
        .Load(opcode, result_ty, flags, offset.into(), base);
    state.push1(dfg.first_result(load));
    Ok(())
}

// Translate a store instruction.
// XXX we could take the flags argument from the wasm store instruction,
// and use that to generate a cton memflags.
fn translate_store<'m>(
    offset: u32,
    opcode: ir::Opcode,
    builder: &mut FunctionBuilder,
    state: &mut TranslationState,
    entity_creator: &mut EntityCreator<'m>,
    compiler: &Compiler,
) -> Result<(), Error> {
    let (addr32, val) = state.pop2();
    let val_ty = builder.func.dfg.value_type(val);

    // We don't yet support multiple linear memories.
    let heap = entity_creator.get_heap(builder.func, 0, compiler)?;
    let (base, offset) = get_heap_addr(heap, addr32, offset, NATIVE_POINTER, builder);
    let flags = MemFlags::new();
    builder
        .ins()
        .Store(opcode, val_ty, flags, offset.into(), val, base);
    Ok(())
}

// Get the address+offset to use for a heap access.
fn get_heap_addr(
    heap: ir::Heap,
    addr32: ir::Value,
    offset: u32,
    addr_ty: ir::Type,
    builder: &mut FunctionBuilder,
) -> (ir::Value, i32) {
    use std::cmp::min;

    let guard_size: u64 = builder.func.heaps[heap].offset_guard_size.into();
    assert!(guard_size > 0, "Heap guard pages currently required");

    // Generate `heap_addr` instructions that are friendly to CSE by checking offsets that are
    // multiples of the guard size. Add one to make sure that we check the pointer itself is in
    // bounds.
    //
    // For accesses on the outer skirts of the guard pages, we expect that we get a trap
    // even if the access goes beyond the guard pages. This is because the first byte pointed to is
    // inside the guard pages.
    let check_size = min(
        u32::MAX as u64,
        1 + (offset as u64 / guard_size) * guard_size,
    ) as u32;
    let base = builder.ins().heap_addr(addr_ty, heap, addr32, check_size);

    // Native load/store instructions take a signed `Offset32` immediate, so adjust the base
    // pointer if necessary.
    if offset > i32::MAX as u32 {
        // Offset doesn't fit in the load/store instruction.
        let adj = builder.ins().iadd_imm(base, i64::from(i32::MAX) + 1);
        (adj, (offset - (i32::MAX as u32 + 1)) as i32)
    } else {
        (base, offset as i32)
    }
}

fn f32_translation(x: u32) -> ir::immediates::Ieee32 {
    ir::immediates::Ieee32::with_bits(x)
}

fn f64_translation(x: u64) -> ir::immediates::Ieee64 {
    ir::immediates::Ieee64::with_bits(x)
}

fn normal_args(sig: &ir::Signature) -> usize {
    sig.params
        .iter()
        .filter(|a| a.purpose == ir::ArgumentPurpose::Normal)
        .count()
}

fn with_vmctx(func: &ir::Function, base_args: &[ir::Value]) -> Result<Vec<ir::Value>, Error> {
    let mut args: Vec<ir::Value> = Vec::with_capacity(base_args.len() + 1);
    args.extend_from_slice(base_args);
    args.insert(
        0,
        func.special_param(ir::ArgumentPurpose::VMContext)
            .ok_or(format_err!(
                "getting vm context parameter insert in call args"
            ))?,
    );
    Ok(args)
}
