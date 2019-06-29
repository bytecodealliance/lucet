use crate::decls::ModuleDecls;
use crate::error::{LucetcError, LucetcErrorKind};
use crate::function::FuncInfo;
use crate::heap::HeapSettings;
use crate::module::ModuleInfo;
use crate::output::{CraneliftFuncs, ObjectFile};
use crate::runtime::Runtime;
use crate::stack_probe;
use crate::table::write_table_data;
use cranelift_codegen::{
    ir,
    isa::TargetIsa,
    settings::{self, Configurable},
    Context as ClifContext,
};
use cranelift_module::{Backend as ClifBackend, Module as ClifModule};
use cranelift_native;
use cranelift_object::{ObjectBackend, ObjectBuilder, ObjectTrapCollection};
use cranelift_wasm::{translate_module, FuncTranslator, WasmError};
use failure::{format_err, Fail, ResultExt};
use lucet_module::bindings::Bindings;
use lucet_module::{ModuleData, MODULE_DATA_SYM};

#[derive(Debug, Clone, Copy)]
pub enum OptLevel {
    None,
    Standard,
    Fast,
}

impl Default for OptLevel {
    fn default() -> OptLevel {
        OptLevel::Standard
    }
}

impl OptLevel {
    pub fn to_flag(&self) -> &str {
        match self {
            OptLevel::None => "fastest",
            OptLevel::Standard => "default",
            OptLevel::Fast => "best",
        }
    }
}

pub struct Compiler<'a> {
    decls: ModuleDecls<'a>,
    clif_module: ClifModule<ObjectBackend>,
    opt_level: OptLevel,
    count_instructions: bool,
}

impl<'a> Compiler<'a> {
    pub fn new(
        wasm_binary: &'a [u8],
        opt_level: OptLevel,
        bindings: &'a Bindings,
        heap_settings: HeapSettings,
        count_instructions: bool,
    ) -> Result<Self, LucetcError> {
        let isa = Self::target_isa(opt_level);

        let frontend_config = isa.frontend_config();
        let mut module_info = ModuleInfo::new(frontend_config.clone());

        // As of cranelift-wasm 0.29, which uses wasmparser 0.23, the parser used inside
        // cranelift-wasm does not validate. We need to run the validating parser on the binary
        // first. The InvalidWebAssembly error below will never trigger.
        use wasmparser::validate;
        if !validate(wasm_binary, None) {
            Err(format_err!("wasmparser validation rejected module"))
                .context(LucetcErrorKind::Validation)?;
        }

        translate_module(wasm_binary, &mut module_info).map_err(|e| match e {
            WasmError::User(_) => e.context(LucetcErrorKind::Input),
            WasmError::InvalidWebAssembly { .. } => e.context(LucetcErrorKind::Validation), // This will trigger once cranelift-wasm upgrades to a validating wasm parser.
            WasmError::Unsupported { .. } => e.context(LucetcErrorKind::Unsupported),
            WasmError::ImplLimitExceeded { .. } => e.context(LucetcErrorKind::TranslatingModule),
        })?;

        let libcalls = Box::new(move |libcall| match libcall {
            ir::LibCall::Probestack => stack_probe::STACK_PROBE_SYM.to_owned(),
            _ => (cranelift_module::default_libcall_names())(libcall),
        });

        let mut clif_module: ClifModule<ObjectBackend> = ClifModule::new(
            ObjectBuilder::new(
                isa,
                "lucet_guest".to_owned(),
                ObjectTrapCollection::Enabled,
                libcalls,
            )
            .context(LucetcErrorKind::Validation)?,
        );

        let runtime = Runtime::lucet(frontend_config);
        let decls = ModuleDecls::new(
            module_info,
            &mut clif_module,
            bindings,
            runtime,
            heap_settings,
        )?;

        Ok(Self {
            decls,
            clif_module,
            opt_level,
            count_instructions,
        })
    }

