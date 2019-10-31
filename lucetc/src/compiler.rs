mod cpu_features;

pub use self::cpu_features::{CpuFeatures, SpecificFeature, TargetCpu};
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
use cranelift_faerie::{FaerieBackend, FaerieBuilder, FaerieTrapCollection};
use cranelift_module::{Backend as ClifBackend, Module as ClifModule};
use cranelift_wasm::{translate_module, FuncTranslator, ModuleTranslationState, WasmError};
use failure::{format_err, Fail, ResultExt};
use lucet_module::bindings::Bindings;
use lucet_module::{FunctionSpec, ModuleData, ModuleFeatures, MODULE_DATA_SYM};
use lucet_validate::Validator;

#[derive(Debug, Clone, Copy)]
pub enum OptLevel {
    None,
    Speed,
    SpeedAndSize,
}

impl Default for OptLevel {
    fn default() -> OptLevel {
        OptLevel::SpeedAndSize
    }
}

impl OptLevel {
    pub fn to_flag(&self) -> &str {
        match self {
            OptLevel::None => "none",
            OptLevel::Speed => "speed",
            OptLevel::SpeedAndSize => "speed_and_size",
        }
    }
}

pub struct Compiler<'a> {
    decls: ModuleDecls<'a>,
    clif_module: ClifModule<FaerieBackend>,
    opt_level: OptLevel,
    cpu_features: CpuFeatures,
    count_instructions: bool,
    module_translation_state: ModuleTranslationState,
}

impl<'a> Compiler<'a> {
    pub fn new(
        wasm_binary: &'a [u8],
        opt_level: OptLevel,
        cpu_features: CpuFeatures,
        bindings: &'a Bindings,
        heap_settings: HeapSettings,
        count_instructions: bool,
        validator: &Option<Validator>,
    ) -> Result<Self, LucetcError> {
        let isa = Self::target_isa(opt_level, &cpu_features)?;

        let frontend_config = isa.frontend_config();
        let mut module_info = ModuleInfo::new(frontend_config.clone());

        if let Some(v) = validator {
            v.validate(wasm_binary)
                .context(LucetcErrorKind::Validation)?;
        } else {
            // As of cranelift-wasm 0.43 which uses wasmparser 0.39.1, the parser used inside
            // cranelift-wasm does not validate. We need to run the validating parser on the binary
            // first. The InvalidWebAssembly error below will never trigger.
            wasmparser::validate(wasm_binary, None)
                .map_err(|e| {
                    format_err!(
                        "invalid WebAssembly module, at offset {}: {}",
                        e.offset,
                        e.message
                    )
                })
                .context(LucetcErrorKind::Validation)?;
        }

        let module_translation_state =
            translate_module(wasm_binary, &mut module_info).map_err(|e| match e {
                WasmError::User(_) => e.context(LucetcErrorKind::Input),
                WasmError::InvalidWebAssembly { .. } => e.context(LucetcErrorKind::Validation), // This will trigger once cranelift-wasm upgrades to a validating wasm parser.
                WasmError::Unsupported { .. } => e.context(LucetcErrorKind::Unsupported),
                WasmError::ImplLimitExceeded { .. } => {
                    e.context(LucetcErrorKind::TranslatingModule)
                }
            })?;

        let libcalls = Box::new(move |libcall| match libcall {
            ir::LibCall::Probestack => stack_probe::STACK_PROBE_SYM.to_owned(),
            _ => (cranelift_module::default_libcall_names())(libcall),
        });

        let mut clif_module: ClifModule<FaerieBackend> = ClifModule::new(
            FaerieBuilder::new(
                isa,
                "lucet_guest".to_owned(),
                FaerieTrapCollection::Enabled,
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
            cpu_features,
            count_instructions,
            module_translation_state,
        })
    }

    pub fn module_features(&self) -> ModuleFeatures {
        // This will grow in the future to encompass other options describing the compiled module.
        (&self.cpu_features).into()
    }

