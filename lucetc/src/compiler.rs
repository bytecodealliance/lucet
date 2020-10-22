mod cpu_features;

pub use self::cpu_features::{CpuFeatures, SpecificFeature, TargetCpu};
use crate::decls::ModuleDecls;
use crate::error::Error;
use crate::function::FuncInfo;
use crate::heap::HeapSettings;
use crate::module::{ModuleValidation, UniqueFuncIndex};
use crate::output::{CraneliftFuncs, ObjectFile, FUNCTION_MANIFEST_SYM};
use crate::runtime::Runtime;
use crate::stack_probe;
use crate::table::write_table_data;
use crate::traps::{translate_trapcode, trap_sym_for_func};
use crate::validate::Validator;
use byteorder::{LittleEndian, WriteBytesExt};
use cranelift_codegen::{
    ir::{self, InstBuilder},
    isa::TargetIsa,
    settings::{self, Configurable},
    Context as ClifContext,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{
    DataContext as ClifDataContext, DataId, FuncId, Linkage as ClifLinkage, Module as ClifModule,
};
use cranelift_object::{ObjectBuilder, ObjectModule, ObjectProduct};
use cranelift_wasm::{
    translate_module,
    wasmparser::{FuncValidator, FunctionBody, ValidatorResources},
    FuncTranslator,
};
use lucet_module::bindings::Bindings;
use lucet_module::{
    InstanceRuntimeData, ModuleData, ModuleFeatures, SerializedModule, VersionInfo,
    LUCET_MODULE_SYM, MODULE_DATA_SYM,
};
use memoffset::offset_of;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex, MutexGuard};
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

pub struct CompilerBuilder {
    target: Triple,
    opt_level: OptLevel,
    cpu_features: CpuFeatures,
    heap_settings: HeapSettings,
    count_instructions: bool,
    canonicalize_nans: bool,
    validator: Option<Validator>,
}

impl CompilerBuilder {
    pub fn new() -> Self {
        Self {
            target: Triple::host(),
            opt_level: OptLevel::default(),
            cpu_features: CpuFeatures::default(),
            heap_settings: HeapSettings::default(),
            count_instructions: false,
            canonicalize_nans: false,
            validator: None,
        }
    }

    pub(crate) fn target_ref(&self) -> &Triple {
        &self.target
    }

    pub fn target(&mut self, target: Triple) {
        self.target = target;
    }

    pub fn with_target(mut self, target: Triple) -> Self {
        self.target(target);
        self
    }

    pub fn opt_level(&mut self, opt_level: OptLevel) {
        self.opt_level = opt_level;
    }

    pub fn with_opt_level(mut self, opt_level: OptLevel) -> Self {
        self.opt_level(opt_level);
        self
    }

    pub fn cpu_features(&mut self, cpu_features: CpuFeatures) {
        self.cpu_features = cpu_features;
    }

    pub fn with_cpu_features(mut self, cpu_features: CpuFeatures) -> Self {
        self.cpu_features(cpu_features);
        self
    }

    pub fn cpu_features_mut(&mut self) -> &mut CpuFeatures {
        &mut self.cpu_features
    }

    pub fn heap_settings(&mut self, heap_settings: HeapSettings) {
        self.heap_settings = heap_settings;
    }

    pub fn with_heap_settings(mut self, heap_settings: HeapSettings) -> Self {
        self.heap_settings(heap_settings);
        self
    }

    pub fn heap_settings_mut(&mut self) -> &mut HeapSettings {
        &mut self.heap_settings
    }

    pub fn count_instructions(&mut self, count_instructions: bool) {
        self.count_instructions = count_instructions;
    }

    pub fn with_count_instructions(mut self, count_instructions: bool) -> Self {
        self.count_instructions(count_instructions);
        self
    }

    pub fn canonicalize_nans(&mut self, canonicalize_nans: bool) {
        self.canonicalize_nans = canonicalize_nans;
    }

    pub fn with_canonicalize_nans(mut self, canonicalize_nans: bool) -> Self {
        self.canonicalize_nans(canonicalize_nans);
        self
    }

    pub fn validator(&mut self, validator: Option<Validator>) {
        self.validator = validator;
    }

    pub fn with_validator(mut self, validator: Option<Validator>) -> Self {
        self.validator(validator);
        self
    }

