mod cpu_features;

pub use self::cpu_features::{CpuFeatures, SpecificFeature, TargetCpu};
use crate::decls::ModuleDecls;
use crate::error::Error;
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
use lucet_module::bindings::Bindings;
use lucet_module::{FunctionSpec, ModuleData, ModuleFeatures, MODULE_DATA_SYM};
use lucet_validate::Validator;
use target_lexicon::Triple;

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
    target: Triple,
    opt_level: OptLevel,
    cpu_features: CpuFeatures,
    count_instructions: bool,
    module_translation_state: ModuleTranslationState,
}

impl<'a> Compiler<'a> {
    pub fn new(
        wasm_binary: &'a [u8],
        target: Triple,
        opt_level: OptLevel,
        cpu_features: CpuFeatures,
        bindings: &'a Bindings,
        heap_settings: HeapSettings,
        count_instructions: bool,
        validator: &Option<Validator>,
    ) -> Result<Self, Error> {
        let isa = Self::target_isa(target.clone(), opt_level, &cpu_features)?;

        let frontend_config = isa.frontend_config();
        let mut module_info = ModuleInfo::new(frontend_config.clone());

        if let Some(v) = validator {
            v.validate(wasm_binary).map_err(Error::LucetValidation)?;
        } else {
            // As of cranelift-wasm 0.43 which uses wasmparser 0.39.1, the parser used inside
            // cranelift-wasm does not validate. We need to run the validating parser on the binary
            // first. The InvalidWebAssembly error below will never trigger.
            wasmparser::validate(wasm_binary, None).map_err(Error::WasmValidation)?;
        }

        let module_translation_state =
            translate_module(wasm_binary, &mut module_info).map_err(|e| match e {
                WasmError::User(u) => Error::Input(u.to_string()),
                WasmError::InvalidWebAssembly { .. } => {
                    // Since wasmparser was already used to validate,
                    // reaching this case means there's a significant
                    // bug in either wasmparser or cranelift-wasm.
                    unreachable!();
                }
                WasmError::Unsupported(s) => Error::Unsupported(s.to_owned()),
                WasmError::ImplLimitExceeded { .. } => Error::ClifWasmError(e),
            })?;

        let libcalls = Box::new(move |libcall| match libcall {
            ir::LibCall::Probestack => stack_probe::STACK_PROBE_SYM.to_owned(),
            _ => (cranelift_module::default_libcall_names())(libcall),
        });

        let mut clif_module: ClifModule<FaerieBackend> = ClifModule::new(FaerieBuilder::new(
            isa,
            "lucet_guest".to_owned(),
            FaerieTrapCollection::Enabled,
            libcalls,
        )?);

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
            target,
        })
    }

    pub fn module_features(&self) -> ModuleFeatures {
        // This will grow in the future to encompass other options describing the compiled module.
        (&self.cpu_features).into()
    }

    pub fn module_data(&self) -> Result<ModuleData<'_>, Error> {
        self.decls.get_module_data(self.module_features())
    }

    pub fn object_file(mut self) -> Result<ObjectFile, Error> {
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
                .map_err(|source| Error::FunctionTranslation {
                    symbol: func.name.symbol().to_string(),
                    source,
                })?;
            self.clif_module
                .define_function(func.name.as_funcid().unwrap(), &mut clif_context)
                .map_err(|source| Error::FunctionDefinition {
                    symbol: func.name.symbol().to_string(),
                    source,
                })?;
        }

        stack_probe::declare_metadata(&mut self.decls, &mut self.clif_module).unwrap();

        let module_data_bytes = self.module_data()?.serialize()?;

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
        )?;

        Ok(obj)
    }

    pub fn cranelift_funcs(self) -> Result<CraneliftFuncs, Error> {
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
                .map_err(|source| Error::FunctionTranslation {
                    symbol: func.name.symbol().to_string(),
                    source,
                })?;

            funcs.insert(func.name.clone(), clif_context.func);
        }
        Ok(CraneliftFuncs::new(
            funcs,
            Self::target_isa(self.target, self.opt_level, &self.cpu_features)?,
        ))
    }

    fn target_isa(
        target: Triple,
        opt_level: OptLevel,
        cpu_features: &CpuFeatures,
    ) -> Result<Box<dyn TargetIsa>, Error> {
        let mut flags_builder = settings::builder();
        let isa_builder = cpu_features.isa_builder(target)?;
        flags_builder.enable("enable_verifier").unwrap();
        flags_builder.enable("is_pic").unwrap();
        flags_builder.set("opt_level", opt_level.to_flag()).unwrap();
        Ok(isa_builder.finish(settings::Flags::new(flags_builder)))
    }
}

fn write_module_data<B: ClifBackend>(
    clif_module: &mut ClifModule<B>,
    module_data_bytes: Vec<u8>,
) -> Result<(), Error> {
    use cranelift_module::{DataContext, Linkage};

    let mut module_data_ctx = DataContext::new();
    module_data_ctx.define(module_data_bytes.into_boxed_slice());

    let module_data_decl = clif_module
        .declare_data(MODULE_DATA_SYM, Linkage::Local, true, None)
        .map_err(Error::ClifModuleError)?;
    clif_module
        .define_data(module_data_decl, &module_data_ctx)
        .map_err(Error::ClifModuleError)?;

    Ok(())
}

fn write_startfunc_data<B: ClifBackend>(
    clif_module: &mut ClifModule<B>,
    decls: &ModuleDecls<'_>,
) -> Result<(), Error> {
    use cranelift_module::{DataContext, Linkage};

    if let Some(func_ix) = decls.get_start_func() {
        let name = clif_module
            .declare_data("guest_start", Linkage::Export, false, None)
            .map_err(Error::MetadataSerializer)?;
        let mut ctx = DataContext::new();
        ctx.define_zeroinit(8);

        let start_func = decls
            .get_func(func_ix)
            .expect("start func is valid func id");
        let fid = start_func.name.as_funcid().expect("start func is a func");
        let fref = clif_module.declare_func_in_data(fid, &mut ctx);
        ctx.write_function_addr(0, fref);
        clif_module
            .define_data(name, &ctx)
            .map_err(Error::MetadataSerializer)?;
    }
    Ok(())
}