    pub fn module_data(&self) -> Result<ModuleData<'_>, LucetcError> {
        self.decls.get_module_data(self.module_features())
    }

    pub fn object_file(mut self) -> Result<ObjectFile, LucetcError> {
        let mut func_translator = FuncTranslator::new();

        for (ref func, (code, code_offset)) in self.decls.function_bodies() {
            let mut func_info = FuncInfo::new(&self.decls, self.count_instructions);
            let mut clif_context = ClifContext::new();
            clif_context.func.name = func.name.as_externalname();
            clif_context.func.signature = func.signature.clone();

            func_translator
                .translate(
                    &self.module_translation_state,
                    code,
                    *code_offset,
                    &mut clif_context.func,
                    &mut func_info,
                )
                .map_err(|e| format_err!("in {}: {:?}", func.name.symbol(), e))
                .context(LucetcErrorKind::FunctionTranslation)?;

            self.clif_module
                .define_function(func.name.as_funcid().unwrap(), &mut clif_context)
                .map_err(|e| format_err!("in {}: {:?}", func.name.symbol(), e))
                .context(LucetcErrorKind::FunctionDefinition)?;
        }

        stack_probe::declare_metadata(&mut self.decls, &mut self.clif_module).unwrap();

        let module_data_bytes = self
            .module_data()?
            .serialize()
            .context(LucetcErrorKind::ModuleData)?;
        let module_data_len = module_data_bytes.len();

        write_module_data(&mut self.clif_module, module_data_bytes)?;
        write_startfunc_data(&mut self.clif_module, &self.decls)?;
        let table_names = write_table_data(&mut self.clif_module, &self.decls)?;

        let function_manifest: Vec<(String, FunctionSpec)> = self
            .clif_module
            .declared_functions()
            .map(|f| {
                (
                    f.decl.name.to_owned(), // this copy is only necessary because `clif_module` is moved in `finish, below`
                    FunctionSpec::new(
                        0,
                        f.compiled.as_ref().map(|c| c.code_length()).unwrap_or(0),
                        0,
                        0,
                    ),
                )
            })
            .collect();

        let obj = ObjectFile::new(
            self.clif_module.finish(),
            module_data_len,
            function_manifest,
            table_names,
        )
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
                .translate(
                    &self.module_translation_state,
                    code,
                    *code_offset,
                    &mut clif_context.func,
                    &mut func_info,
                )
                .map_err(|e| format_err!("in {}: {:?}", func.name.symbol(), e))
                .context(LucetcErrorKind::FunctionTranslation)?;

            funcs.insert(func.name.clone(), clif_context.func);
        }
        Ok(CraneliftFuncs::new(
            funcs,
            Self::target_isa(self.opt_level, &self.cpu_features)?,
        ))
    }

    fn target_isa(
        opt_level: OptLevel,
        cpu_features: &CpuFeatures,
    ) -> Result<Box<dyn TargetIsa>, LucetcError> {
        let mut flags_builder = settings::builder();
        let isa_builder = cpu_features.isa_builder()?;
        flags_builder.enable("enable_verifier").unwrap();
        flags_builder.enable("is_pic").unwrap();
        flags_builder.set("opt_level", opt_level.to_flag()).unwrap();
        Ok(isa_builder.finish(settings::Flags::new(flags_builder)))
    }
}

fn write_module_data<B: ClifBackend>(
    clif_module: &mut ClifModule<B>,
    module_data_bytes: Vec<u8>,
) -> Result<(), LucetcError> {
    use cranelift_module::{DataContext, Linkage};

    let mut module_data_ctx = DataContext::new();
    module_data_ctx.define(module_data_bytes.into_boxed_slice());

    let module_data_decl = clif_module
        .declare_data(MODULE_DATA_SYM, Linkage::Local, true, None)
        .context(LucetcErrorKind::ModuleData)?;
    clif_module
        .define_data(module_data_decl, &module_data_ctx)
        .context(LucetcErrorKind::ModuleData)?;

    Ok(())
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