    pub fn create<'a>(
        &'a self,
        wasm_binary: &'a [u8],
        bindings: &'a Bindings,
    ) -> Result<Compiler<'a>, Error> {
        Compiler::new(
            wasm_binary,
            self.target.clone(),
            self.opt_level,
            self.cpu_features.clone(),
            bindings,
            self.heap_settings.clone(),
            self.count_instructions,
            self.validator.clone(),
            self.canonicalize_nans,
        )
    }
}

pub struct Compiler<'a> {
    decls: ModuleDecls<'a>,
    codegen_context: CodegenContext,
    target: Triple,
    opt_level: OptLevel,
    cpu_features: CpuFeatures,
    count_instructions: bool,
    canonicalize_nans: bool,
    function_bodies:
        HashMap<UniqueFuncIndex, (FuncValidator<ValidatorResources>, FunctionBody<'a>)>,
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
        validator: Option<Validator>,
        canonicalize_nans: bool,
    ) -> Result<Self, Error> {
        let isa = Self::target_isa(target.clone(), opt_level, &cpu_features, canonicalize_nans)?;

        let frontend_config = isa.frontend_config();
        let mut module_validation = ModuleValidation::new(frontend_config.clone(), validator);

        let _module_translation_state = translate_module(wasm_binary, &mut module_validation)?;

        module_validation.validation_errors()?;

        let codegen_context = CodegenContext::new(isa)?;

        let runtime = Runtime::lucet(frontend_config);
        let decls = ModuleDecls::new(
            module_validation.info,
            codegen_context.clone(),
            bindings,
            runtime,
            heap_settings,
        )?;

        Ok(Self {
            decls,
            codegen_context,
            opt_level,
            cpu_features,
            count_instructions,
            target,
            canonicalize_nans,
            function_bodies: module_validation.function_bodies,
        })
    }

    pub fn builder() -> CompilerBuilder {
        CompilerBuilder::new()
    }

    pub fn module_features(&self) -> ModuleFeatures {
        let mut mf: ModuleFeatures = (&self.cpu_features).into();
        mf.instruction_count = self.count_instructions;
        mf
    }

    pub fn module_data(&self) -> Result<ModuleData<'_>, Error> {
        self.decls.get_module_data(self.module_features())
    }

    pub fn object_file(self) -> Result<ObjectFile, Error> {
        let mut func_translator = FuncTranslator::new();
        let mut function_manifest_ctx = ClifDataContext::new();
        let mut function_manifest_bytes = Cursor::new(Vec::new());

        let module_data_bytes = self.module_data()?.serialize()?;
        let module_data_len = module_data_bytes.len();

        let mut decls = self.decls;
        let codegen_context = self.codegen_context.clone();
        let count_instructions = self.count_instructions;

        let mut function_map = self
            .function_bodies
            .into_iter()
            .map(|(unique_func_ix, (mut validator, func_body))| {
                let func = decls
                    .get_func(unique_func_ix)
                    .expect("decl exists for func body");
                let mut func_info = FuncInfo::new(&decls, &codegen_context, count_instructions);
                let mut clif_context = ClifContext::new();
                clif_context.func.name = func.name.as_externalname();
                clif_context.func.signature = func.signature.clone();

                func_translator
                    .translate_body(
                        &mut validator,
                        func_body.clone(),
                        &mut clif_context.func,
                        &mut func_info,
                    )
                    .map_err(|source| Error::FunctionTranslation {
                        symbol: func.name.symbol().to_string(),
                        source: Box::new(Error::from(source)),
                    })?;
                let func_id = func.name.as_funcid().unwrap();
                let mut traps = TrapSites::new();
                let compiled = codegen_context
                    .module()
                    .define_function(func_id, &mut clif_context, &mut traps)
                    .map_err(|source| Error::FunctionDefinition {
                        symbol: func.name.symbol().to_string(),
                        source,
                    })?;

                let func_size = compiled.size;

                let trap_data_id = traps.write(&codegen_context, func.name.symbol())?;

                Ok((
                    func_id,
                    TrapMetadata {
                        func_size,
                        trap_data_id,
                        trap_len: traps.len(),
                    },
                ))
            })
            .collect::<Result<HashMap<FuncId, TrapMetadata>, Error>>()?;

        // Now that we've defined all functions, we know what trampolines must also be created.
        let trampoline_metas = codegen_context
            .trampolines()
            .iter()
            .map(|(hostcall_name, (trampoline_id, hostcall_func_index))| {
                let meta = synthesize_trampoline(
                    &decls,
                    &codegen_context,
                    hostcall_name,
                    *trampoline_id,
                    *hostcall_func_index,
                )?;
                Ok((*trampoline_id, meta))
            })
            .collect::<Result<Vec<(FuncId, TrapMetadata)>, Error>>()?;

        for (id, m) in trampoline_metas.into_iter() {
            function_map.insert(id, m);
        }

        // Write out the stack probe and associated data.
        let probe_id = stack_probe::declare(&mut decls, &codegen_context)?;
        let probe_func = decls.get_func(probe_id).unwrap();
        let probe_func_id = probe_func.name.as_funcid().unwrap();
        let compiled = codegen_context
            .module()
            .define_function_bytes(probe_func_id, stack_probe::STACK_PROBE_BINARY)?;

        let func_size = compiled.size;
        let stack_probe_traps: TrapSites = stack_probe::trap_sites().into();

        let trap_data_id = stack_probe_traps.write(&codegen_context, probe_func.name.symbol())?;

        function_map.insert(
            probe_func_id,
            TrapMetadata {
                func_size,
                trap_data_id,
                trap_len: stack_probe_traps.len(),
            },
        );

        let module_data_id = write_module_data(codegen_context.clone(), module_data_bytes)?;
        let (table_id, table_len) = write_table_data(codegen_context.clone(), &decls)?;

        // The function manifest must be written out in the order that
        // cranelift-module is going to lay out the functions.  We also
        // have to be careful to write function manifest entries for VM
        // functions, which will not be represented in function_map.

        let ids: Vec<FuncId> = codegen_context
            .module()
            .declarations()
            .get_functions()
            .map(|(func_id, _f)| func_id)
            .collect();
        let function_manifest_len = ids.len();

        for func_id in ids {
            write_function_spec(
                codegen_context.clone(),
                &mut function_manifest_ctx,
                &mut function_manifest_bytes,
                func_id,
                function_map.get(&func_id),
            )?;
        }

        function_manifest_ctx.define(function_manifest_bytes.into_inner().into());
        let manifest_data_id = codegen_context.module().declare_data(
            FUNCTION_MANIFEST_SYM,
            ClifLinkage::Local,
            false,
            false,
        )?;
        codegen_context
            .module()
            .define_data(manifest_data_id, &function_manifest_ctx)?;

        // Write out the structure tying everything together.
        let mut native_data =
            Cursor::new(Vec::with_capacity(std::mem::size_of::<SerializedModule>()));
        let mut native_data_ctx = ClifDataContext::new();
        let native_data_id = codegen_context.module().declare_data(
            LUCET_MODULE_SYM,
            ClifLinkage::Export,
            false,
            false,
        )?;

        let version =
            VersionInfo::current(include_str!(concat!(env!("OUT_DIR"), "/commit_hash")).as_bytes());

        version.write_to(&mut native_data)?;

        fn write_slice(
            codegen_context: &CodegenContext,
            mut ctx: &mut ClifDataContext,
            bytes: &mut Cursor<Vec<u8>>,
            id: DataId,
            len: usize,
        ) -> Result<(), Error> {
            let data_ref = codegen_context.module().declare_data_in_data(id, &mut ctx);
            let offset = bytes.position() as u32;
            ctx.write_data_addr(offset, data_ref, 0);
            bytes.write_u64::<LittleEndian>(0 as u64)?;
            bytes.write_u64::<LittleEndian>(len as u64)?;
            Ok(())
        }

        write_slice(
            &codegen_context,
            &mut native_data_ctx,
            &mut native_data,
            module_data_id,
            module_data_len,
        )?;
        write_slice(
            &codegen_context,
            &mut native_data_ctx,
            &mut native_data,
            table_id,
            table_len,
        )?;
        write_slice(
            &codegen_context,
            &mut native_data_ctx,
            &mut native_data,
            manifest_data_id,
            function_manifest_len,
        )?;

        native_data_ctx.define(native_data.into_inner().into());
        codegen_context
            .module()
            .define_data(native_data_id, &native_data_ctx)?;

        let obj = ObjectFile::new(codegen_context.finish())?;

        Ok(obj)
    }

    pub fn cranelift_funcs(self) -> Result<CraneliftFuncs, Error> {
        let mut funcs = HashMap::new();
        let mut func_translator = FuncTranslator::new();

        for (unique_func_ix, (mut validator, body)) in self.function_bodies.into_iter() {
            let func = self
                .decls
                .get_func(unique_func_ix)
                .expect("decl exists for func body");
            let mut func_info =
                FuncInfo::new(&self.decls, &self.codegen_context, self.count_instructions);
            let mut clif_context = ClifContext::new();
            clif_context.func.name = func.name.as_externalname();
            clif_context.func.signature = func.signature.clone();

            func_translator
                .translate_body(
                    &mut validator,
                    body.clone(),
                    &mut clif_context.func,
                    &mut func_info,
                )
                .map_err(|source| Error::FunctionTranslation {
                    symbol: func.name.symbol().to_string(),
                    source: Box::new(Error::from(source)),
                })?;

            funcs.insert(func.name.clone(), clif_context.func);
        }
        Ok(CraneliftFuncs::new(
            funcs,
            Self::target_isa(
                self.target,
                self.opt_level,
                &self.cpu_features,
                self.canonicalize_nans,
            )?,
        ))
    }

    fn target_isa(
        target: Triple,
        opt_level: OptLevel,
        cpu_features: &CpuFeatures,
        canonicalize_nans: bool,
    ) -> Result<Box<dyn TargetIsa>, Error> {
        let mut flags_builder = settings::builder();
        let isa_builder = cpu_features.isa_builder(target)?;
        flags_builder.enable("enable_verifier").unwrap();
        flags_builder.enable("is_pic").unwrap();
        flags_builder.set("opt_level", opt_level.to_flag()).unwrap();
        if canonicalize_nans {
            flags_builder.enable("enable_nan_canonicalization").unwrap();
        }
        Ok(isa_builder.finish(settings::Flags::new(flags_builder)))
    }
}

