use crate::bindings::Bindings;
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
use cranelift_native;
use cranelift_wasm::{translate_module, FuncTranslator, WasmError};
use failure::{format_err, Fail, ResultExt};
use lucet_module_data::{FunctionSpec, ModuleData};

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
            OptLevel::None => "fast",
            OptLevel::Standard => "default",
            OptLevel::Fast => "best",
        }
    }
}

pub struct Compiler<'a> {
    decls: ModuleDecls<'a>,
    clif_module: ClifModule<FaerieBackend>,
    opt_level: OptLevel,
}

impl<'a> Compiler<'a> {
    pub fn new(
        wasm_binary: &'a [u8],
        opt_level: OptLevel,
        bindings: &Bindings,
        heap_settings: HeapSettings,
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
            WasmError::InvalidWebAssembly { .. } => e.context(LucetcErrorKind::Validation), // This will trigger once cranelift-wasm upgrades to a validating wasm parser.
            WasmError::Unsupported { .. } => e.context(LucetcErrorKind::Unsupported),
            WasmError::ImplLimitExceeded { .. } => e.context(LucetcErrorKind::TranslatingModule),
        })?;

        let libcalls = Box::new(move |libcall| match libcall {
            ir::LibCall::Probestack => stack_probe::STACK_PROBE_SYM.to_owned(),
            _ => (FaerieBuilder::default_libcall_names())(libcall),
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
        })
    }

    pub fn module_data(&self) -> Result<ModuleData, LucetcError> {
        self.decls.get_module_data()
    }

    pub fn object_file(mut self) -> Result<ObjectFile, LucetcError> {
        let mut func_translator = FuncTranslator::new();

        for (ref func, (code, code_offset)) in self.decls.function_bodies() {
            let mut func_info = FuncInfo::new(&self.decls);
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

        write_module_data(&mut self.clif_module, &self.decls)?;
        write_startfunc_data(&mut self.clif_module, &self.decls)?;
        write_table_data(&mut self.clif_module, &self.decls)?;

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

        let obj = ObjectFile::new(self.clif_module.finish(), function_manifest)
            .context(LucetcErrorKind::Output)?;
        Ok(obj)
    }

    pub fn cranelift_funcs(self) -> Result<CraneliftFuncs, LucetcError> {
        use std::collections::HashMap;

        let mut funcs = HashMap::new();
        let mut func_translator = FuncTranslator::new();

        for (ref func, (code, code_offset)) in self.decls.function_bodies() {
            let mut func_info = FuncInfo::new(&self.decls);
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
    decls: &ModuleDecls,
) -> Result<(), LucetcError> {
    use byteorder::{LittleEndian, WriteBytesExt};
    use cranelift_module::{DataContext, Linkage};

    let module_data_serialized: Vec<u8> = decls
        .get_module_data()?
        .serialize()
        .context(LucetcErrorKind::ModuleData)?;
    {
        let mut serialized_len: Vec<u8> = Vec::new();
        serialized_len
            .write_u32::<LittleEndian>(module_data_serialized.len() as u32)
            .unwrap();
        let mut data_len_ctx = DataContext::new();
        data_len_ctx.define(serialized_len.into_boxed_slice());

        let data_len_decl = clif_module
            .declare_data("lucet_module_data_len", Linkage::Export, false)
            .context(LucetcErrorKind::ModuleData)?;
        clif_module
            .define_data(data_len_decl, &data_len_ctx)
            .context(LucetcErrorKind::ModuleData)?;
    }

    {
        let mut module_data_ctx = DataContext::new();
        module_data_ctx.define(module_data_serialized.into_boxed_slice());

        let module_data_decl = clif_module
            .declare_data("lucet_module_data", Linkage::Export, true)
            .context(LucetcErrorKind::ModuleData)?;
        clif_module
            .define_data(module_data_decl, &module_data_ctx)
            .context(LucetcErrorKind::ModuleData)?;
    }
    Ok(())
}

fn write_startfunc_data<B: ClifBackend>(
    clif_module: &mut ClifModule<B>,
    decls: &ModuleDecls,
) -> Result<(), LucetcError> {
    use cranelift_module::{DataContext, Linkage};

    let error_kind = LucetcErrorKind::MetadataSerializer;

    if let Some(func_ix) = decls.get_start_func() {
        let name = clif_module
            .declare_data("guest_start", Linkage::Export, false)
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
