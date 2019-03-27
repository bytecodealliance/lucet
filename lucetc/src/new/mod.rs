mod decls;
mod function;
mod module;
mod runtime;
mod sparsedata;
mod table;

use crate::bindings::Bindings;
use crate::compiler::{stack_probe, ObjectFile, OptLevel};
use crate::error::{LucetcError, LucetcErrorKind};
use crate::program::memory::HeapSettings;
use cranelift_codegen::{
    ir,
    isa::TargetIsa,
    settings::{self, Configurable},
    Context as ClifContext,
};
use cranelift_faerie::{FaerieBackend, FaerieBuilder, FaerieTrapCollection};
use cranelift_module::{Backend as ClifBackend, Module as ClifModule};
use cranelift_native;
use cranelift_wasm::{translate_module, FuncTranslator};
use decls::ModuleDecls;
use failure::ResultExt;
use function::FuncInfo;
use module::ModuleInfo;
use runtime::Runtime;
use table::write_table_data;

fn target_isa(opt_level: OptLevel) -> Box<dyn TargetIsa> {
    let mut flags_builder = settings::builder();
    let isa_builder = cranelift_native::builder().expect("host machine is not a supported target");
    flags_builder.enable("enable_verifier").unwrap();
    flags_builder.enable("is_pic").unwrap();
    flags_builder.set("opt_level", opt_level.to_flag()).unwrap();
    isa_builder.finish(settings::Flags::new(flags_builder))
}

pub fn compile<'a>(
    wasm_binary: &'a [u8],
    opt_level: OptLevel,
    bindings: &Bindings,
    heap_settings: HeapSettings,
) -> Result<ObjectFile, LucetcError> {
    let isa = target_isa(opt_level);
    let frontend_config = isa.frontend_config();
    let mut module_info = ModuleInfo::new(frontend_config.clone());
    translate_module(wasm_binary, &mut module_info).context(LucetcErrorKind::TranslatingModule)?;

    let libcalls = Box::new(move |libcall| match libcall {
        ir::LibCall::Probestack => stack_probe::STACK_PROBE_SYM.to_owned(),
        _ => (FaerieBuilder::default_libcall_names())(libcall),
    });

    let mut func_translator = FuncTranslator::new();
    let mut clif_module: ClifModule<FaerieBackend> = ClifModule::new(
        FaerieBuilder::new(
            isa,
            "lucet_guest".to_owned(),
            FaerieTrapCollection::Enabled,
            libcalls,
        )
        .context(LucetcErrorKind::Other("FIXME".to_owned()))?,
    );

    let runtime = Runtime::lucet(frontend_config);
    let decls = ModuleDecls::new(
        module_info,
        &mut clif_module,
        bindings,
        runtime,
        heap_settings,
    )?;

    for (ref func, (code, code_offset)) in decls.function_bodies() {
        let mut func_info = FuncInfo::new(&decls);
        let mut clif_context = ClifContext::new();
        clif_context.func.name = func.name.as_externalname();
        clif_context.func.signature = func.signature.clone();

        func_translator
            .translate(code, *code_offset, &mut clif_context.func, &mut func_info)
            .context(LucetcErrorKind::Function(func.name.symbol().to_owned()))?;

        clif_module
            .define_function(func.name.as_funcid().unwrap(), &mut clif_context)
            .context(LucetcErrorKind::Function(func.name.symbol().to_owned()))?;
    }

    write_module_data(&mut clif_module, &decls)?;
    write_startfunc_data(&mut clif_module, &decls)?;
    write_table_data(&mut clif_module, &decls)?;

    let obj = ObjectFile::new(clif_module.finish())
        .context(LucetcErrorKind::Other("FIXME".to_owned()))?;
    Ok(obj)
}

fn write_module_data<B: ClifBackend>(
    clif_module: &mut ClifModule<B>,
    decls: &ModuleDecls,
) -> Result<(), LucetcError> {
    use crate::new::sparsedata::OwnedSparseData;
    use byteorder::{LittleEndian, WriteBytesExt};
    use cranelift_codegen::entity::EntityRef;
    use cranelift_module::{DataContext, Linkage};
    use cranelift_wasm::MemoryIndex;
    use lucet_module_data::ModuleData;
    let memix = MemoryIndex::new(0);

    let module_data_serialized: Vec<u8> = {
        let heap_spec = decls.get_heap(memix).context(LucetcErrorKind::ModuleData)?;

        let compiled_data = OwnedSparseData::new(
            decls
                .get_data_initializers(memix)
                .context(LucetcErrorKind::ModuleData)?,
            heap_spec.clone(),
        )?;
        let sparse_data = compiled_data.sparse_data();

        let globals_spec = decls.get_globals_spec()?;

        let module_data = ModuleData::new(heap_spec.clone(), sparse_data, globals_spec);
        module_data
            .serialize()
            .context(LucetcErrorKind::ModuleData)?
    };
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
    use failure::format_err;

    let error_kind = LucetcErrorKind::Other("start_func".to_owned());

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