struct TrapMetadata {
    func_size: u32,
    trap_data_id: DataId,
    trap_len: usize,
}

#[derive(Clone)]
pub struct CodegenContext {
    // the `FuncId` references the declared trampoline function Cranelift knows, but the
    // `UniqueFuncIndex` references the hostcall being trampoline'd to.
    trampolines: Arc<Mutex<HashMap<String, (FuncId, UniqueFuncIndex)>>>,
    clif_module: Arc<Mutex<ObjectModule>>,
}

impl CodegenContext {
    pub fn new(isa: Box<dyn TargetIsa>) -> Result<CodegenContext, Error> {
        let libcalls = Box::new(move |libcall| match libcall {
            ir::LibCall::Probestack => stack_probe::STACK_PROBE_SYM.to_owned(),
            _ => (cranelift_module::default_libcall_names())(libcall),
        });
        let mut builder = ObjectBuilder::new(isa, "lucet_guest".to_owned(), libcalls)?;
        builder.function_alignment(16);
        let clif_module = ObjectModule::new(builder);
        Ok(CodegenContext {
            trampolines: Arc::new(Mutex::new(HashMap::new())),
            clif_module: Arc::new(Mutex::new(clif_module)),
        })
    }

    pub fn module(&self) -> MutexGuard<'_, ObjectModule> {
        self.clif_module
            .lock()
            .expect("possible to lock clifmodule")
    }

    pub fn trampolines(&self) -> MutexGuard<'_, HashMap<String, (FuncId, UniqueFuncIndex)>> {
        self.trampolines
            .lock()
            .expect("possible to lock trampolines")
    }

    pub fn finish(self) -> ObjectProduct {
        match Arc::try_unwrap(self.clif_module) {
            Ok(m) => match m.into_inner() {
                Ok(module) => module.finish(),
                _ => panic!("module lock somehow held"),
            },
            Err(e) => panic!("{} outstanding references to module", Arc::strong_count(&e)),
        }
    }
}

