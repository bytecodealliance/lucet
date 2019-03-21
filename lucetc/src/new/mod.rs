mod decls;
mod function;
mod module;

use crate::bindings::Bindings;
use crate::compiler::{stack_probe, ObjectFile, OptLevel};
use crate::error::{LucetcError, LucetcErrorKind};
use cranelift_codegen::{
    ir,
    isa::TargetIsa,
    settings::{self, Configurable},
    Context as ClifContext,
};
use cranelift_faerie::{FaerieBackend, FaerieBuilder, FaerieTrapCollection};
use cranelift_module::Module as ClifModule;
use cranelift_native;
use cranelift_wasm::{translate_module, FuncTranslator};
use decls::ModuleDecls;
use failure::ResultExt;
use function::FuncInfo;
use module::ModuleInfo;

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
) -> Result<ObjectFile, LucetcError> {
    let isa = target_isa(opt_level);

    let mut module_info = ModuleInfo::new(isa.frontend_config());
    translate_module(wasm_binary, &mut module_info).context(LucetcErrorKind::TranslatingModule)?;

    let libcalls = Box::new(move |libcall| match libcall {
        ir::LibCall::Probestack => stack_probe::STACK_PROBE_SYM.to_owned(),
        _ => (FaerieBuilder::default_libcall_names())(libcall),
    });

    let mut func_translator = FuncTranslator::new();
    let mut clif_context = ClifContext::new();
    let mut clif_module: ClifModule<FaerieBackend> = ClifModule::new(
        FaerieBuilder::new(
            isa,
            "lucet_guest".to_owned(),
            FaerieTrapCollection::Enabled,
            libcalls,
        )
        .context(LucetcErrorKind::Other("FIXME".to_owned()))?,
    );

    let decls = ModuleDecls::declare(module_info, &mut clif_module, bindings)?;

    for (ref func, (code, code_offset)) in decls.function_bodies() {
        let mut func_info = FuncInfo::new(&decls);
        func_translator
            .translate(code, *code_offset, &mut clif_context.func, &mut func_info)
            .context(LucetcErrorKind::Function("FIXME".to_owned()))?;

        clif_module
            .define_function(func.name.into_funcid().unwrap(), &mut clif_context)
            .context(LucetcErrorKind::Function("FIXME".to_owned()))?;
    }

    let obj = ObjectFile::new(clif_module.finish())
        .context(LucetcErrorKind::Other("FIXME".to_owned()))?;
    Ok(obj)
}
