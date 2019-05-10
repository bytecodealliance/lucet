use crate::traps::trap_sym_for_func;
use byteorder::{LittleEndian, WriteBytesExt};
use faerie::{Artifact, Decl, Link};
use failure::{Error, ResultExt};
use lucet_module_data::FunctionSpec;
use std::io::Cursor;
use std::mem::size_of;

fn write_relocated_slice(
    obj: &mut Artifact,
    buf: &mut Cursor<Vec<u8>>,
    from: &str,
    to: &str,
    len: u64,
) -> Result<(), Error> {
    if len > 0 {
        obj.link(Link {
            from, // the data at `from` + `at` (eg. manifest_sym)
            to,   // is a reference to `to`    (eg. fn_name)
            at: buf.position(),
        })
        .context(format!("linking {} into function manifest", to))?;
    }

    buf.write_u64::<LittleEndian>(0).unwrap();
    buf.write_u64::<LittleEndian>(len).unwrap();

    Ok(())
}

///
/// Writes a manifest of functions, with relocations, to the artifact.
///
pub fn write_function_manifest(
    functions: &[(String, FunctionSpec)],
    obj: &mut Artifact,
) -> Result<(), Error> {
    let manifest_len_sym = "lucet_function_manifest_len";
    obj.declare(&manifest_len_sym, Decl::data().global())
        .context(format!("declaring {}", &manifest_len_sym))?;

    let manifest_sym = "lucet_function_manifest";
    obj.declare(&manifest_sym, Decl::data().global())
        .context(format!("declaring {}", &manifest_sym))?;

    let mut manifest_len_buf: Vec<u8> = Vec::new();
    manifest_len_buf
        .write_u32::<LittleEndian>(functions.len() as u32)
        .unwrap();
    obj.define(manifest_len_sym, manifest_len_buf)
        .context(format!("defining {}", &manifest_len_sym))?;

    let mut manifest_buf: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(
        functions.len() * size_of::<FunctionSpec>(),
    ));

    for (fn_name, fn_spec) in functions.iter() {
        /*
         * This has implicit knowledge of the layout of `FunctionSpec`!
         *
         * Each iteraction writes out bytes with relocations that will
         * result in data forming a valid FunctionSpec when this is loaded.
         *
         * Because the addresses don't exist until relocations are applied
         * when the artifact is loaded, we can't just populate the fields
         * and transmute, unfortunately.
         */
        // Writes a (ptr, len) pair with relocation for code
        write_relocated_slice(
            obj,
            &mut manifest_buf,
            &manifest_sym,
            fn_name,
            fn_spec.code_len() as u64,
        )?;
        // Writes a (ptr, len) pair with relocation for this function's trap table
        write_relocated_slice(
            obj,
            &mut manifest_buf,
            &manifest_sym,
            &trap_sym_for_func(fn_name),
            fn_spec.traps_len() as u64,
        )?;
    }

    obj.define(&manifest_sym, manifest_buf.into_inner())
        .context(format!("defining {}", &manifest_sym))?;

    Ok(())
}