// Hostcall trampolines have the general shape of:
//
// ```
// fn trampoline_$hostcall(&vmctx, $hostcall_args) -> $hostcall_result {
//     if context.rsp < vmctx.instance_implicits.stack_limit {
//         // insufficient stack space to make the call
//         terminate_with_stack_overflow();
//     }
//
//     $hostcall(vmctx, $hostcall_args..)
// }
// ```
//
// but are specified here as Cranelift IR for lack of source to generate them from.
fn synthesize_trampoline(
    decls: &ModuleDecls,
    codegen_context: &CodegenContext,
    hostcall_name: &str,
    trampoline_id: FuncId,
    hostcall_func_index: UniqueFuncIndex,
) -> Result<TrapMetadata, Error> {
    let mut trampoline_context = ClifContext::new();
    trampoline_context.func.name = ir::ExternalName::from(trampoline_id);
    // the trampoline's signature is the same as the hostcall it calls' signature
    let hostcall_sig = decls
        .info
        .signature_for_function(hostcall_func_index)
        .0
        .clone();
    trampoline_context.func.signature = hostcall_sig;

    // We're going to load the stack limit later, create the global value to load while we
    // can.
    let vmctx = trampoline_context
        .func
        .create_global_value(ir::GlobalValueData::VMContext);
    let stack_limit_gv = trampoline_context
        .func
        .create_global_value(ir::GlobalValueData::Load {
            base: vmctx,
            offset: (-(std::mem::size_of::<InstanceRuntimeData>() as i32)
                + (offset_of!(InstanceRuntimeData, stack_limit) as i32))
                .into(),
            global_type: ir::types::I64,
            readonly: true,
        });

    let mut builder_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut trampoline_context.func, &mut builder_ctx);

    let entry = builder.create_block();
    let hostcall_block = builder.create_block();
    builder.append_block_params_for_function_params(entry);
    // The hostcall block will end up having all the same arguments as the trampoline,
    // which itself matches the signature of the hostcall to be called.
    builder.append_block_params_for_function_params(hostcall_block);
    let trampoline_args = builder.block_params(entry).to_vec();

    let hostcall_decl = decls
        .get_func(hostcall_func_index)
        .expect("hostcall has been declared");
    let hostcall_sig_ref = builder.import_signature(hostcall_decl.signature.clone());
    let hostcall_ref = builder.import_function(ir::ExtFuncData {
        name: hostcall_decl.name.into(),
        signature: hostcall_sig_ref,
        colocated: false,
    });

    // Reserve a block for handling a stack check fail.
    let stack_check_fail = builder.create_block();

    // And start building the trampoline from entry.
    builder.switch_to_block(entry);

    let stack_limit = builder.ins().global_value(ir::types::I64, stack_limit_gv);
    let sp_cmp = builder.ins().ifcmp_sp(stack_limit);

    builder.ins().brif(
        ir::condcodes::IntCC::UnsignedGreaterThanOrEqual,
        sp_cmp,
        stack_check_fail,
        &[],
    );
    builder.ins().fallthrough(hostcall_block, &trampoline_args);

    builder.switch_to_block(hostcall_block);
    let hostcall_args = builder.block_params(hostcall_block).to_vec();
    let call_inst = builder.ins().call(hostcall_ref, &hostcall_args);
    let results = builder.inst_results(call_inst).to_vec();
    builder.ins().return_(&results);

    builder.switch_to_block(stack_check_fail);
    builder.ins().trap(ir::TrapCode::StackOverflow);

    let mut traps = TrapSites::new();

    let trampoline_name = format!("trampoline_{}", hostcall_name);

    let compiled = codegen_context
        .module()
        .define_function(trampoline_id, &mut trampoline_context, &mut traps)
        .map_err(|source| Error::FunctionDefinition {
            symbol: trampoline_name.clone(),
            source,
        })?;

    let func_size = compiled.size;

    let trap_data_id = traps.write(codegen_context, &trampoline_name)?;

    Ok(TrapMetadata {
        func_size,
        trap_data_id,
        trap_len: traps.len(),
    })
}

