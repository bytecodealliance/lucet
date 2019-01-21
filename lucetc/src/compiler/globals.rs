use crate::compiler::Compiler;
use crate::program::globals::Global;
use byteorder::{LittleEndian, WriteBytesExt};
use cranelift_module::{DataContext, Linkage};
use failure::Error;
use std::io::Cursor;

pub fn compile_global_specs(compiler: &mut Compiler) -> Result<(), Error> {
    let globals = compiler.prog.globals();
    let len = globals.len();

    let mut spec_contents: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(8 + 24 * len));
    spec_contents.write_u64::<LittleEndian>(len as u64).unwrap();

    let mut spec_ctx = DataContext::new();

    for ref g in globals {
        spec_contents.write_u64::<LittleEndian>(flags(g)).unwrap();
        spec_contents.write_u64::<LittleEndian>(initval(g)).unwrap();
        if let Some(name) = name(&g) {
            let sym_name = &format!("lucet_globals_name_{}", name);
            // Declare data with the name
            let name_decl = compiler
                .module
                .declare_data(sym_name, Linkage::Local, false)?;
            // Put a relocation to the name into the spec
            let sym_gv = compiler
                .module
                .declare_data_in_data(name_decl, &mut spec_ctx);
            let position = spec_contents.position();
            assert!(position < <u32>::max_value() as u64);
            spec_ctx.write_data_addr(position as u32, sym_gv, 0);

            // Define the name in the module
            let mut name_ctx = DataContext::new();
            name_ctx.define(sym_name.clone().into_bytes().into_boxed_slice());
            compiler.module.define_data(name_decl, &name_ctx)?;
        }
        spec_contents
            .write_u64::<LittleEndian>(0) // Reloc goes here
            .unwrap();
    }
    spec_ctx.define(spec_contents.into_inner().into_boxed_slice());
    let spec_decl = compiler
        .module
        .declare_data("lucet_globals_spec", Linkage::Export, false)?;
    compiler.module.define_data(spec_decl, &spec_ctx)?;

    Ok(())
}

fn flags(g: &Global) -> u64 {
    let mut flags = 0;
    match g {
        &Global::Import(_) => {
            flags |= 1;
        }
        _ => {}
    }
    match name(g) {
        Some(_) => {
            flags |= 2;
        }
        _ => {}
    }
    flags
}

fn initval(g: &Global) -> u64 {
    match g {
        &Global::Def(ref def) => def.value() as u64,
        _ => 0,
    }
}

fn name(g: &Global) -> Option<String> {
    match g {
        &Global::Def(ref def) => def.export(),
        &Global::Import(ref import) => Some(format!("{}::{}", import.module(), import.field())),
    }
}
