use crate::compiler::entity::EntityCreator;
use crate::compiler::opcode::translate_opcode;
use crate::compiler::state::TranslationState;
use crate::compiler::Compiler;
use crate::program::types::cton_valuetype;
use crate::program::FunctionDef;
use cranelift_codegen::ir::{self, InstBuilder};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use failure::{format_err, Error};
use parity_wasm::elements::{self, FuncBody, ValueType};

pub fn compile_function<'p>(
    compiler: &mut Compiler<'p>,
    function: &FunctionDef,
    body: &FuncBody,
) -> Result<(), Error> {
    let sig = function.signature();

    let name = compiler.get_function(function)?;
    let mut func = ir::Function::with_name_signature(name.clone().into(), sig.clone());

    {
        let mut ctx: FunctionBuilderContext = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut func, &mut ctx);

        // Create entry block.
        let entry_block = builder.create_ebb();
        builder.append_ebb_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);
        builder.ensure_inserted_ebb();

        let mut vargen = VariableGen::new();

        // Declare a local for each wasm parameter.
        for (param_ix, param_type) in sig.params.iter().enumerate() {
            if param_type.purpose == ir::ArgumentPurpose::Normal {
                let var = vargen.mint();
                builder.declare_var(var, param_type.value_type);
                let value = builder.ebb_params(entry_block)[param_ix];
                builder.def_var(var, value);
            }
        }
        // local decls
        declare_locals(&mut builder, &mut vargen, body.locals());

        // Create exit block.
        let exit_block = builder.create_ebb();
        builder.append_ebb_params_for_function_returns(exit_block);

        // TranslationState is used to track control frames, the wasm stack, and translate
        // wasm entities into cretone entities (entities means globals, heaps, sigs, funcs).
        // The exit block is the final dest of all control frames, and return values are the
        // bottom of the wasm stack.
        let mut translation = TranslationState::new(&sig, exit_block);

        let mut entity_creator = EntityCreator::new(&compiler.prog);

        // Function body
        let mut op_iter = body.code().elements().iter();
        while !translation.control_stack.is_empty() {
            let op = op_iter
                .next()
                .ok_or(format_err!("ran out of opcodes before control stack"))?;
            translate_opcode(
                op,
                &mut builder,
                &mut translation,
                &mut entity_creator,
                compiler,
            )?;
        }

        // The end of the iteration left us in the exit block. As long as that block is reachable,
        // need to manually pass the wasm stack to the return instruction.
        if translation.reachable {
            debug_assert!(builder.is_pristine());
            if !builder.is_unreachable() {
                builder.ins().return_(&translation.stack);
            }
        }

        builder.finalize();
    }

    compiler.define_function(name, func)?;
    Ok(())
}

fn declare_locals(
    builder: &mut FunctionBuilder,
    vargen: &mut VariableGen,
    locals: &[elements::Local],
) {
    for local in locals {
        let localtype = local.value_type();
        let zeroval = match localtype {
            ValueType::I32 => builder.ins().iconst(ir::types::I32, 0),
            ValueType::I64 => builder.ins().iconst(ir::types::I64, 0),
            ValueType::F32 => builder.ins().f32const(ir::immediates::Ieee32::with_bits(0)),
            ValueType::F64 => builder.ins().f64const(ir::immediates::Ieee64::with_bits(0)),
            ValueType::V128 => unimplemented!(),
        };
        for _ in 0..local.count() {
            let lvar = vargen.mint();
            builder.declare_var(lvar, cton_valuetype(&localtype));
            builder.def_var(lvar, zeroval);
        }
    }
}

/// `VariableGen` is a source of fresh `Variable`s. It is never used directly by Cretonne.
#[derive(Debug)]
struct VariableGen {
    index: u32,
}

impl VariableGen {
    pub fn new() -> Self {
        Self { index: 0 }
    }
    pub fn mint(&mut self) -> Variable {
        let var = Variable::with_u32(self.index);
        self.index += 1;
        var
    }
}