fn write_module_data(
    codegen_context: CodegenContext,
    module_data_bytes: Vec<u8>,
) -> Result<DataId, Error> {
    use cranelift_module::{DataContext, Linkage};

    let mut module_data_ctx = DataContext::new();
    module_data_ctx.define(module_data_bytes.into_boxed_slice());

    let module_data_decl = codegen_context
        .module()
        .declare_data(MODULE_DATA_SYM, Linkage::Local, true, false)
        .map_err(Error::ClifModuleError)?;
    codegen_context
        .module()
        .define_data(module_data_decl, &module_data_ctx)
        .map_err(Error::ClifModuleError)?;

    Ok(module_data_decl)
}

/// Collect traps from cranelift_module codegen:
struct TrapSites {
    traps: Vec<cranelift_module::TrapSite>,
}

/// Convert from representation in stack_probe:
impl From<Vec<cranelift_module::TrapSite>> for TrapSites {
    fn from(traps: Vec<cranelift_module::TrapSite>) -> TrapSites {
        TrapSites { traps }
    }
}

impl TrapSites {
    /// Empty
    fn new() -> Self {
        Self { traps: Vec::new() }
    }
    /// Serialize for lucet_module:
    fn serialize(&self) -> Box<[u8]> {
        let traps: Vec<lucet_module::TrapSite> = self
            .traps
            .iter()
            .map(|site| lucet_module::TrapSite {
                offset: site.offset,
                code: translate_trapcode(site.code),
            })
            .collect();

        let trap_site_bytes = unsafe {
            std::slice::from_raw_parts(
                traps.as_ptr() as *const u8,
                traps.len() * std::mem::size_of::<lucet_module::TrapSite>(),
            )
        };

        trap_site_bytes.to_vec().into()
    }
    /// Write traps for a given function into the cranelift module:
    pub fn write(
        &self,
        codegen_context: &CodegenContext,
        func_name: &str,
    ) -> Result<DataId, Error> {
        let trap_sym = trap_sym_for_func(func_name);
        let mut trap_sym_ctx = ClifDataContext::new();
        trap_sym_ctx.define(self.serialize());

        let trap_data_id =
            codegen_context
                .module()
                .declare_data(&trap_sym, ClifLinkage::Local, false, false)?;

        codegen_context
            .module()
            .define_data(trap_data_id, &trap_sym_ctx)?;

        Ok(trap_data_id)
    }
    pub fn len(&self) -> usize {
        self.traps.len()
    }
}