    pub fn module_data(&self) -> Result<ModuleData<'_>, LucetcError> {
        self.decls.get_module_data()
    }

    pub fn object_file(mut self) -> Result<ObjectFile, LucetcError> {
        let mut func_translator = FuncTranslator::new();

        for (ref func, (code, code_offset)) in self.decls.function_bodies() {
            let mut func_info = FuncInfo::new(&self.decls, self.count_instructions);
            let mut clif_context = ClifContext::new();
            clif_context.func.name = func.name.as_externalname();
            clif_context.func.signature = func.signature.clone();

            func_translator
                .translate(code, *code_offset, &mut clif_context.func, &mut func_info)
                .map_err(|e| format_err!("in {}: {:?}", func.name.symbol(), e))
                .context(LucetcErrorKind::FunctionTranslation)?;

            self.clif_module
                .define_function(func.name.as_funcid().unwrap(), &mut clif_context)
                .map_err(|e| format_err!("in {}: {:?}", func.name.symbol(), e))
                .context(LucetcErrorKind::FunctionDefinition)?;
        }

        stack_probe::declare_metadata(&mut self.decls, &mut self.clif_module).unwrap();

        let module_data_len = write_module_data(&mut self.clif_module, &self.decls)?;
        write_startfunc_data(&mut self.clif_module, &self.decls)?;
        let table_names = write_table_data(&mut self.clif_module, &self.decls)?;

        self.clif_module.finalize_definitions();
        let obj = ObjectFile::new(self.clif_module.finish(), module_data_len, table_names)
            .context(LucetcErrorKind::Output)?;
        Ok(obj)
    }

    pub fn cranelift_funcs(self) -> Result<CraneliftFuncs, LucetcError> {
        use std::collections::HashMap;

        let mut funcs = HashMap::new();
        let mut func_translator = FuncTranslator::new();

        for (ref func, (code, code_offset)) in self.decls.function_bodies() {
            let mut func_info = FuncInfo::new(&self.decls, self.count_instructions);
            let mut clif_context = ClifContext::new();
            clif_context.func.name = func.name.as_externalname();
            clif_context.func.signature = func.signature.clone();

            func_translator
                .translate(code, *code_offset, &mut clif_context.func, &mut func_info)
                .map_err(|e| format_err!("in {}: {:?}", func.name.symbol(), e))
                .context(LucetcErrorKind::FunctionTranslation)?;

            funcs.insert(func.name.clone(), clif_context.func);
        }
        Ok(CraneliftFuncs::new(funcs, Self::target_isa(self.opt_level)))
    }

    fn target_isa(opt_level: OptLevel) -> Box<dyn TargetIsa> {
        let mut flags_builder = settings::builder();
        let isa_builder =
            cranelift_native::builder().expect("host machine is not a supported target");
        flags_builder.enable("enable_verifier").unwrap();
        flags_builder.enable("is_pic").unwrap();
        flags_builder.set("opt_level", opt_level.to_flag()).unwrap();
        isa_builder.finish(settings::Flags::new(flags_builder))
    }
}

fn write_module_data<B: ClifBackend>(
    clif_module: &mut ClifModule<B>,
    decls: &ModuleDecls<'_>,
) -> Result<usize, LucetcError> {
    use cranelift_module::{DataContext, Linkage};

    let module_data_serialized: Vec<u8> = decls
        .get_module_data()?
        .serialize()
        .context(LucetcErrorKind::ModuleData)?;

    let module_data_len = module_data_serialized.len();

    let mut module_data_ctx = DataContext::new();
    module_data_ctx.define(module_data_serialized.into_boxed_slice());

    let module_data_decl = clif_module
        .declare_data(MODULE_DATA_SYM, Linkage::Local, true, None)
        .context(LucetcErrorKind::ModuleData)?;
    clif_module
        .define_data(module_data_decl, &module_data_ctx)
        .context(LucetcErrorKind::ModuleData)?;

    Ok(module_data_len)
}

fn write_startfunc_data<B: ClifBackend>(
    clif_module: &mut ClifModule<B>,
    decls: &ModuleDecls<'_>,
) -> Result<(), LucetcError> {
    use cranelift_module::{DataContext, Linkage};

    let error_kind = LucetcErrorKind::MetadataSerializer;

    if let Some(func_ix) = decls.get_start_func() {
        let name = clif_module
            .declare_data("guest_start", Linkage::Export, false, None)
            .context(error_kind.clone())?;
        let mut ctx = DataContext::new();
        ctx.define_zeroinit(8);

        let start_func = decls
            .get_func(func_ix)
            .expect("start func is valid func id");
        let fid = start_func
            .name
            .as_funcid()
            .ok_or(format_err!("start index pointed to a non-function"))
            .context(error_kind.clone())?;
        let fref = clif_module.declare_func_in_data(fid, &mut ctx);
        ctx.write_function_addr(0, fref);
        clif_module.define_data(name, &ctx).context(error_kind)?;
    }
    Ok(())
}