impl cranelift_codegen::binemit::TrapSink for TrapSites {
    fn trap(
        &mut self,
        offset: cranelift_codegen::binemit::CodeOffset,
        srcloc: cranelift_codegen::ir::SourceLoc,
        code: cranelift_codegen::ir::TrapCode,
    ) {
        self.traps.push(cranelift_module::TrapSite {
            offset,
            srcloc,
            code,
        })
    }
}

fn write_function_spec(
    codegen_context: CodegenContext,
    mut manifest_ctx: &mut ClifDataContext,
    manifest_bytes: &mut Cursor<Vec<u8>>,
    func_id: FuncId,
    metadata: Option<&TrapMetadata>,
) -> Result<(), Error> {
    let size = metadata.as_ref().map(|m| m.func_size).unwrap_or(0);
    // This code has implicit knowledge of the layout of `FunctionSpec`!
    //
    // Write a (ptr, len) pair with relocation for the code.
    let func_ref = codegen_context
        .module()
        .declare_func_in_data(func_id, &mut manifest_ctx);
    let offset = manifest_bytes.position() as u32;
    manifest_ctx.write_function_addr(offset, func_ref);
    manifest_bytes.write_u64::<LittleEndian>(0 as u64)?;
    manifest_bytes.write_u64::<LittleEndian>(size as u64)?;
    // Write a (ptr, len) pair with relocation for the trap table.
    if let Some(m) = metadata {
        if m.trap_len > 0 {
            let data_ref = codegen_context
                .module()
                .declare_data_in_data(m.trap_data_id, &mut manifest_ctx);
            let offset = manifest_bytes.position() as u32;
            manifest_ctx.write_data_addr(offset, data_ref, 0);
        }
    }
    // ptr data
    manifest_bytes.write_u64::<LittleEndian>(0 as u64)?;
    // len data
    let trap_len = metadata.as_ref().map(|m| m.trap_len).unwrap_or(0);
    manifest_bytes.write_u64::<LittleEndian>(trap_len as u64)?;

    Ok(())
}
